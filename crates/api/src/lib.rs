pub mod model;
pub mod store;

use axum::{
    Router,
    http::{HeaderValue, Method},
};
use rusqlite::Connection;
use skarma_core::Command;
use skarma_http::Repository;
use std::sync::{Arc, Mutex};
use store::{ApiError, Result};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

#[derive(Clone)]
pub struct Database(pub Arc<Mutex<Connection>>);

impl Repository for Database {
    async fn execute(&self, command: Command) -> Result<serde_json::Value> {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = db.lock().map_err(|_| ApiError::storage())?;
            store::execute(&mut connection, command)
        })
        .await
        .map_err(|_| ApiError::storage())?
    }

    async fn preview_import(
        &self,
        authorization: Command,
        filename: String,
        bytes: Vec<u8>,
    ) -> Result<serde_json::Value> {
        self.execute(authorization.clone()).await?;
        let input =
            tokio::task::spawn_blocking(move || skarma_http::import::parse(&filename, &bytes))
                .await
                .map_err(|_| ApiError::storage())??;
        self.execute(skarma_http::import::preview_command(authorization, input)?)
            .await
    }

    async fn export(&self, command: Command) -> Result<Vec<u8>> {
        let snapshot = self.execute(command).await?;
        // The transaction has committed. Compression never blocks the DB mutex or Tokio workers.
        tokio::task::spawn_blocking(move || {
            skarma_http::export::workbook(serde_json::from_value(snapshot)?)
        })
        .await
        .map_err(|_| ApiError::storage())?
    }
}

pub fn router(db: Database, web_origin: HeaderValue, assets: &str) -> Router {
    router_with_auth(
        db,
        web_origin,
        assets,
        skarma_http::auth::AuthConfig::default(),
    )
}

pub fn router_with_auth(
    db: Database,
    web_origin: HeaderValue,
    assets: &str,
    auth: skarma_http::auth::AuthConfig,
) -> Router {
    skarma_http::router_with_auth(db, auth)
        .fallback_service(
            ServeDir::new(assets).not_found_service(ServeFile::new(format!("{assets}/index.html"))),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(web_origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderName::from_static("x-skarma-request"),
                ])
                .allow_credentials(true),
        )
}

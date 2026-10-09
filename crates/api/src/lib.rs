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
}

pub fn router(db: Database, web_origin: HeaderValue, assets: &str) -> Router {
    skarma_http::router(db)
        .fallback_service(
            ServeDir::new(assets).not_found_service(ServeFile::new(format!("{assets}/index.html"))),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(web_origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT])
                .allow_headers([axum::http::header::CONTENT_TYPE]),
        )
}

pub mod model;
pub mod store;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderValue, Method, StatusCode},
    routing::{get, post, put},
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

use model::*;
use store::{ApiError, Result};

#[derive(Clone)]
pub struct Database(pub Arc<Mutex<Connection>>);

impl Database {
    async fn execute<T, F>(&self, command: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = db.lock().map_err(|_| {
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, "数据库暂时不可用".into())
            })?;
            command(&mut connection)
        })
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "操作暂时不可用".into()))?
    }
}

async fn read_state(State(db): State<Database>) -> Result<Json<AppState>> {
    Ok(Json(db.execute(|db| store::state(db)).await?))
}

async fn create_goal(State(db): State<Database>, Json(input): Json<NewGoal>) -> Result<Json<Goal>> {
    Ok(Json(
        db.execute(move |db| store::create_goal(db, input)).await?,
    ))
}

async fn save_session(
    State(db): State<Database>,
    Path(id): Path<String>,
    Json(input): Json<SaveSession>,
) -> Result<Json<SaveResult>> {
    Ok(Json(
        db.execute(move |db| store::save_session(db, &id, input))
            .await?,
    ))
}

async fn update_difficulty(
    State(db): State<Database>,
    Path(id): Path<String>,
    Json(input): Json<UpdateFollowUp>,
) -> Result<Json<FollowUp>> {
    Ok(Json(
        db.execute(move |db| store::update_follow_up(db, "difficulties", &id, input))
            .await?,
    ))
}

async fn update_experience(
    State(db): State<Database>,
    Path(id): Path<String>,
    Json(input): Json<UpdateFollowUp>,
) -> Result<Json<FollowUp>> {
    Ok(Json(
        db.execute(move |db| store::update_follow_up(db, "experiences", &id, input))
            .await?,
    ))
}

pub fn router(db: Database, web_origin: HeaderValue, assets: &str) -> Router {
    let api = Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status": "ok"})) }),
        )
        .route("/state", get(read_state))
        .route("/goals", post(create_goal))
        .route("/sessions/{id}", put(save_session))
        .route("/difficulties/{id}", put(update_difficulty))
        .route("/experiences/{id}", put(update_experience))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error":"接口不存在"})),
            )
        });
    Router::new()
        .nest("/api", api)
        .fallback_service(
            ServeDir::new(assets).not_found_service(ServeFile::new(format!("{assets}/index.html"))),
        )
        .layer(DefaultBodyLimit::max(128 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(web_origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT])
                .allow_headers([axum::http::header::CONTENT_TYPE]),
        )
        .with_state(db)
}

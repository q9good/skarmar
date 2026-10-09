use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde_json::Value;
use skarma_core::{Command, model::*, store::Result};
use std::future::Future;

/// Each execute call is one atomic unit of work on the selected backend.
pub trait Repository: Clone + Send + Sync + 'static {
    fn execute(&self, command: Command) -> impl Future<Output = Result<Value>> + Send;
}

async fn read_state<R: Repository>(State(repo): State<R>) -> Result<Json<Value>> {
    Ok(Json(repo.execute(Command::State).await?))
}
async fn goal_history<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    Ok(Json(repo.execute(Command::GoalHistory { id }).await?))
}
async fn create_goal<R: Repository>(
    State(repo): State<R>,
    Json(input): Json<NewGoal>,
) -> Result<Json<Value>> {
    Ok(Json(repo.execute(Command::CreateGoal(input)).await?))
}
async fn update_goal<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    Json(request): Json<UpdateGoal>,
) -> Result<Json<Value>> {
    Ok(Json(
        repo.execute(Command::UpdateGoal { id, request }).await?,
    ))
}
async fn save_session<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    Json(request): Json<SaveSession>,
) -> Result<Json<Value>> {
    Ok(Json(
        repo.execute(Command::SaveSession { id, request }).await?,
    ))
}
async fn update_difficulty<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    Json(request): Json<UpdateFollowUp>,
) -> Result<Json<Value>> {
    Ok(Json(
        repo.execute(Command::UpdateFollowUp {
            table: "difficulties".into(),
            id,
            request,
        })
        .await?,
    ))
}
async fn update_experience<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    Json(request): Json<UpdateFollowUp>,
) -> Result<Json<Value>> {
    Ok(Json(
        repo.execute(Command::UpdateFollowUp {
            table: "experiences".into(),
            id,
            request,
        })
        .await?,
    ))
}

pub fn router<R: Repository>(repo: R) -> Router {
    let api = Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .route("/state", get(read_state::<R>))
        .route("/goals", post(create_goal::<R>))
        .route("/goals/{id}", put(update_goal::<R>))
        .route("/goals/{id}/history", get(goal_history::<R>))
        .route("/sessions/{id}", put(save_session::<R>))
        .route("/difficulties/{id}", put(update_difficulty::<R>))
        .route("/experiences/{id}", put(update_experience::<R>))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error":"接口不存在"})),
            )
        });
    Router::new()
        .nest("/api", api)
        .layer(DefaultBodyLimit::max(128 * 1024))
        .with_state(repo)
}

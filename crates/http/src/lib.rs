use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde_json::Value;
use skarma_core::{Command, model::*, store::Result};
use std::future::Future;
pub mod auth;

/// Each execute call is one atomic unit of work on the selected backend.
pub trait Repository: Clone + Send + Sync + 'static {
    fn execute(&self, command: Command) -> impl Future<Output = Result<Value>> + Send;
}

async fn execute<R: Repository>(
    repo: &R,
    proof: Option<Extension<auth::SessionProof>>,
    command: Command,
) -> Result<Value> {
    let command = if let Some(Extension(auth::SessionProof(token))) = proof {
        Command::Authenticated {
            token,
            command: Box::new(command),
        }
    } else {
        command
    };
    repo.execute(command).await
}
async fn read_state<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(execute(&repo, proof, Command::State).await?))
}
async fn goal_history<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::GoalHistory { id }).await?,
    ))
}
async fn create_goal<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(input): Json<NewGoal>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::CreateGoal(input)).await?,
    ))
}
async fn update_goal<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(request): Json<UpdateGoal>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::UpdateGoal { id, request }).await?,
    ))
}
async fn save_session<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(request): Json<SaveSession>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::SaveSession { id, request }).await?,
    ))
}
async fn update_difficulty<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(request): Json<UpdateFollowUp>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(
            &repo,
            proof,
            Command::UpdateFollowUp {
                table: "difficulties".into(),
                id,
                request,
            },
        )
        .await?,
    ))
}
async fn update_experience<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(request): Json<UpdateFollowUp>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(
            &repo,
            proof,
            Command::UpdateFollowUp {
                table: "experiences".into(),
                id,
                request,
            },
        )
        .await?,
    ))
}

pub fn router<R: Repository>(repo: R) -> Router {
    router_with_auth(repo, auth::AuthConfig::default())
}

pub fn router_with_auth<R: Repository>(repo: R, config: auth::AuthConfig) -> Router {
    let auth_state = auth::AuthState {
        repo: repo.clone(),
        config,
    };
    let protected = Router::new()
        .route("/state", get(read_state::<R>))
        .route("/goals", post(create_goal::<R>))
        .route("/goals/{id}", put(update_goal::<R>))
        .route("/goals/{id}/history", get(goal_history::<R>))
        .route("/sessions/{id}", put(save_session::<R>))
        .route("/difficulties/{id}", put(update_difficulty::<R>))
        .route("/experiences/{id}", put(update_experience::<R>))
        .route_layer(axum::middleware::from_fn_with_state(
            auth_state.clone(),
            auth::require_session::<R>,
        ))
        .with_state(repo);
    let api = protected
        .merge(auth::routes(auth_state.clone()))
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error":"接口不存在"})),
            )
        });
    Router::new()
        .nest("/api", api)
        .layer(DefaultBodyLimit::max(128 * 1024))
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            auth::protect_requests::<R>,
        ))
}

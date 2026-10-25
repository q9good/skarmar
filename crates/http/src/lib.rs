use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde_json::Value;
use skarma_core::{
    Command,
    import::ImportCommitInput,
    model::*,
    store::{ApiError, Result},
};
use std::future::Future;
pub mod auth;
pub mod export;
pub mod import;
pub mod prd_export;

/// Each execute call is one atomic unit of work on the selected backend.
pub trait Repository: Clone + Send + Sync + 'static {
    fn execute(&self, command: Command) -> impl Future<Output = Result<Value>> + Send;
    /// Capture the authenticated snapshot atomically, then render without holding the DB lock.
    fn export(&self, command: Command) -> impl Future<Output = Result<Vec<u8>>> + Send;
    /// Parse uploads where the backend has a suitable CPU budget. The Worker
    /// adapter overrides this to parse inside its private Durable Object.
    fn preview_import(
        &self,
        authorization: Command,
        filename: String,
        bytes: Vec<u8>,
    ) -> impl Future<Output = Result<Value>> + Send {
        async move {
            self.execute(authorization.clone()).await?;
            let input = import::parse(&filename, &bytes)?;
            self.execute(import::preview_command(authorization, input)?)
                .await
        }
    }
}

fn authenticated(proof: Option<Extension<auth::SessionProof>>, command: Command) -> Command {
    if let Some(Extension(auth::SessionProof {
        token,
        expected_actor,
    })) = proof
    {
        Command::Authenticated {
            token,
            expected_actor,
            command: Box::new(command),
        }
    } else {
        command
    }
}
async fn execute<R: Repository>(
    repo: &R,
    proof: Option<Extension<auth::SessionProof>>,
    command: Command,
) -> Result<Value> {
    repo.execute(authenticated(proof, command)).await
}
async fn export_workbook<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Response> {
    let bytes = repo
        .export(authenticated(proof, Command::ExportSnapshot))
        .await?;
    Ok((
        [
            (header::CONTENT_TYPE, export::MIME),
            (header::CONTENT_DISPOSITION, export::DISPOSITION),
        ],
        bytes,
    )
        .into_response())
}
async fn export_prd<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    headers: HeaderMap,
) -> Result<Response> {
    if headers
        .get("x-skarma-contract")
        .and_then(|v| v.to_str().ok())
        != Some("skarmar.prd.v1")
    {
        return Err(ApiError::conflict("导出页面版本不匹配，请保留填写后刷新"));
    }
    let bytes = repo
        .export(authenticated(proof, Command::FoundationExportSnapshot))
        .await?;
    Ok((
        [
            (header::CONTENT_TYPE, export::MIME),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"skarmar-prd-export.xlsx\"",
            ),
        ],
        bytes,
    )
        .into_response())
}
async fn preview_import<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    execute(&repo, proof.clone(), Command::ImportAuthorize).await?;
    let field = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::invalid("上传内容无法读取"))?
        .ok_or_else(|| ApiError::invalid("请选择 Excel 或 CSV 文件"))?;
    if field.name() != Some("file") {
        return Err(ApiError::invalid("上传表单缺少 file 字段"));
    }
    let filename = field
        .file_name()
        .ok_or_else(|| ApiError::invalid("上传文件没有名称"))?
        .to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| ApiError::invalid("上传文件无法读取"))?;
    if multipart
        .next_field()
        .await
        .map_err(|_| ApiError::invalid("上传内容无法读取"))?
        .is_some()
    {
        return Err(ApiError::invalid("一次只能上传一个文件"));
    }
    Ok(Json(
        repo.preview_import(
            authenticated(proof, Command::ImportAuthorize),
            filename,
            bytes.to_vec(),
        )
        .await?,
    ))
}
async fn preview_prd_import<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>> {
    if headers
        .get("x-skarma-contract")
        .and_then(|h| h.to_str().ok())
        != Some("skarmar.prd.v1")
    {
        return Err(ApiError::conflict("导入页面版本不匹配，请刷新"));
    }
    let authorization =
        Command::DataTransfer(skarma_core::foundation::transfer::Command::AuthorizeUpload);
    execute(&repo, proof.clone(), authorization.clone()).await?;
    let field = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::invalid("上传内容无法读取"))?
        .ok_or_else(|| ApiError::invalid("请选择 Excel 或 CSV 文件"))?;
    if field.name() != Some("file") {
        return Err(ApiError::invalid("上传表单缺少 file 字段"));
    }
    let filename = field
        .file_name()
        .ok_or_else(|| ApiError::invalid("上传文件没有名称"))?
        .to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| ApiError::invalid("上传文件无法读取"))?;
    if multipart
        .next_field()
        .await
        .map_err(|_| ApiError::invalid("上传内容无法读取"))?
        .is_some()
    {
        return Err(ApiError::invalid("一次只能上传一个文件"));
    }
    Ok(Json(
        repo.preview_import(
            authenticated(proof, authorization),
            filename,
            bytes.to_vec(),
        )
        .await?,
    ))
}
async fn commit_import<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(input): Json<ImportCommitInput>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::ImportCommit(input)).await?,
    ))
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
async fn session_history<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::SessionHistory { id }).await?,
    ))
}
macro_rules! delete_endpoint {
    ($name:ident, $kind:literal) => {
        async fn $name<R: Repository>(
            State(repo): State<R>,
            Path(id): Path<String>,
            proof: Option<Extension<auth::SessionProof>>,
            Json(request): Json<DeleteRecord>,
        ) -> Result<Json<Value>> {
            Ok(Json(
                execute(
                    &repo,
                    proof,
                    Command::DeleteRecord {
                        kind: $kind.into(),
                        id,
                        request,
                    },
                )
                .await?,
            ))
        }
    };
}
delete_endpoint!(delete_goal, "goals");
delete_endpoint!(delete_pathway, "pathways");
delete_endpoint!(delete_experience, "experiences");
delete_endpoint!(delete_difficulty, "difficulties");
delete_endpoint!(delete_session, "sessions");
delete_endpoint!(delete_lesson, "lessons");
async fn create_goal<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(input): Json<NewGoal>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::CreateGoal(input)).await?,
    ))
}

async fn create_pathway<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(input): Json<NewPathway>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::CreatePathway(input)).await?,
    ))
}

async fn update_pathway<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(request): Json<UpdatePathway>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::UpdatePathway { id, request }).await?,
    ))
}

async fn create_experience<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(input): Json<NewExperience>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::CreateExperience(input)).await?,
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
async fn save_lesson<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(request): Json<SaveLesson>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::SaveLesson { id, request }).await?,
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

async fn add_lesson_option<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(input): Json<NewLessonOption>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::AddLessonOption(input)).await?,
    ))
}

async fn update_lesson_option<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    Json(input): Json<UpdateLessonOption>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::UpdateLessonOption(input)).await?,
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

async fn runtime_info<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(execute(&repo, proof, Command::RuntimeInfo).await?))
}
async fn foundation_state<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(execute(&repo, proof, Command::FoundationState).await?))
}
async fn foundation_history<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::FoundationHistory { id }).await?,
    ))
}
async fn foundation_write<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    headers: HeaderMap,
    input: std::result::Result<
        Json<skarma_core::foundation::model::Write>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Json<Value>> {
    if headers
        .get("x-skarma-contract")
        .and_then(|v| v.to_str().ok())
        != Some("skarmar.prd.v1")
    {
        return Err(ApiError::conflict("页面版本已更新，请保留填写后刷新页面"));
    }
    let Json(request) = input
        .map_err(|error| ApiError(error.status(), "填写信息格式不正确，请核对后重试".into()))?;
    if request.id != id {
        return Err(ApiError::invalid("记录编号与请求路径不一致"));
    }
    Ok(Json(
        execute(&repo, proof, Command::FoundationWrite(request)).await?,
    ))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct QueuedInput {
    user_id: String,
    write: skarma_core::foundation::lesson_model::QueuedWrite,
}
async fn foundation_queue<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    headers: HeaderMap,
    input: std::result::Result<Json<QueuedInput>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>> {
    if headers
        .get("x-skarma-contract")
        .and_then(|v| v.to_str().ok())
        != Some("skarmar.prd.v1")
    {
        return Err(ApiError::conflict(
            "页面版本已更新，请保留待上传记录后刷新页面",
        ));
    }
    let Json(input) =
        input.map_err(|e| ApiError(e.status(), "上传内容格式不匹配，请保留原始填写".into()))?;
    Ok(Json(
        execute(
            &repo,
            proof,
            Command::FoundationQueued {
                user_id: input.user_id,
                write: input.write,
            },
        )
        .await?,
    ))
}

async fn foundation_lesson<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    headers: HeaderMap,
    input: std::result::Result<
        Json<skarma_core::foundation::lesson_model::LessonWrite>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Json<Value>> {
    if headers
        .get("x-skarma-contract")
        .and_then(|v| v.to_str().ok())
        != Some("skarmar.prd.v1")
    {
        return Err(ApiError::conflict("页面版本已更新，请保留填写后刷新页面"));
    }
    let Json(request) =
        input.map_err(|e| ApiError(e.status(), "填写信息格式不正确，请核对后重试".into()))?;
    if id != request.id {
        return Err(ApiError::invalid("记录编号与请求路径不一致"));
    }
    Ok(Json(
        execute(&repo, proof, Command::FoundationLesson(request)).await?,
    ))
}
async fn foundation_lesson_history<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::FoundationLessonHistory { id }).await?,
    ))
}
async fn foundation_lesson_withdrawal_capability<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(
            &repo,
            proof,
            Command::FoundationLessonWithdrawalCapability { id },
        )
        .await?,
    ))
}

async fn foundation_session<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
    headers: HeaderMap,
    input: std::result::Result<
        Json<skarma_core::foundation::session_model::Write>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Json<Value>> {
    if headers
        .get("x-skarma-contract")
        .and_then(|v| v.to_str().ok())
        != Some("skarmar.prd.v1")
    {
        return Err(ApiError::conflict("页面版本已更新，请保留填写后刷新页面"));
    }
    let Json(request) =
        input.map_err(|e| ApiError(e.status(), "填写信息格式不正确，请核对".into()))?;
    if request.id != id {
        return Err(ApiError::invalid("记录编号与请求路径不一致"));
    }
    Ok(Json(
        execute(&repo, proof, Command::FoundationSession(request)).await?,
    ))
}
async fn foundation_session_history<R: Repository>(
    State(repo): State<R>,
    Path(id): Path<String>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::FoundationSessionHistory { id }).await?,
    ))
}
async fn foundation_coverage<R: Repository>(
    State(repo): State<R>,
    axum::extract::Query(query): axum::extract::Query<skarma_core::foundation::coverage::Request>,
    proof: Option<Extension<auth::SessionProof>>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(&repo, proof, Command::FoundationCoverage(query)).await?,
    ))
}

async fn foundation_process<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    headers: HeaderMap,
    input: std::result::Result<
        Json<skarma_core::foundation::process_model::Command>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Json<Value>> {
    if headers
        .get("x-skarma-contract")
        .and_then(|v| v.to_str().ok())
        != Some("skarmar.prd.v1")
    {
        return Err(ApiError::conflict("页面版本已更新，请保留填写后刷新页面"));
    }
    let Json(request) =
        input.map_err(|e| ApiError(e.status(), "填写信息格式不正确，请核对".into()))?;
    Ok(Json(
        execute(&repo, proof, Command::FoundationProcess(request)).await?,
    ))
}
async fn foundation_process_read<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    axum::extract::Query(query): axum::extract::Query<skarma_core::foundation::process_model::Read>,
) -> Result<Json<Value>> {
    Ok(Json(
        execute(
            &repo,
            proof,
            Command::FoundationProcess(skarma_core::foundation::process_model::Command::Read(
                query,
            )),
        )
        .await?,
    ))
}

async fn data_transfer<R: Repository>(
    State(repo): State<R>,
    proof: Option<Extension<auth::SessionProof>>,
    headers: HeaderMap,
    input: std::result::Result<
        Json<skarma_core::foundation::transfer::Command>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Json<Value>> {
    if headers
        .get("x-skarma-transfer")
        .and_then(|h| h.to_str().ok())
        != Some("skarmar.transfer.v1")
    {
        return Err(ApiError::conflict("资料转换页面版本不匹配"));
    }
    let Json(command) =
        input.map_err(|e| ApiError(e.status(), "转换字段格式不正确，请保留填写后核对".into()))?;
    if matches!(
        &command,
        skarma_core::foundation::transfer::Command::AuthorizeUpload
            | skarma_core::foundation::transfer::Command::Workbook(_)
    ) {
        return Err(ApiError::invalid("文件资料须通过上传解析入口"));
    }
    Ok(Json(
        execute(&repo, proof, Command::DataTransfer(command)).await?,
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
        .route("/runtime", get(runtime_info::<R>))
        .route(
            "/data-transfer",
            post(data_transfer::<R>).layer(DefaultBodyLimit::max(1_100_000)),
        )
        .route(
            "/prd/import/preview",
            post(preview_prd_import::<R>).layer(DefaultBodyLimit::max(11 * 1024 * 1024)),
        )
        .route("/prd/state", get(foundation_state::<R>))
        .route(
            "/prd/process",
            post(foundation_process::<R>).get(foundation_process_read::<R>),
        )
        .route("/prd/sessions/{id}", put(foundation_session::<R>))
        .route(
            "/prd/sessions/{id}/history",
            get(foundation_session_history::<R>),
        )
        .route("/prd/coverage", get(foundation_coverage::<R>))
        .route("/prd/export.xlsx", get(export_prd::<R>))
        .route("/prd/queue", post(foundation_queue::<R>))
        .route("/prd/lessons/{id}", put(foundation_lesson::<R>))
        .route(
            "/prd/lessons/{id}/history",
            get(foundation_lesson_history::<R>),
        )
        .route(
            "/prd/lessons/{id}/withdrawal-capability",
            get(foundation_lesson_withdrawal_capability::<R>),
        )
        .route("/prd/records/{id}", put(foundation_write::<R>))
        .route("/prd/records/{id}/history", get(foundation_history::<R>))
        .route("/state", get(read_state::<R>))
        .route("/export.xlsx", get(export_workbook::<R>))
        .route(
            "/import/preview",
            post(preview_import::<R>).layer(DefaultBodyLimit::max(11 * 1024 * 1024)),
        )
        .route("/import/commit", post(commit_import::<R>))
        .route("/goals", post(create_goal::<R>))
        .route("/pathways", post(create_pathway::<R>))
        .route(
            "/pathways/{id}",
            put(update_pathway::<R>).delete(delete_pathway::<R>),
        )
        .route("/experiences", post(create_experience::<R>))
        .route(
            "/goals/{id}",
            put(update_goal::<R>).delete(delete_goal::<R>),
        )
        .route("/goals/{id}/history", get(goal_history::<R>))
        .route(
            "/sessions/{id}",
            put(save_session::<R>).delete(delete_session::<R>),
        )
        .route("/sessions/{id}/history", get(session_history::<R>))
        .route(
            "/lessons/{id}",
            put(save_lesson::<R>).delete(delete_lesson::<R>),
        )
        .route(
            "/lesson-options",
            post(add_lesson_option::<R>).put(update_lesson_option::<R>),
        )
        .route(
            "/difficulties/{id}",
            put(update_difficulty::<R>).delete(delete_difficulty::<R>),
        )
        .route(
            "/experiences/{id}",
            put(update_experience::<R>).delete(delete_experience::<R>),
        )
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

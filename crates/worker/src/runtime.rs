use axum::{body::Body, http::StatusCode};
use skarma_core::{
    Command,
    config::{Config, DataModel, Runtime},
    sql::{Sql, Value},
    store::{ApiError, Result as AppResult},
};
use skarma_http::Repository;
use std::{cell::RefCell, rc::Rc};
use tower_service::Service;
use worker::{send::SendFuture, *};

fn config(env: &Env) -> AppResult<Config> {
    Config::load(Runtime::Cloudflare, |key| {
        env.var(key).ok().map(|value| value.to_string())
    })
    .map_err(|message| ApiError(StatusCode::INTERNAL_SERVER_ERROR, message))
}

#[derive(Clone)]
struct DurableRepository {
    env: Env,
    space: String,
}
impl DurableRepository {
    async fn request(&self, command: &Command, path: &str) -> AppResult<worker::Response> {
        let namespace = self
            .env
            .durable_object("SKARMA_DB")
            .map_err(|_| ApiError::storage())?;
        let stub = namespace
            .id_from_name(&self.space)
            .and_then(|id| id.get_stub())
            .map_err(|_| ApiError::storage())?;
        let mut init = RequestInit::new();
        init.with_method(Method::Post)
            .with_body(Some(serde_json::to_string(command)?.into()));
        let req = Request::new_with_init(&format!("https://skarma.internal{path}"), &init)
            .map_err(|_| ApiError::storage())?;
        stub.fetch_with_request(req)
            .await
            .map_err(|_| ApiError::storage())
    }
}
impl Repository for DurableRepository {
    fn execute(
        &self,
        command: Command,
    ) -> impl std::future::Future<Output = AppResult<serde_json::Value>> + Send {
        // The SDK's SendFuture bridge is valid in the single-threaded Worker runtime.
        SendFuture::new(async move {
            let mut response = self.request(&command, "/command").await?;
            let status =
                StatusCode::from_u16(response.status_code()).map_err(|_| ApiError::storage())?;
            let value: serde_json::Value =
                response.json().await.map_err(|_| ApiError::storage())?;
            if !status.is_success() {
                return Err(ApiError(
                    status,
                    value["error"].as_str().unwrap_or("数据保存失败").into(),
                ));
            }
            Ok(value)
        })
    }

    fn preview_import(
        &self,
        authorization: Command,
        filename: String,
        bytes: Vec<u8>,
    ) -> impl std::future::Future<Output = AppResult<serde_json::Value>> + Send {
        SendFuture::new(async move {
            // Keep decompression/XML parsing out of the ordinary Worker. Raw
            // bytes avoid a large JSON number array and the duplicate cell copy.
            let namespace = self
                .env
                .durable_object("SKARMA_DB")
                .map_err(|_| ApiError::storage())?;
            let stub = namespace
                .id_from_name(&self.space)
                .and_then(|id| id.get_stub())
                .map_err(|_| ApiError::storage())?;
            let mut url = Url::parse("https://skarma.internal/import/preview")
                .map_err(|_| ApiError::storage())?;
            url.query_pairs_mut().append_pair("filename", &filename);
            let headers = Headers::new();
            headers
                .set("content-type", "application/octet-stream")
                .map_err(|_| ApiError::storage())?;
            headers
                .set(
                    "x-skarma-import-command",
                    &serde_json::to_string(&authorization)?,
                )
                .map_err(|_| ApiError::storage())?;
            let mut init = RequestInit::new();
            init.with_method(Method::Post)
                .with_headers(headers)
                .with_body(Some(js_sys::Uint8Array::from(bytes.as_slice()).into()));
            let request =
                Request::new_with_init(url.as_str(), &init).map_err(|_| ApiError::storage())?;
            let mut response = stub
                .fetch_with_request(request)
                .await
                .map_err(|_| ApiError::storage())?;
            let status =
                StatusCode::from_u16(response.status_code()).map_err(|_| ApiError::storage())?;
            let value: serde_json::Value =
                response.json().await.map_err(|_| ApiError::storage())?;
            if !status.is_success() {
                return Err(ApiError(
                    status,
                    value["error"].as_str().unwrap_or("导入预览失败").into(),
                ));
            }
            Ok(value)
        })
    }

    fn export(
        &self,
        command: Command,
    ) -> impl std::future::Future<Output = AppResult<Vec<u8>>> + Send {
        SendFuture::new(async move {
            // Generate and compress in the DO, outside the ordinary gateway's short CPU budget.
            let mut response = self.request(&command, "/export.xlsx").await?;
            let status =
                StatusCode::from_u16(response.status_code()).map_err(|_| ApiError::storage())?;
            if !status.is_success() {
                let value: serde_json::Value =
                    response.json().await.map_err(|_| ApiError::storage())?;
                return Err(ApiError(
                    status,
                    value["error"].as_str().unwrap_or("Excel 导出失败").into(),
                ));
            }
            response.bytes().await.map_err(|_| ApiError::storage())
        })
    }
}

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, _ctx: Context) -> Result<axum::http::Response<Body>> {
    let cfg = match config(&env) {
        Ok(cfg) => cfg,
        Err(error) => return Ok(axum::response::IntoResponse::into_response(error)),
    };
    let auth = match skarma_http::auth::AuthConfig::load(&cfg, |key| {
        env.var(key).ok().map(|value| value.to_string())
    }) {
        Ok(auth) => auth,
        Err(message) => {
            return Ok(axum::response::IntoResponse::into_response(ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                message,
            )));
        }
    };
    let repo = DurableRepository {
        env,
        space: cfg.space,
    };
    Ok(skarma_http::router_with_auth(repo, auth).call(req).await?)
}

struct DurableSql(SqlStorage);
fn values(params: &[Value]) -> Vec<SqlStorageValue> {
    params
        .iter()
        .map(|value| match value {
            Value::Null => SqlStorageValue::Null,
            Value::Text(text) => SqlStorageValue::String(text.clone()),
            Value::Integer(n) => SqlStorageValue::Integer(*n),
        })
        .collect()
}
impl Sql for DurableSql {
    fn execute(&self, sql: &str, params: &[Value]) -> AppResult<()> {
        self.0
            .exec(sql, values(params))
            .map_err(|_| ApiError::storage())?;
        Ok(())
    }
    fn execute_batch(&self, sql: &str) -> AppResult<()> {
        self.0.exec(sql, None).map_err(|_| ApiError::storage())?;
        Ok(())
    }
    fn query(&self, sql: &str, params: &[Value]) -> AppResult<Vec<Vec<String>>> {
        // Materialize each cursor before any await: a cursor across awaits is not a stable snapshot.
        self.0
            .exec(sql, values(params))
            .map_err(|_| ApiError::storage())?
            .raw()
            .map(|row| {
                row.map_err(|_| ApiError::storage())?
                    .into_iter()
                    .map(|value| match value {
                        SqlStorageValue::String(text) => Ok(text),
                        _ => Err(ApiError::storage()),
                    })
                    .collect()
            })
            .collect()
    }
}

#[durable_object]
pub struct SkarmaSpace {
    state: State,
    env: Env,
}
impl DurableObject for SkarmaSpace {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }
    async fn fetch(&self, mut req: worker::Request) -> Result<worker::Response> {
        let export = req.path() == "/export.xlsx";
        let import = req.path() == "/import/preview";
        if req.method() != Method::Post || (!export && !import && req.path() != "/command") {
            return Response::error("Unknown internal command", 404);
        }
        let cfg = match config(&self.env) {
            Ok(cfg) => cfg,
            Err(error) => {
                return Response::from_json(&serde_json::json!({"error":error.1}))
                    .map(|response| response.with_status(error.0.as_u16()));
            }
        };
        let (command, upload) = if import {
            let Some(encoded) = req.headers().get("x-skarma-import-command")? else {
                return Response::error("Missing import authorization", 400);
            };
            let command: Command = serde_json::from_str(&encoded)?;
            // Restrict the private upload endpoint to an authorization proof;
            // arbitrary commands cannot be smuggled through upload metadata.
            if !matches!(&command, Command::Authenticated { command, .. } if matches!(**command, Command::ImportAuthorize | Command::DataTransfer(skarma_core::foundation::transfer::Command::AuthorizeUpload)))
            {
                return Response::error("Invalid import authorization", 400);
            }
            let filename = req
                .url()?
                .query_pairs()
                .find(|(name, _)| name == "filename")
                .map(|(_, value)| value.into_owned())
                .unwrap_or_default();
            let bytes = req.bytes().await?;
            if bytes.len() > skarma_http::import::MAX_FILE_BYTES {
                return Response::error("Upload is too large", 413);
            }
            (command, Some((filename, bytes)))
        } else {
            (req.json().await?, None)
        };
        if export
            && !matches!(&command, Command::ExportSnapshot)
            && !matches!(&command, Command::Authenticated { command, .. } if matches!(**command, Command::ExportSnapshot | Command::FoundationExportSnapshot))
        {
            return Response::error("Invalid export command", 400);
        }
        // Optional private bootstrap for a future authenticated Worker deployment.
        // Provision through Wrangler secrets; there is no public bootstrap endpoint.
        let initial_admin = self
            .env
            .secret("SKARMA_INITIAL_ADMIN_PASSWORD")
            .ok()
            .map(|password| skarma_core::auth::NewUser {
                username: self
                    .env
                    .var("SKARMA_INITIAL_ADMIN_USER")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|_| "owner".into()),
                display_name: self
                    .env
                    .var("SKARMA_INITIAL_ADMIN_NAME")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|_| "管理员".into()),
                password: password.to_string(),
                role: skarma_core::auth::AccountRole::Admin,
            });
        let output = Rc::new(RefCell::new(None));
        let result_slot = output.clone();
        let storage = self.state.storage();
        let sql = DurableSql(storage.sql());
        // Business errors MUST reject the transaction callback, not be returned as Ok(error JSON).
        let transaction = storage
            .transaction(move |_txn| async move {
                let result = (|| {
                    match cfg.model {
                        DataModel::Legacy => skarma_core::store::initialize(&sql)?,
                        DataModel::PrdV1 => {
                            if cfg.initialize_empty_prd {
                                skarma_core::foundation::transfer::initialize_empty_preserving_legacy(&sql)?;
                            } else {
                                skarma_core::foundation::initialize_authenticated(&sql)?;
                            }
                        }
                    }
                    if let Some(input) = initial_admin
                        && sql
                            .query("SELECT id FROM auth_users LIMIT 1", &[])?
                            .is_empty()
                    {
                        skarma_core::auth::bootstrap_admin(&sql, input)?;
                    }
                    if cfg.demo {
                        skarma_core::store::seed_demo(&sql)?;
                    }
                    if let Some((filename, bytes)) = upload {
                        skarma_core::execute(&sql, command.clone())?;
                        let input = skarma_http::import::parse(&filename, &bytes)?;
                        let preview = skarma_http::import::preview_command(command, input)?;
                        skarma_core::execute(&sql, preview)
                    } else {
                        skarma_core::execute(&sql, command)
                    }
                })();
                let failed = result.is_err();
                *result_slot.borrow_mut() = Some(result);
                if failed {
                    return Err(worker::Error::RustError("command rolled back".into()));
                }
                Ok(())
            })
            .await;
        let outcome = output
            .borrow_mut()
            .take()
            .unwrap_or_else(|| Err(ApiError::storage()));
        // A storage commit failure takes precedence over a successful business result.
        let outcome = if transaction.is_err() && outcome.is_ok() {
            Err(ApiError::storage())
        } else {
            outcome
        };
        if export {
            let bytes = outcome.and_then(skarma_http::export::workbook_value);
            return match bytes {
                Ok(bytes) => Response::from_bytes(bytes),
                Err(error) => Response::from_json(&serde_json::json!({"error":error.1}))
                    .map(|response| response.with_status(error.0.as_u16())),
            };
        }
        match outcome {
            Ok(value) => Response::from_json(&value),
            Err(error) => Response::from_json(&serde_json::json!({"error":error.1}))
                .map(|response| response.with_status(error.0.as_u16())),
        }
    }
}

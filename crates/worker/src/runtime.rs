use axum::{body::Body, http::StatusCode};
use skarma_core::{
    Command,
    config::{Config, Runtime},
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
impl Repository for DurableRepository {
    fn execute(
        &self,
        command: Command,
    ) -> impl std::future::Future<Output = AppResult<serde_json::Value>> + Send {
        // The SDK's SendFuture bridge is valid in the single-threaded Worker runtime.
        SendFuture::new(async move {
            let namespace = self
                .env
                .durable_object("SKARMA_DB")
                .map_err(|_| ApiError::storage())?;
            // Prototype space is server configured. Public profile selection requires authorization.
            let stub = namespace
                .id_from_name(&self.space)
                .and_then(|id| id.get_stub())
                .map_err(|_| ApiError::storage())?;
            let mut init = RequestInit::new();
            init.with_method(Method::Post)
                .with_body(Some(serde_json::to_string(&command)?.into()));
            let req = Request::new_with_init("https://skarma.internal/command", &init)
                .map_err(|_| ApiError::storage())?;
            let mut response = stub
                .fetch_with_request(req)
                .await
                .map_err(|_| ApiError::storage())?;
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
}

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, _ctx: Context) -> Result<axum::http::Response<Body>> {
    let cfg = match config(&env) {
        Ok(cfg) => cfg,
        Err(error) => return Ok(axum::response::IntoResponse::into_response(error)),
    };
    let repo = DurableRepository {
        env,
        space: cfg.space,
    };
    Ok(skarma_http::router(repo).call(req).await?)
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
        if req.method() != Method::Post || req.path() != "/command" {
            return Response::error("Unknown internal command", 404);
        }
        let cfg = match config(&self.env) {
            Ok(cfg) => cfg,
            Err(error) => {
                return Response::from_json(&serde_json::json!({"error":error.1}))
                    .map(|response| response.with_status(error.0.as_u16()));
            }
        };
        let command: Command = req.json().await?;
        let output = Rc::new(RefCell::new(None));
        let result_slot = output.clone();
        let storage = self.state.storage();
        let sql = DurableSql(storage.sql());
        // Business errors MUST reject the transaction callback, not be returned as Ok(error JSON).
        let transaction = storage
            .transaction(move |_txn| async move {
                let result = (|| {
                    skarma_core::store::initialize(&sql)?;
                    if cfg.demo {
                        skarma_core::store::seed_demo(&sql)?;
                    }
                    skarma_core::execute(&sql, command)
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
        match outcome {
            Ok(value) => Response::from_json(&value),
            Err(error) => Response::from_json(&serde_json::json!({"error":error.1}))
                .map(|response| response.with_status(error.0.as_u16())),
        }
    }
}

use axum::{Json, Router, routing::get};
use serde::Serialize;
use tower_service::Service;
use worker::*;

#[derive(Serialize)]
struct Health {
    language: &'static str,
    framework: &'static str,
}
async fn health() -> Json<Health> {
    Json(Health {
        language: "Rust",
        framework: "Axum 0.8",
    })
}
#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    if req.uri().path().starts_with("/probe/") {
        let ns = env.durable_object("PROBE")?;
        let stub = ns.id_from_name("synthetic-only")?.get_stub()?;
        let resp = stub.fetch_with_request(req.try_into()?).await?;
        let resp: axum::http::Response<worker::Body> = resp.try_into()?;
        return Ok(resp.map(axum::body::Body::new));
    }
    Ok(Router::new()
        .route("/api/health", get(health))
        .call(req)
        .await?)
}

#[durable_object]
pub struct Probe {
    storage: Storage,
}
impl DurableObject for Probe {
    fn new(state: State, _env: Env) -> Self {
        let storage = state.storage();
        storage
            .sql()
            .exec(
                "CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY, kind TEXT);",
                None,
            )
            .expect("schema");
        Self { storage }
    }
    async fn fetch(&self, req: worker::Request) -> Result<worker::Response> {
        match req.path().as_str() {
            "/probe/reset" => {
                self.storage.sql().exec("DELETE FROM items;", None)?;
                Response::ok("reset")
            }
            "/probe/fail" | "/probe/commit" => {
                let fail = req.path() == "/probe/fail";
                let sql = self.storage.sql();
                let outcome = self
                    .storage
                    .transaction(move |_txn| async move {
                        sql.exec("INSERT INTO items VALUES (1, 'session');", None)?;
                        sql.exec("INSERT INTO items VALUES (2, 'difficulty');", None)?;
                        if fail {
                            return Err(Error::RustError("intentional failure".into()));
                        }
                        Ok(())
                    })
                    .await;
                match outcome {
                    Ok(()) => Response::ok("committed"),
                    Err(_) => Ok(Response::ok("rolled back")?.with_status(409)),
                }
            }
            "/probe/count" => {
                #[derive(serde::Deserialize, Serialize)]
                struct Row {
                    count: i32,
                }
                let rows: Vec<Row> = self
                    .storage
                    .sql()
                    .exec("SELECT count(*) AS count FROM items;", None)?
                    .to_array()?;
                Response::from_json(&rows)
            }
            _ => Response::error("unknown probe", 404),
        }
    }
}

use rusqlite::Connection;
use skarma_api::{Database, router, store};
use skarma_core::config::{Config, Runtime};
use std::{
    env,
    path::Path,
    sync::{Arc, Mutex},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(Runtime::Local, |key| env::var(key).ok())?;
    // This first prototype intentionally serves only the local development machine.
    // Public deployment needs authenticated identities and profile authorization.
    let path = env::var("SKARMA_DATABASE").unwrap_or_else(|_| ".local/skarma.db".into());
    if let Some(parent) = Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut connection = Connection::open(path)?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    store::initialize(&connection)?;
    if config.demo {
        store::seed_demo(&mut connection)?;
    }
    let origin = env::var("SKARMA_WEB_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:8081".into())
        .parse()?;
    let assets = env::var("SKARMA_WEB_ASSETS").unwrap_or_else(|_| "apps/client/dist".into());
    let db = Database(Arc::new(Mutex::new(connection)));
    let port: u16 = env::var("SKARMA_PORT")
        .unwrap_or_else(|_| "3001".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let port = listener.local_addr()?.port();
    eprintln!("skarma development server listening on port {port}");
    axum::serve(listener, router(db, origin, &assets)).await?;
    Ok(())
}

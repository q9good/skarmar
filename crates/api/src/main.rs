use rusqlite::Connection;
use skarma_api::{Database, router_with_auth, store};
use skarma_core::{
    auth::{AccountRole, NewUser},
    config::{Config, DataModel, Runtime},
};
use skarma_http::auth::AuthConfig;
use std::{
    env,
    io::{Read, Write},
    path::Path,
    sync::{Arc, Mutex},
};

fn password(stdin: bool) -> Result<String, Box<dyn std::error::Error>> {
    if stdin {
        let mut value = String::new();
        std::io::stdin().take(130).read_to_string(&mut value)?;
        Ok(value.trim_end_matches(['\r', '\n']).into())
    } else {
        let first = rpassword::prompt_password("密码（不能为空）: ")?;
        let second = rpassword::prompt_password("再次输入密码: ")?;
        if first != second {
            return Err("两次密码不一致".into());
        }
        Ok(first)
    }
}
fn option(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
fn healthcheck() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = env::var("SKARMA_PORT")
        .unwrap_or_else(|_| "3001".into())
        .parse()?;
    let mut stream = std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse()?,
        std::time::Duration::from_secs(3),
    )?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;
    stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    if !response.starts_with("HTTP/1.1 200") || !response.contains("\"status\":\"ok\"") {
        return Err("health check failed".into());
    }
    Ok(())
}

fn restore_stdin(path: &str, model: DataModel) -> Result<(), Box<dyn std::error::Error>> {
    let parent = Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".restore-{}.db", uuid::Uuid::new_v4().simple()));
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        let bytes = std::io::copy(
            &mut std::io::stdin().take(10 * 1024 * 1024 * 1024 + 1),
            &mut file,
        )?;
        if bytes > 10 * 1024 * 1024 * 1024 {
            return Err("恢复文件超过 10 GB".into());
        }
        file.sync_all()?;
        drop(file);
        let mut header = [0u8; 16];
        std::fs::File::open(&temp)?.read_exact(&mut header)?;
        if &header != b"SQLite format 3\0" {
            return Err("恢复文件不是 SQLite 数据库，当前数据库未修改".into());
        }
        let source = Connection::open(&temp)?;
        let check: String = source.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if check != "ok" {
            return Err("恢复文件检查失败，当前数据库未修改".into());
        }
        let tables:i64=source.query_row(if model==DataModel::Legacy {"SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('goals','sessions')"} else {"SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('prd_schema','prd_entities')"},[],|row|row.get(0))?;
        if tables != 2 {
            return Err("恢复文件缺少应用数据表，当前数据库未修改".into());
        }
        // Migrate and validate the private copy before replacing the real database.
        match model {
            DataModel::Legacy => {
                store::initialize(&source)?;
                store::state(&source)?;
            }
            DataModel::PrdV1 => store::initialize_prd_runtime(&source)?,
        }
        source.execute("DELETE FROM auth_sessions", [])?;
        source.backup("main", path, None)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&temp);
    let _ = std::fs::remove_file(format!("{}-wal", temp.display()));
    let _ = std::fs::remove_file(format!("{}-shm", temp.display()));
    result
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("serve");
    if command == "healthcheck" {
        return healthcheck();
    }
    if !matches!(
        command,
        "serve" | "admin-init" | "admin-reset" | "backup" | "restore-stdin" | "data-transfer-stdin"
    ) {
        return Err("命令: serve / admin-init --username USER --name NAME / admin-reset --username USER / backup PATH / restore-stdin --offline / data-transfer-stdin --username USER --offline".into());
    }
    let config = Config::load(Runtime::Local, |key| env::var(key).ok())?;
    let auth = if command == "serve" {
        Some(AuthConfig::load(&config, |key| env::var(key).ok())?)
    } else {
        None
    };
    let path = env::var("SKARMA_DATABASE").unwrap_or_else(|_| ".local/skarma.db".into());
    if command == "restore-stdin" {
        if !args.iter().any(|arg| arg == "--offline") {
            return Err("恢复前必须停止服务，并明确传入 --offline".into());
        }
        restore_stdin(&path, config.model)?;
        eprintln!("恢复完成；所有会话已撤销，请重新登录");
        return Ok(());
    }
    if command == "backup" {
        let target = args.get(1).ok_or("backup 需要目标文件名")?;
        let target = Path::new(target);
        if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let db = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let check: String = db.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if check != "ok" {
            return Err("数据库检查失败，备份已中止".into());
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(target)?;
        drop(file);
        if let Err(e) = db.backup("main", target, None) {
            let _ = std::fs::remove_file(target);
            return Err(e.into());
        }
        let copy = Connection::open_with_flags(target, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let check: String = copy.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if check != "ok" {
            return Err("备份检查失败".into());
        }
        eprintln!("数据库一致性备份完成");
        return Ok(());
    }
    if command == "data-transfer-stdin" && !Path::new(&path).is_file() {
        return Err("维护命令要求已有数据库，拒绝创建空库".into());
    }
    if let Some(parent) = Path::new(&path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut connection = Connection::open(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
    if command == "data-transfer-stdin" {
        if !args.iter().any(|a| a == "--offline") {
            return Err("数据转换命令仅供停止服务后的离线维护，请传入 --offline".into());
        }
        let username = option(&args, "--username").ok_or("请指定已有管理员 --username")?;
        let actor = connection.query_row(
            "SELECT id,username,display_name,role,active FROM auth_users WHERE username=?1",
            [username],
            |r| {
                Ok(skarma_core::auth::User {
                    id: r.get(0)?,
                    username: r.get(1)?,
                    display_name: r.get(2)?,
                    role: if r.get::<_, String>(3)? == "admin" {
                        AccountRole::Admin
                    } else {
                        AccountRole::Member
                    },
                    active: r.get::<_, i64>(4)? == 1,
                })
            },
        )?;
        let mut input = String::new();
        std::io::stdin()
            .take(1_000_001)
            .read_to_string(&mut input)?;
        if input.len() > 1_000_000 {
            return Err("维护输入超过1MB".into());
        }
        let command = serde_json::from_str(&input)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&store::data_transfer(&mut connection, &actor, command)?)?
        );
        return Ok(());
    }
    match config.model {
        DataModel::Legacy => store::initialize(&connection)?,
        DataModel::PrdV1 if config.initialize_empty_prd => {
            store::initialize_empty_prd_baseline(&connection)?
        }
        DataModel::PrdV1 => store::initialize_prd_runtime(&connection)?,
    }
    if matches!(command, "admin-init" | "admin-reset") {
        let username = option(&args, "--username").ok_or("请用 --username 指定账号")?;
        let password = password(args.iter().any(|arg| arg == "--password-stdin"))?;
        if command == "admin-init" {
            store::bootstrap_admin(
                &mut connection,
                NewUser {
                    display_name: option(&args, "--name").unwrap_or_else(|| username.clone()),
                    username,
                    password,
                    role: AccountRole::Admin,
                },
            )?;
            eprintln!("管理员已创建");
        } else {
            store::recover_admin(&mut connection, &username, &password)?;
            eprintln!("管理员密码已重置，旧会话已撤销");
        }
        return Ok(());
    }
    if config.demo {
        store::seed_demo(&mut connection)?;
    }
    let auth = auth.unwrap();
    let origin = env::var("SKARMA_PUBLIC_URL")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| env::var("SKARMA_WEB_ORIGIN").ok())
        .unwrap_or_else(|| "http://localhost:8081".into())
        .trim_end_matches('/')
        .parse()?;
    let assets = env::var("SKARMA_WEB_ASSETS").unwrap_or_else(|_| "apps/client/dist".into());
    let db = Database(Arc::new(Mutex::new(connection)));
    let port: u16 = env::var("SKARMA_PORT")
        .unwrap_or_else(|_| "3001".into())
        .parse()?;
    let bind: std::net::IpAddr = env::var("SKARMA_BIND")
        .unwrap_or_else(|_| "127.0.0.1".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind((bind, port)).await?;
    let port = listener.local_addr()?.port();
    eprintln!(
        "skarma server listening on port {port}; authentication {}",
        if auth.demo { "demo" } else { "required" }
    );
    axum::serve(listener, router_with_auth(db, origin, &assets, auth)).await?;
    Ok(())
}

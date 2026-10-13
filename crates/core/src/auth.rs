//! Account and session rules shared by native SQLite and Durable Objects.
//! Every command runs inside the adapter's transaction. Rejected login outcomes
//! are values so that attempt counters commit even when credentials are wrong.
use crate::{
    sql::{Sql, params},
    store::{ApiError, Result},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const SESSION_SECONDS: i64 = 7 * 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRole {
    Admin,
    Member,
}
impl AccountRole {
    fn name(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: AccountRole,
    pub active: bool,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct NewUser {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: AccountRole,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct UserUpdate {
    pub active: Option<bool>,
    pub password: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AuthCommand {
    Login {
        username: String,
        password: String,
    },
    Session {
        token: String,
    },
    Logout {
        token: String,
    },
    ListUsers {
        token: String,
    },
    CreateUser {
        token: String,
        input: NewUser,
    },
    UpdateUser {
        token: String,
        id: String,
        input: UserUpdate,
    },
    ChangePassword {
        token: String,
        current_password: String,
        password: String,
    },
}
#[derive(Deserialize, Serialize)]
pub struct LoginOutcome {
    pub user: Option<User>,
    pub token: Option<String>,
    pub rate_limited: bool,
}

pub fn unauthorized() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "请登录后继续".into())
}
fn forbidden() -> ApiError {
    ApiError(StatusCode::FORBIDDEN, "只有管理员可以管理成员".into())
}
fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
fn normalize(username: &str) -> Result<String> {
    let value = username.trim().to_ascii_lowercase();
    if value.len() < 3
        || value.len() > 64
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err(ApiError::invalid(
            "账号使用 3–64 位英文字母、数字、点、下划线或短横线",
        ));
    }
    Ok(value)
}
fn hash_password(password: &str) -> Result<String> {
    if password.chars().count() < 12 || password.len() > 128 {
        return Err(ApiError::invalid("密码至少 12 个字符，最多 128 字节"));
    }
    let salt =
        SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(|_| ApiError::storage())?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::storage())
}
fn matches(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}
fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
fn user(row: &[String]) -> Result<User> {
    Ok(User {
        id: row[0].clone(),
        username: row[1].clone(),
        display_name: row[2].clone(),
        role: match row[3].as_str() {
            "admin" => AccountRole::Admin,
            "member" => AccountRole::Member,
            _ => return Err(ApiError::storage()),
        },
        active: row[4] == "1",
    })
}
const USER_COLUMNS: &str = "id,username,display_name,role,CAST(active AS TEXT)";

pub fn session(db: &dyn Sql, token: &str) -> Result<User> {
    if token.len() != 64 || !token.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(unauthorized());
    }
    let rows = db.query("SELECT u.id,u.username,u.display_name,u.role,CAST(u.active AS TEXT) FROM auth_sessions s JOIN auth_users u ON u.id=s.user_id WHERE s.token_hash=?1 AND s.expires_at>?2 AND u.active=1",
        params![token_hash(token), now()])?;
    rows.first()
        .map(|row| user(row))
        .unwrap_or_else(|| Err(unauthorized()))
}
fn admin(db: &dyn Sql, token: &str) -> Result<User> {
    let user = session(db, token)?;
    if user.role != AccountRole::Admin {
        return Err(forbidden());
    }
    Ok(user)
}
fn create(db: &dyn Sql, input: NewUser) -> Result<User> {
    let username = normalize(&input.username)?;
    let name = input.display_name.trim();
    if name.is_empty() || name.len() > 120 {
        return Err(ApiError::invalid("请填写称呼（最多 120 字节）"));
    }
    if !db
        .query(
            "SELECT id FROM auth_users WHERE username=?1",
            params![username],
        )?
        .is_empty()
    {
        return Err(ApiError::conflict("这个账号已存在"));
    }
    let hash = hash_password(&input.password)?;
    let id = Uuid::new_v4().to_string();
    db.execute("INSERT INTO auth_users(id,username,display_name,password_hash,role,active,created_at) VALUES (?1,?2,?3,?4,?5,1,?6)",
        params![id, username, name, hash, input.role.name(), chrono::Utc::now().to_rfc3339()])?;
    Ok(User {
        id,
        username,
        display_name: name.into(),
        role: input.role,
        active: true,
    })
}
/// Invoked by the local administrator CLI, never exposed as an HTTP route.
pub fn bootstrap_admin(db: &dyn Sql, mut input: NewUser) -> Result<User> {
    if !db
        .query("SELECT id FROM auth_users LIMIT 1", &[])?
        .is_empty()
    {
        return Err(ApiError::conflict("已有账号，请通过管理员登录管理成员"));
    }
    input.role = AccountRole::Admin;
    create(db, input)
}

/// Recovery requires direct access to the database on the host; no HTTP equivalent.
pub fn recover_admin(db: &dyn Sql, username: &str, password: &str) -> Result<()> {
    let name = normalize(username)?;
    let rows = db.query(
        "SELECT id FROM auth_users WHERE username=?1 AND role='admin'",
        params![name],
    )?;
    let id = rows
        .first()
        .map(|row| row[0].clone())
        .ok_or_else(|| ApiError::invalid("管理员账号不存在"))?;
    let hash = hash_password(password)?;
    db.execute(
        "UPDATE auth_users SET password_hash=?1,active=1 WHERE id=?2",
        params![hash, id],
    )?;
    db.execute("DELETE FROM auth_sessions WHERE user_id=?1", params![id])?;
    db.execute(
        "DELETE FROM auth_login_attempts WHERE key=?1 OR key='global'",
        params![format!("user:{name}")],
    )
}

fn limited(db: &dyn Sql, key: &str, time: i64) -> Result<bool> {
    let rows = db.query(
        "SELECT CAST(locked_until AS TEXT) FROM auth_login_attempts WHERE key=?1",
        params![key],
    )?;
    rows.first()
        .map(|row| {
            row[0]
                .parse::<i64>()
                .map(|n| n > time)
                .map_err(|_| ApiError::storage())
        })
        .unwrap_or(Ok(false))
}
fn failure(db: &dyn Sql, key: &str, time: i64, window: i64, max: i64) -> Result<()> {
    let rows = db.query("SELECT CAST(failures AS TEXT),CAST(window_start AS TEXT) FROM auth_login_attempts WHERE key=?1", params![key])?;
    let (count, start) = if let Some(row) = rows.first() {
        let start = row[1].parse::<i64>().map_err(|_| ApiError::storage())?;
        if time - start < window {
            (
                row[0].parse::<i64>().map_err(|_| ApiError::storage())? + 1,
                start,
            )
        } else {
            (1, time)
        }
    } else {
        (1, time)
    };
    let lock = if count >= max { time + window } else { 0 };
    db.execute("INSERT INTO auth_login_attempts(key,failures,window_start,locked_until) VALUES (?1,?2,?3,?4) ON CONFLICT(key) DO UPDATE SET failures=excluded.failures,window_start=excluded.window_start,locked_until=excluded.locked_until", params![key,count,start,lock])
}
fn login(db: &dyn Sql, username: &str, password: &str) -> Result<LoginOutcome> {
    let name = normalize(username)?;
    if password.len() > 128 {
        return Err(ApiError::invalid("密码最多 128 字节"));
    }
    let time = now();
    let key = format!("user:{name}");
    if limited(db, &key, time)? || limited(db, "global", time)? {
        return Ok(LoginOutcome {
            user: None,
            token: None,
            rate_limited: true,
        });
    }
    let rows = db.query(
        &format!("SELECT {USER_COLUMNS},password_hash FROM auth_users WHERE username=?1"),
        params![name],
    )?;
    let valid = if let Some(row) = rows.first() {
        matches(password, &row[5]) && row[4] == "1"
    } else {
        // Spend the same password-hash work for an unknown account.
        let salt = SaltString::encode_b64(b"skarma-dummy-salt").map_err(|_| ApiError::storage())?;
        let _ = Argon2::default().hash_password(password.as_bytes(), &salt);
        false
    };
    if !valid {
        db.execute(
            "DELETE FROM auth_login_attempts WHERE locked_until<=?1 AND window_start<?2",
            params![time, time - 900],
        )?;
        failure(db, &key, time, 900, 5)?;
        failure(db, "global", time, 60, 30)?;
        return Ok(LoginOutcome {
            user: None,
            token: None,
            rate_limited: false,
        });
    }
    let user = user(&rows[0])?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    db.execute("DELETE FROM auth_login_attempts WHERE key=?1", params![key])?;
    db.execute(
        "DELETE FROM auth_sessions WHERE expires_at<=?1",
        params![time],
    )?;
    db.execute("DELETE FROM auth_sessions WHERE token_hash IN (SELECT token_hash FROM auth_sessions WHERE user_id=?1 ORDER BY created_at DESC,token_hash LIMIT -1 OFFSET 9)", params![user.id])?;
    db.execute(
        "INSERT INTO auth_sessions(token_hash,user_id,created_at,expires_at) VALUES (?1,?2,?3,?4)",
        params![token_hash(&token), user.id, time, time + SESSION_SECONDS],
    )?;
    Ok(LoginOutcome {
        user: Some(user),
        token: Some(token),
        rate_limited: false,
    })
}

pub fn execute(db: &dyn Sql, command: AuthCommand) -> Result<serde_json::Value> {
    use AuthCommand::*;
    Ok(match command {
        Login { username, password } => serde_json::to_value(login(db, &username, &password)?)?,
        Session { token } => serde_json::to_value(session(db, &token)?)?,
        Logout { token } => {
            db.execute(
                "DELETE FROM auth_sessions WHERE token_hash=?1",
                params![token_hash(&token)],
            )?;
            serde_json::json!({"ok":true})
        }
        ListUsers { token } => {
            admin(db, &token)?;
            let rows = db.query(
                &format!("SELECT {USER_COLUMNS} FROM auth_users ORDER BY created_at,id"),
                &[],
            )?;
            serde_json::to_value(
                rows.iter()
                    .map(|row| user(row))
                    .collect::<Result<Vec<_>>>()?,
            )?
        }
        CreateUser { token, input } => {
            admin(db, &token)?;
            serde_json::to_value(create(db, input)?)?
        }
        UpdateUser { token, id, input } => {
            let actor = admin(db, &token)?;
            let rows = db.query(
                &format!("SELECT {USER_COLUMNS} FROM auth_users WHERE id=?1"),
                params![id],
            )?;
            let mut target = rows
                .first()
                .map(|row| user(row))
                .unwrap_or_else(|| Err(ApiError(StatusCode::NOT_FOUND, "成员不存在".into())))?;
            if actor.id == id && input.active == Some(false) {
                return Err(ApiError::invalid("不能停用当前管理员"));
            }
            if let Some(active) = input.active {
                db.execute(
                    "UPDATE auth_users SET active=?1 WHERE id=?2",
                    params![i64::from(active), id],
                )?;
                target.active = active;
            }
            if let Some(password) = input.password {
                let hash = hash_password(&password)?;
                db.execute(
                    "UPDATE auth_users SET password_hash=?1 WHERE id=?2",
                    params![hash, id],
                )?;
            }
            db.execute("DELETE FROM auth_sessions WHERE user_id=?1", params![id])?;
            serde_json::to_value(target)?
        }
        ChangePassword {
            token,
            current_password,
            password,
        } => {
            let actor = session(db, &token)?;
            if current_password.len() > 128 {
                return Err(ApiError::invalid("当前密码不正确"));
            }
            let rows = db.query(
                "SELECT password_hash FROM auth_users WHERE id=?1",
                params![actor.id],
            )?;
            if !matches(&current_password, &rows[0][0]) {
                return Err(ApiError::invalid("当前密码不正确"));
            }
            let hash = hash_password(&password)?;
            db.execute(
                "UPDATE auth_users SET password_hash=?1 WHERE id=?2",
                params![hash, actor.id],
            )?;
            db.execute(
                "DELETE FROM auth_sessions WHERE user_id=?1",
                params![actor.id],
            )?;
            serde_json::json!({"ok":true})
        }
    })
}

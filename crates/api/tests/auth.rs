use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::{Database, router_with_auth, store};
use skarma_core::{
    Command,
    auth::{AccountRole, AuthCommand, NewUser},
    config::{Config, Runtime},
};
use skarma_http::Repository;
use skarma_http::auth::AuthConfig;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const PASSWORD: &str = "synthetic-owner-password";
fn fixture() -> (Database, Router) {
    let mut connection = Connection::open_in_memory().unwrap();
    store::initialize(&connection).unwrap();
    store::bootstrap_admin(
        &mut connection,
        NewUser {
            username: "owner".into(),
            display_name: "测试管理员".into(),
            password: PASSWORD.into(),
            role: AccountRole::Admin,
        },
    )
    .unwrap();
    let db = Database(Arc::new(Mutex::new(connection)));
    let app = router_with_auth(
        db.clone(),
        "https://app.example.test".parse().unwrap(),
        "/tmp/skarma-no-assets",
        AuthConfig {
            demo: false,
            secure_cookie: true,
            origins: vec!["https://app.example.test".into()],
            space: "family".into(),
        },
    );
    (db, app)
}
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("x-skarma-request", "1")
        .header("origin", "https://app.example.test");
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    if body.is_some() {
        request = request.header("content-type", "application/json");
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
    (status, headers, serde_json::from_slice(&bytes).unwrap())
}
async fn login(app: &Router, user: &str, password: &str) -> String {
    let (status, headers, body) = send(
        app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"username":user,"password":password})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("token").is_none());
    assert!(body["user"].get("password_hash").is_none());
    let cookie = headers["set-cookie"].to_str().unwrap();
    assert!(cookie.starts_with("__Host-skarma_session="));
    for attribute in ["HttpOnly", "SameSite=Strict", "Secure", "Path=/"] {
        assert!(cookie.contains(attribute));
    }
    cookie.split(';').next().unwrap().into()
}

fn registration(username: &str, email: &str) -> Value {
    json!({
        "username": username,
        "display_name": "新成员",
        "email": email,
        "password": "synthetic-registration-password"
    })
}

#[tokio::test]
async fn daily_class_records_require_a_live_session() {
    let (_, app) = fixture();
    let path = format!("/api/lessons/{}", uuid::Uuid::new_v4());
    let body = json!({"expected_version":null,"lesson":{
        "business_date":"2026-10-18","course":"音乐课","teacher":"王老师",
        "content":"唱歌","goals":"跟唱","problems":"","home_cooperation":""
    }});
    assert_eq!(
        send(&app, "PUT", &path, None, Some(body.clone())).await.0,
        StatusCode::UNAUTHORIZED
    );
    let cookie = login(&app, "owner", PASSWORD).await;
    assert_eq!(
        send(&app, "PUT", &path, Some(&cookie), Some(body)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, "GET", "/api/state", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&cookie), None).await.2["lessons"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn registration_requires_admin_approval_before_login_and_never_exposes_hashes() {
    let (db, app) = fixture();
    let mut input = registration(" applicant ", " Applicant@Example.Test ");
    input["password"] = json!("123456");
    let (status, headers, body) = send(&app, "POST", "/api/auth/register", None, Some(input)).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body, json!({"ok":true}));
    assert!(headers.get("set-cookie").is_none());
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"username":"applicant","password":"123456"}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&app, "GET", "/api/auth/registrations", None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let owner = login(&app, "owner", PASSWORD).await;
    let (status, _, requests) =
        send(&app, "GET", "/api/auth/registrations", Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(requests.as_array().unwrap().len(), 1);
    let pending = &requests[0];
    assert_eq!(pending["username"], "applicant");
    assert_eq!(pending["email"], "applicant@example.test");
    assert!(pending["created_at"].is_string());
    assert!(pending.get("password").is_none());
    assert!(pending.get("password_hash").is_none());
    let id = pending["id"].as_str().unwrap();
    let hash_before: String =
        db.0.lock()
            .unwrap()
            .query_row(
                "SELECT password_hash FROM auth_registration_requests WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
    assert!(hash_before.starts_with("$argon2id$"));
    let path = format!("/api/auth/registrations/{id}/approve");
    assert_eq!(
        send(&app, "POST", &path, None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, _, body) = send(&app, "POST", &path, Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"ok":true}));
    assert_eq!(
        send(&app, "POST", &path, Some(&owner), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "GET", "/api/auth/registrations", Some(&owner), None)
            .await
            .2,
        json!([])
    );
    let stored: (String, String, String) =
        db.0.lock()
            .unwrap()
            .query_row(
                "SELECT email,password_hash,role FROM auth_users WHERE username='applicant'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
    assert_eq!(stored.0, "applicant@example.test");
    assert_eq!(stored.1, hash_before);
    assert_eq!(stored.2, "member");
    let member = login(&app, "applicant", "123456").await;
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&member), None).await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn registration_accepts_one_character_but_rejects_blank_and_oversized_passwords() {
    let (db, app) = fixture();
    for password in [
        String::new(),
        " \t\n".into(),
        "a".repeat(129),
        "字".repeat(43),
    ] {
        let mut input = registration("shortpass", "shortpass@example.test");
        input["password"] = json!(password);
        assert_eq!(
            send(&app, "POST", "/api/auth/register", None, Some(input))
                .await
                .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        db.0.lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM auth_registration_requests",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    let mut input = registration("shortpass", "shortpass@example.test");
    input["password"] = json!("字");
    assert_eq!(
        send(&app, "POST", "/api/auth/register", None, Some(input))
            .await
            .0,
        StatusCode::ACCEPTED
    );
    let owner = login(&app, "owner", PASSWORD).await;
    let pending = send(&app, "GET", "/api/auth/registrations", Some(&owner), None)
        .await
        .2;
    let path = format!(
        "/api/auth/registrations/{}/approve",
        pending[0]["id"].as_str().unwrap()
    );
    assert_eq!(
        send(&app, "POST", &path, Some(&owner), None).await.0,
        StatusCode::OK
    );
    login(&app, "shortpass", "字").await;
}

#[tokio::test]
async fn member_cannot_review_and_rejection_allows_new_application() {
    let (_, app) = fixture();
    let owner = login(&app, "owner", PASSWORD).await;
    let (_, _, member) = send(&app, "POST", "/api/auth/users", Some(&owner),
        Some(json!({"username":"member","display_name":"成员","password":"synthetic-member-password","role":"member"}))).await;
    assert!(member["id"].is_string());
    let member_cookie = login(&app, "member", "synthetic-member-password").await;
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("candidate", "candidate@example.test"))
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let requests = send(&app, "GET", "/api/auth/registrations", Some(&owner), None)
        .await
        .2;
    let id = requests[0]["id"].as_str().unwrap();
    for action in ["approve", "reject"] {
        let path = format!("/api/auth/registrations/{id}/{action}");
        assert_eq!(
            send(&app, "POST", &path, Some(&member_cookie), None)
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        send(
            &app,
            "GET",
            "/api/auth/registrations",
            Some(&member_cookie),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let path = format!("/api/auth/registrations/{id}/reject");
    assert_eq!(
        send(&app, "POST", &path, Some(&owner), None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, "POST", &path, Some(&owner), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("candidate", "candidate@example.test"))
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"username":"candidate","password":"synthetic-registration-password"}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn registration_validates_email_conflicts_and_commits_throttle_counters() {
    let (db, app) = fixture();
    for email in [
        "missing-at",
        "a@localhost",
        "a@-bad.test",
        "a..b@example.test",
    ] {
        assert_eq!(
            send(
                &app,
                "POST",
                "/api/auth/register",
                None,
                Some(registration("candidate", email))
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("owner", "different@example.test"))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("candidate", "candidate@example.test"))
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("candidate", "different@example.test"))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("other", "candidate@example.test"))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    for _ in 0..3 {
        assert_eq!(
            send(
                &app,
                "POST",
                "/api/auth/register",
                None,
                Some(registration("candidate", "different@example.test"))
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
    }
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("candidate", "different@example.test"))
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        db.0.lock()
            .unwrap()
            .query_row(
                "SELECT attempts FROM auth_registration_limits WHERE key='user:candidate'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        5
    );
    assert_eq!(
        db.0.lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM auth_registration_requests",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn approval_conflict_keeps_pending_request_for_review() {
    let (db, app) = fixture();
    let owner = login(&app, "owner", PASSWORD).await;
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("candidate", "candidate@example.test"))
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
    let id = send(&app, "GET", "/api/auth/registrations", Some(&owner), None)
        .await
        .2[0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    db.0.lock()
        .unwrap()
        .execute(
            "UPDATE auth_users SET email='candidate@example.test' WHERE username='owner'",
            [],
        )
        .unwrap();
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/auth/registrations/{id}/approve"),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        send(&app, "GET", "/api/auth/registrations", Some(&owner), None)
            .await
            .2
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        db.0.lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM auth_users WHERE username='candidate'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn registration_queue_and_global_attempts_are_bounded() {
    let (db, app) = fixture();
    {
        let conn = db.0.lock().unwrap();
        for n in 0..100 {
            conn.execute(
                "INSERT INTO auth_registration_requests(id,username,display_name,email,password_hash,created_at) VALUES (?1,?2,'待审','pending@example.test' || ?3,'hash','2026-01-01')",
                rusqlite::params![format!("pending-{n}"),format!("pending{n}"),n.to_string()],
            ).unwrap();
        }
    }
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("newperson", "newperson@example.test"))
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        db.0.lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM auth_registration_requests",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        100
    );

    let (db, app) = fixture();
    for n in 0..60 {
        let status = send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("owner", &format!("attempt{n}@example.test"))),
        )
        .await
        .0;
        assert!(matches!(
            status,
            StatusCode::CONFLICT | StatusCode::TOO_MANY_REQUESTS
        ));
    }
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/register",
            None,
            Some(registration("fresh", "fresh@example.test"))
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    let conn = db.0.lock().unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT attempts FROM auth_registration_limits WHERE key='global'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        60
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM auth_registration_limits WHERE key='user:fresh'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn registration_rejects_large_anonymous_bodies_before_hashing() {
    let (db, app) = fixture();
    let oversized = registration("candidate", "candidate@example.test")
        .to_string()
        .replace("synthetic-registration-password", &"a".repeat(9 * 1024));
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/register")
        .header("content-type", "application/json")
        .header("x-skarma-request", "1")
        .header("origin", "https://app.example.test")
        .body(Body::from(oversized))
        .unwrap();
    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        db.0.lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM auth_registration_limits", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn private_records_require_a_live_session_and_logout_revokes_it() {
    let (db, app) = fixture();
    let (status, headers, _) = send(&app, "GET", "/api/state", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(headers["cache-control"], "no-store");
    let cookie = login(&app, " OWNER ", PASSWORD).await;
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&cookie), None).await.0,
        StatusCode::OK
    );
    {
        let conn = db.0.lock().unwrap();
        let hash: String = conn
            .query_row("SELECT password_hash FROM auth_users", [], |r| r.get(0))
            .unwrap();
        let session_hash: String = conn
            .query_row("SELECT token_hash FROM auth_sessions", [], |r| r.get(0))
            .unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains(PASSWORD));
        assert!(!cookie.contains(&session_hash));
    }
    assert_eq!(
        send(&app, "POST", "/api/auth/logout", Some(&cookie), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&cookie), None).await.0,
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn expired_and_duplicate_cookies_cannot_read_records() {
    let (db, app) = fixture();
    let cookie = login(&app, "owner", PASSWORD).await;
    let duplicate = format!("{cookie}; {cookie}");
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&duplicate), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    db.0.lock()
        .unwrap()
        .execute("UPDATE auth_sessions SET expires_at=0", [])
        .unwrap();
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&cookie), None).await.0,
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn browser_cross_site_and_form_posts_are_rejected() {
    let (_, app) = fixture();
    let cookie = login(&app, "owner", PASSWORD).await;
    for (header, origin, site) in [
        (false, "https://app.example.test", "same-origin"),
        (true, "https://evil.test", "cross-site"),
        (true, "https://app.example.test", "cross-site"),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/auth/logout")
            .header("cookie", &cookie)
            .header("origin", origin)
            .header("sec-fetch-site", site);
        if header {
            request = request.header("x-skarma-request", "1");
        }
        assert_eq!(
            app.clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&cookie), None).await.0,
        StatusCode::OK
    );
}
#[tokio::test]
async fn member_management_checks_role_and_disabling_or_resetting_revokes_sessions() {
    let (_, app) = fixture();
    let owner = login(&app, "owner", PASSWORD).await;
    let input = json!({"username":"teacher","display_name":"测试老师","role":"member","password":"synthetic-member-password"});
    let (status, _, member) =
        send(&app, "POST", "/api/auth/users", Some(&owner), Some(input)).await;
    assert_eq!(status, StatusCode::OK);
    let member_id = member["id"].as_str().unwrap();
    let member_cookie = login(&app, "teacher", "synthetic-member-password").await;
    assert_eq!(
        send(&app, "GET", "/api/auth/users", Some(&member_cookie), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &app,
            "PUT",
            &format!("/api/auth/users/{member_id}"),
            Some(&owner),
            Some(json!({"active":false}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&member_cookie), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"username":"teacher","password":"synthetic-member-password"}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    send(
        &app,
        "PUT",
        &format!("/api/auth/users/{member_id}"),
        Some(&owner),
        Some(json!({"active":true})),
    )
    .await;
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&member_cookie), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let member_cookie = login(&app, "teacher", "synthetic-member-password").await;
    send(
        &app,
        "PUT",
        &format!("/api/auth/users/{member_id}"),
        Some(&owner),
        Some(json!({"password":"synthetic-reset-password"})),
    )
    .await;
    assert_eq!(
        send(&app, "GET", "/api/state", Some(&member_cookie), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    login(&app, "teacher", "synthetic-reset-password").await;
}
#[tokio::test]
async fn failed_attempts_commit_and_password_change_invalidates_all_sessions() {
    let (db, app) = fixture();
    for _ in 0..5 {
        assert_eq!(
            send(
                &app,
                "POST",
                "/api/auth/login",
                None,
                Some(json!({"username":"owner","password":"wrong"}))
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/auth/login",
            None,
            Some(json!({"username":"owner","password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        db.0.lock()
            .unwrap()
            .query_row(
                "SELECT failures FROM auth_login_attempts WHERE key='user:owner'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        5
    );
    db.0.lock()
        .unwrap()
        .execute("DELETE FROM auth_login_attempts", [])
        .unwrap();
    let first = login(&app, "owner", PASSWORD).await;
    let second = login(&app, "owner", PASSWORD).await;
    assert_eq!(
        send(
            &app,
            "PUT",
            "/api/auth/password",
            Some(&first),
            Some(json!({"current_password":PASSWORD,"password":"synthetic-new-password"}))
        )
        .await
        .0,
        StatusCode::OK
    );
    for cookie in [first, second] {
        assert_eq!(
            send(&app, "GET", "/api/state", Some(&cookie), None).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    login(&app, "owner", "synthetic-new-password").await;
}
#[test]
fn bootstrap_and_recovery_are_local_only_and_deployment_config_fails_closed() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    let input = NewUser {
        username: "owner".into(),
        display_name: "管理员".into(),
        password: PASSWORD.into(),
        role: AccountRole::Admin,
    };
    store::bootstrap_admin(&mut db, input.clone()).unwrap();
    assert!(store::bootstrap_admin(&mut db, input).is_err());
    let result = store::execute(
        &mut db,
        Command::Auth(AuthCommand::Login {
            username: "owner".into(),
            password: PASSWORD.into(),
        }),
    )
    .unwrap();
    let token = result["token"].as_str().unwrap().to_owned();
    store::recover_admin(&mut db, "owner", "synthetic-recovery-password").unwrap();
    assert!(store::execute(&mut db, Command::Auth(AuthCommand::Session { token })).is_err());
    let config = Config::load(Runtime::Local, |_| None).unwrap();
    assert!(
        AuthConfig::load(&config, |key| (key == "SKARMA_PRODUCTION")
            .then(|| "1".into()))
        .is_err()
    );
    assert!(
        AuthConfig::load(&config, |key| (key == "SKARMA_AUTH").then(|| "demo".into())).is_err()
    );
    assert!(
        AuthConfig::load(&config, |key| (key == "SKARMA_PUBLIC_URL")
            .then(|| "http://public.example.test".into()))
        .is_err()
    );
    assert!(
        AuthConfig::load(&config, |key| (key == "SKARMA_PUBLIC_URL")
            .then(|| "https://example.test/path".into()))
        .is_err()
    );
    let extra = AuthConfig::load(&config, |key| match key {
        "SKARMA_PRODUCTION" => Some("1".into()),
        "SKARMA_PUBLIC_URL" => Some("https://evo.example.test".into()),
        "SKARMA_ADDITIONAL_PUBLIC_URLS" => Some("https://temporary.example.test".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(
        extra.origins,
        ["https://evo.example.test", "https://temporary.example.test"]
    );
    assert!(extra.secure_cookie);
    for invalid in [
        "http://temporary.example.test",
        "https://*.example.test",
        "https://example.test,",
    ] {
        assert!(
            AuthConfig::load(&config, |key| match key {
                "SKARMA_PRODUCTION" => Some("1".into()),
                "SKARMA_PUBLIC_URL" => Some("https://evo.example.test".into()),
                "SKARMA_ADDITIONAL_PUBLIC_URLS" => Some(invalid.into()),
                _ => None,
            })
            .is_err()
        );
    }
}

async fn queued_request(path: &str) {
    #[derive(Clone)]
    struct Paused {
        db: Database,
        queued: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    }
    impl Repository for Paused {
        async fn execute(&self, command: Command) -> store::Result<Value> {
            if matches!(command, Command::State | Command::Authenticated { .. }) {
                self.queued.notify_one();
                self.release.notified().await;
            }
            self.db.execute(command).await
        }
        async fn export(&self, command: Command) -> store::Result<Vec<u8>> {
            let snapshot = self.execute(command).await?;
            skarma_http::export::workbook(serde_json::from_value(snapshot)?)
        }
    }
    let (db, _) = fixture();
    let outcome = db
        .execute(Command::Auth(AuthCommand::Login {
            username: "owner".into(),
            password: PASSWORD.into(),
        }))
        .await
        .unwrap();
    let token = outcome["token"].as_str().unwrap().to_owned();
    let queued = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let repo = Paused {
        db: db.clone(),
        queued: queued.clone(),
        release: release.clone(),
    };
    let app = skarma_http::router(repo);
    let path = path.to_owned();
    let task = tokio::spawn(async move {
        app.oneshot(
            Request::builder()
                .uri(path)
                .header("cookie", format!("skarma_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    });
    queued.notified().await;
    db.execute(Command::Auth(AuthCommand::Logout {
        token: outcome["token"].as_str().unwrap().into(),
    }))
    .await
    .unwrap();
    release.notify_one();
    assert_eq!(task.await.unwrap().status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn queued_business_request_is_rechecked_after_session_revocation() {
    queued_request("/api/state").await;
}

#[tokio::test]
async fn queued_export_is_rechecked_after_session_revocation() {
    queued_request("/api/export.xlsx").await;
}

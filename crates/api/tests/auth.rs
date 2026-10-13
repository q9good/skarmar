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
}

#[tokio::test]
async fn queued_business_request_is_rechecked_after_session_revocation() {
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
    let task = tokio::spawn(async move {
        app.oneshot(
            Request::builder()
                .uri("/api/state")
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

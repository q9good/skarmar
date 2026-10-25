use crate::Repository;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::Deserialize;
use serde_json::{Value, json};
use skarma_core::{
    Command,
    auth::{
        AuthCommand, LoginOutcome, NewUser, RegistrationInput, RegistrationOutcome, UserUpdate,
        unauthorized,
    },
    config::Config,
    store::{ApiError, Result},
};

#[derive(Clone, Default)]
pub struct AuthConfig {
    pub demo: bool,
    pub secure_cookie: bool,
    pub origins: Vec<String>,
    pub space: String,
}
impl AuthConfig {
    pub fn load(
        config: &Config,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> std::result::Result<Self, String> {
        let production = match lookup("SKARMA_PRODUCTION").as_deref().unwrap_or("0") {
            "0" => false,
            "1" => true,
            _ => return Err("SKARMA_PRODUCTION 必须是 0 或 1".into()),
        };
        let demo = match lookup("SKARMA_AUTH").as_deref().unwrap_or("required") {
            "required" => false,
            "demo" if config.demo && !production => true,
            _ => return Err("SKARMA_AUTH 必须是 required；仅明确的本机示例环境可使用 demo".into()),
        };
        if production && config.demo {
            return Err("生产环境不能加载示例数据".into());
        }
        let public = lookup("SKARMA_PUBLIC_URL").filter(|value| !value.is_empty());
        if production && public.is_none() {
            return Err("生产环境需要 SKARMA_PUBLIC_URL=https://你的域名".into());
        }
        let mut origins = Vec::new();
        let mut secure_cookie = false;
        if let Some(public) = public {
            let origin = normalize_origin(&public)?;
            secure_cookie = origin.starts_with("https://");
            if !secure_cookie && (production || !loopback(&origin)) {
                return Err("公网访问必须使用 HTTPS；仅本机开发允许 HTTP".into());
            }
            origins.push(origin);
        }
        if !production && let Some(origin) = lookup("SKARMA_WEB_ORIGIN") {
            origins.push(normalize_origin(&origin)?);
        }
        if let Some(additional) = lookup("SKARMA_ADDITIONAL_PUBLIC_URLS") {
            for value in additional.split(',') {
                let value = value.trim();
                if value.is_empty() {
                    return Err("SKARMA_ADDITIONAL_PUBLIC_URLS 不能包含空网址".into());
                }
                let origin = normalize_origin(value)?;
                if production && !origin.starts_with("https://") {
                    return Err("生产环境的额外公开网址必须使用 HTTPS".into());
                }
                if !origins.contains(&origin) {
                    origins.push(origin);
                }
            }
        }
        if origins.len() > 4 {
            return Err("最多允许 4 个精确网址".into());
        }
        Ok(Self {
            demo,
            secure_cookie,
            origins,
            space: config.space.clone(),
        })
    }
    fn cookie_name(&self) -> &'static str {
        if self.secure_cookie {
            "__Host-skarma_session"
        } else {
            "skarma_session"
        }
    }
    fn cookie(&self, token: &str, max_age: i64) -> HeaderValue {
        HeaderValue::from_str(&format!(
            "{}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}{}",
            self.cookie_name(),
            if self.secure_cookie { "; Secure" } else { "" }
        ))
        .expect("server-generated hex cookie")
    }
    fn allows_origin(&self, origin: &str) -> bool {
        self.origins.iter().any(|allowed| allowed == origin)
            || (!self.secure_cookie && loopback(origin))
    }
}
fn normalize_origin(value: &str) -> std::result::Result<String, String> {
    let uri: Uri = value.parse().map_err(|_| "网址格式错误")?;
    if !matches!(uri.scheme_str(), Some("http" | "https"))
        || uri.authority().is_none()
        || uri
            .authority()
            .is_some_and(|a| a.as_str().contains('@') || a.as_str().contains('*'))
        || uri.path() != "/"
        || uri.query().is_some()
    {
        return Err("网址只填写 http(s)://主机[:端口]，不要附加路径、用户名或查询参数".into());
    }
    Ok(format!(
        "{}://{}",
        uri.scheme_str().unwrap(),
        uri.authority().unwrap()
    ))
}
fn loopback(origin: &str) -> bool {
    origin.parse::<Uri>().ok().is_some_and(|uri| {
        matches!(uri.scheme_str(), Some("http" | "https"))
            && matches!(uri.host(), Some("localhost" | "127.0.0.1" | "[::1]"))
    })
}

#[derive(Clone)]
pub(crate) struct AuthState<R: Repository> {
    pub repo: R,
    pub config: AuthConfig,
}
fn token(headers: &HeaderMap, config: &AuthConfig) -> Result<String> {
    let cookies = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let mut matches = cookies.split(';').filter_map(|cookie| {
        let (name, value) = cookie.trim().split_once('=')?;
        (name == config.cookie_name()).then_some(value)
    });
    let value = matches.next().ok_or_else(unauthorized)?;
    if matches.next().is_some()
        || value.len() != 64
        || !value.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(unauthorized());
    }
    Ok(value.into())
}
fn identity(user: Value, config: &AuthConfig) -> Value {
    json!({"user": user, "space": config.space, "demo": config.demo})
}

#[derive(Clone)]
pub(crate) struct SessionProof {
    pub token: String,
    pub expected_actor: Option<String>,
}

pub(crate) async fn require_session<R: Repository>(
    State(state): State<AuthState<R>>,
    mut request: Request,
    next: Next,
) -> Response {
    if request.uri().path().ends_with("/prd/queue")
        && (state.config.demo
            || request
                .headers()
                .get("x-skarma-space")
                .and_then(|v| v.to_str().ok())
                != Some(state.config.space.as_str()))
    {
        return ApiError(
            StatusCode::FORBIDDEN,
            "上传空间不匹配，请用原账号重新打开".into(),
        )
        .into_response();
    }
    if !state.config.demo {
        let token = match token(request.headers(), &state.config) {
            Ok(token) => token,
            Err(e) => return e.into_response(),
        };
        // Validate in the same SQL transaction as the business operation, so a
        // queued request cannot outlive a committed account/session revocation.
        let expected_actor = request
            .headers()
            .get("x-skarma-actor")
            .map(|v| v.to_str().unwrap_or("").to_owned());
        if request
            .headers()
            .get("x-skarma-space")
            .is_some_and(|v| v.to_str().ok() != Some(state.config.space.as_str()))
        {
            return ApiError::conflict(skarma_core::auth::SCOPE_CHANGED).into_response();
        }
        request.extensions_mut().insert(SessionProof {
            token,
            expected_actor,
        });
    }
    next.run(request).await
}
pub(crate) async fn protect_requests<R: Repository>(
    State(state): State<AuthState<R>>,
    request: Request,
    next: Next,
) -> Response {
    if !state.config.demo
        && !matches!(
            *request.method(),
            Method::GET | Method::HEAD | Method::OPTIONS
        )
    {
        let valid_header = request
            .headers()
            .get("x-skarma-request")
            .is_some_and(|value| value == "1");
        let valid_origin = request
            .headers()
            .get(header::ORIGIN)
            .map(|value| {
                value
                    .to_str()
                    .is_ok_and(|origin| state.config.allows_origin(origin))
            })
            .unwrap_or(true);
        let same_site = request
            .headers()
            .get("sec-fetch-site")
            .is_none_or(|value| value != "cross-site");
        if !valid_header || !valid_origin || !same_site {
            let mut response = ApiError(
                StatusCode::FORBIDDEN,
                "请求来源不匹配，请从应用网址重新打开".into(),
            )
            .into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            return response;
        }
    }
    // Account-management routes execute Auth commands directly. A request's
    // cookie token has an immutable user binding; revalidate the same token in
    // the command too so revocation remains effective after this preflight.
    let path = request.uri().path();
    if !state.config.demo
        && path.contains("/auth/")
        && !["/auth/login", "/auth/me", "/auth/register"]
            .iter()
            .any(|p| path.ends_with(p))
        && request.headers().contains_key("x-skarma-actor")
    {
        let expected = request
            .headers()
            .get("x-skarma-actor")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let wrong_space = request
            .headers()
            .get("x-skarma-space")
            .is_some_and(|v| v.to_str().ok() != Some(state.config.space.as_str()));
        let request_token = match token(request.headers(), &state.config) {
            Ok(v) => v,
            Err(e) => return e.into_response(),
        };
        let details = match state
            .repo
            .execute(Command::Auth(AuthCommand::SessionDetails {
                token: request_token,
            }))
            .await
        {
            Ok(v) => v,
            Err(e) => return e.into_response(),
        };
        if details["user"]["id"].as_str() != Some(expected.as_str()) || wrong_space {
            return ApiError::conflict(skarma_core::auth::SCOPE_CHANGED).into_response();
        }
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("same-origin"));
    response
}
#[derive(Deserialize)]
struct Login {
    username: String,
    password: String,
    #[serde(default)]
    trusted_device: bool,
}
async fn register<R: Repository>(
    State(state): State<AuthState<R>>,
    Json(input): Json<RegistrationInput>,
) -> Result<Response> {
    if state.config.demo {
        return Err(ApiError(StatusCode::FORBIDDEN, "示例模式不接受注册".into()));
    }
    let outcome: RegistrationOutcome = serde_json::from_value(
        state
            .repo
            .execute(Command::Auth(AuthCommand::Register { input }))
            .await?,
    )?;
    match outcome {
        RegistrationOutcome::Accepted => {
            Ok((StatusCode::ACCEPTED, Json(json!({"ok":true}))).into_response())
        }
        RegistrationOutcome::Conflict => {
            Err(ApiError::conflict("账号或邮箱已被使用，或申请正在等待审批"))
        }
        RegistrationOutcome::RateLimited => Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "申请次数较多，请稍后再试".into(),
        )),
        RegistrationOutcome::QueueFull => Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "待审批申请较多，请稍后再试".into(),
        )),
    }
}
async fn login<R: Repository>(
    State(state): State<AuthState<R>>,
    Json(input): Json<Login>,
) -> Result<Response> {
    if state.config.demo {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "示例模式没有正式账号".into(),
        ));
    }
    let result: LoginOutcome = serde_json::from_value(
        state
            .repo
            .execute(Command::Auth(if input.trusted_device {
                AuthCommand::LoginTrusted {
                    username: input.username,
                    password: input.password,
                }
            } else {
                AuthCommand::Login {
                    username: input.username,
                    password: input.password,
                }
            }))
            .await?,
    )?;
    let (Some(user), Some(token)) = (result.user, result.token) else {
        return Err(ApiError(
            if result.rate_limited {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::UNAUTHORIZED
            },
            if result.rate_limited {
                "尝试次数较多，请稍后再试"
            } else {
                "账号或密码不正确"
            }
            .into(),
        ));
    };
    let mut response = Json(identity(serde_json::to_value(user)?, &state.config)).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        state.config.cookie(&token, result.session_seconds),
    );
    Ok(response)
}
async fn me<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    if state.config.demo {
        return Ok(Json(identity(
            json!({"id":"demo","username":"demo","display_name":"示例体验","role":"member","active":true}),
            &state.config,
        )));
    }
    let details = state
        .repo
        .execute(Command::Auth(AuthCommand::SessionDetails {
            token: token(&headers, &state.config)?,
        }))
        .await?;
    let mut value = identity(details["user"].clone(), &state.config);
    value["session"] = details["session"].clone();
    Ok(Json(value))
}
async fn logout<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Ok(token) = token(&headers, &state.config) {
        state
            .repo
            .execute(Command::Auth(AuthCommand::Logout { token }))
            .await?;
    }
    let mut response = Json(json!({"ok":true})).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, state.config.cookie("", 0));
    Ok(response)
}
async fn revoke_other_sessions<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    Ok(Json(
        state
            .repo
            .execute(Command::Auth(AuthCommand::RevokeOtherSessions {
                token: token(&headers, &state.config)?,
            }))
            .await?,
    ))
}
async fn users<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    Ok(Json(
        state
            .repo
            .execute(Command::Auth(AuthCommand::ListUsers {
                token: token(&headers, &state.config)?,
            }))
            .await?,
    ))
}
async fn registrations<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    Ok(Json(
        state
            .repo
            .execute(Command::Auth(AuthCommand::ListRegistrations {
                token: token(&headers, &state.config)?,
            }))
            .await?,
    ))
}
async fn approve_registration<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    Ok(Json(
        state
            .repo
            .execute(Command::Auth(AuthCommand::ApproveRegistration {
                token: token(&headers, &state.config)?,
                id,
            }))
            .await?,
    ))
}
async fn reject_registration<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    Ok(Json(
        state
            .repo
            .execute(Command::Auth(AuthCommand::RejectRegistration {
                token: token(&headers, &state.config)?,
                id,
            }))
            .await?,
    ))
}
async fn create_user<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
    Json(input): Json<NewUser>,
) -> Result<Json<Value>> {
    Ok(Json(
        state
            .repo
            .execute(Command::Auth(AuthCommand::CreateUser {
                token: token(&headers, &state.config)?,
                input,
            }))
            .await?,
    ))
}
async fn update_user<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UserUpdate>,
) -> Result<Json<Value>> {
    Ok(Json(
        state
            .repo
            .execute(Command::Auth(AuthCommand::UpdateUser {
                token: token(&headers, &state.config)?,
                id,
                input,
            }))
            .await?,
    ))
}
#[derive(Deserialize)]
struct PasswordChange {
    current_password: String,
    password: String,
}
async fn password<R: Repository>(
    State(state): State<AuthState<R>>,
    headers: HeaderMap,
    Json(input): Json<PasswordChange>,
) -> Result<Response> {
    state
        .repo
        .execute(Command::Auth(AuthCommand::ChangePassword {
            token: token(&headers, &state.config)?,
            current_password: input.current_password,
            password: input.password,
        }))
        .await?;
    let mut response = Json(json!({"ok":true})).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, state.config.cookie("", 0));
    Ok(response)
}
pub(crate) fn routes<R: Repository>(state: AuthState<R>) -> Router {
    Router::new()
        .route("/auth/login", post(login::<R>))
        .route(
            "/auth/register",
            post(register::<R>).layer(DefaultBodyLimit::max(8 * 1024)),
        )
        .route("/auth/me", get(me::<R>))
        .route("/auth/logout", post(logout::<R>))
        .route(
            "/auth/sessions/revoke-others",
            post(revoke_other_sessions::<R>),
        )
        .route("/auth/password", put(password::<R>))
        .route("/auth/users", get(users::<R>).post(create_user::<R>))
        .route("/auth/users/{id}", put(update_user::<R>))
        .route("/auth/registrations", get(registrations::<R>))
        .route(
            "/auth/registrations/{id}/approve",
            post(approve_registration::<R>),
        )
        .route(
            "/auth/registrations/{id}/reject",
            post(reject_registration::<R>),
        )
        .with_state(state)
}

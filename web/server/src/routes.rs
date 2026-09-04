//! Маршруты сервера: вход, здоровье, статика (фича 0531, задача 09a).
//!
//! # Что здесь есть
//!
//! Остов: учётные записи, обмен токенов, `/health` и раздача собранной
//! статики. Ручки проектов заводит задача `09b` — здесь их нет намеренно, и
//! список маршрутов проверяется тестом: маршрут, заведённый мимо этого списка,
//! иначе появился бы у сервиса, не появившись в его описании.
//!
//! # Заголовки кеша
//!
//! Правило то же, что у выкладки: **форма адреса задаёт срок**. Помеченное
//! отпечатком (`/b/<хеш>/…`, `/wasm/<версия>/…`) — год и `immutable`,
//! остальное — `no-cache`. Правило продублировано здесь нарочно: без nginx
//! (своя машина, проба стенда) заголовки обязаны быть теми же, иначе то, что
//! проверено локально, ведёт себя иначе на стенде.

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

use crate::auth::{self, Role, User};
use crate::config::Config;
use crate::error::ApiError;
use crate::rate::{Decision, Window};

/// Общее состояние сервера.
pub struct AppState {
    pub config: Config,
    /// Пул соединений к PostgreSQL: у неё запись параллельна, и одно
    /// соединение под мьютексом создавало бы очередь, которой в базе нет.
    pub pool: deadpool_postgres::Pool,
    pub rate: Window,
}

/// Список маршрутов сервиса.
///
/// ⚠️ Объявлен ЯВНО и проверяется тестом: маршрут, заведённый мимо этого
/// списка, появился бы у сервиса, не появившись в его описании, — и о нём не
/// узнал бы никто, кроме автора.
pub const ROUTES: &[(&str, &str)] = &[
    ("GET", "/health"),
    ("POST", "/api/register"),
    ("POST", "/api/token"),
    ("POST", "/api/revoke"),
    ("GET", "/api/me"),
];

/// Собирает роутер.
pub fn router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/register", post(register))
        .route("/token", post(token))
        .route("/revoke", post(revoke))
        .route("/me", get(me))
        .with_state(state.clone());

    // ⚠️ Неизвестный путь отдаёт `index.html`: страница проекта живёт по
    // адресу `/p/<id>`, и без этого перезагрузка на ней давала бы 404.
    let index = state.config.static_dir.join("index.html");
    let statics = ServeDir::new(&state.config.static_dir).fallback(ServeFile::new(index));

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .with_state(state.clone())
        .fallback_service(statics)
        .layer(axum::middleware::from_fn(cache_headers))
        .layer(axum::extract::DefaultBodyLimit::max(
            state.config.body_limit,
        ));

    // Префикс за обратным прокси. `/` — не префикс, и вкладывать в него ничего
    // не надо: `nest("/")` у axum запрещён.
    if state.config.base_path == "/" {
        app
    } else {
        Router::new().nest(&state.config.base_path, app)
    }
}

/// Проверка живости: база обязана отвечать.
///
/// ⚠️ `SELECT 1` через соединение, а не просто `200`: сервис, у которого база
/// недоступна, живым не является, и балансировщик обязан узнать об этом сам.
async fn health(State(state): State<Arc<AppState>>) -> Response {
    let ok = match state.pool.get().await {
        Ok(client) => client.query_one("SELECT 1", &[]).await.is_ok(),
        Err(_) => false,
    };
    if ok {
        (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "db"})),
        )
            .into_response()
    }
}

/// Запрос регистрации.
#[derive(Deserialize)]
struct RegisterRequest {
    login: String,
    password: String,
}

/// Запрос выдачи токена — форма OAuth 2.0.
#[derive(Deserialize)]
struct TokenRequest {
    grant_type: String,
    #[serde(default)]
    login: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    refresh_token: String,
}

/// Запрос гашения токена.
#[derive(Deserialize)]
struct RevokeRequest {
    refresh_token: String,
}

/// Выданная пара.
#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: u64,
}

/// Сведения о себе.
#[derive(Serialize)]
struct MeResponse {
    id: String,
    login: String,
    role: Role,
}

async fn register(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<RegisterRequest>,
) -> Result<Response, ApiError> {
    limit(&state, peer.ip())?;
    let client = state.pool.get().await?;
    let user = auth::register(&client, &request.login, &request.password, Role::User).await?;
    let pair = auth::start_session(
        &client,
        &state.config.jwt_secret,
        &user,
        state.config.access_ttl.as_secs() as i64,
        state.config.refresh_ttl.as_secs() as i64,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(pair_body(&state, pair))).into_response())
}

async fn token(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<TokenRequest>,
) -> Result<Response, ApiError> {
    let access_ttl = state.config.access_ttl.as_secs() as i64;
    let refresh_ttl = state.config.refresh_ttl.as_secs() as i64;
    let pair = match request.grant_type.as_str() {
        "password" => {
            // Частота ограничивается ТОЛЬКО у входа паролем: обмен refresh-
            // токена делает открытая вкладка сама, и окно било бы по своим.
            limit(&state, peer.ip())?;
            let client = state.pool.get().await?;
            let user = auth::authenticate(&client, &request.login, &request.password).await?;
            auth::start_session(
                &client,
                &state.config.jwt_secret,
                &user,
                access_ttl,
                refresh_ttl,
            )
            .await?
        }
        "refresh_token" => {
            let client = state.pool.get().await?;
            auth::refresh(
                &client,
                &state.config.jwt_secret,
                &request.refresh_token,
                access_ttl,
                refresh_ttl,
            )
            .await?
        }
        // Неизвестный `grant_type` отвечает тем же отказом, что неверный
        // пароль: перечислять поддерживаемые виды выдачи незачем.
        _ => return Err(ApiError::InvalidCredentials),
    };
    Ok(Json(pair_body(&state, pair)).into_response())
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RevokeRequest>,
) -> Result<Response, ApiError> {
    let client = state.pool.get().await?;
    auth::revoke(&client, &request.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn me(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Result<Response, ApiError> {
    let user = current_user(&state, &headers).await?;
    Ok(Json(MeResponse {
        id: user.id,
        login: user.login,
        role: user.role,
    })
    .into_response())
}

/// Опознаёт того, кто пришёл.
///
/// ⚠️ Роль читается **из базы**, а не из токена: иначе снятие права
/// администратора действовало бы только после истечения часа.
pub async fn current_user(state: &AppState, headers: &HeaderMap) -> Result<User, ApiError> {
    let header = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;
    let claims =
        auth::read_access(&state.config.jwt_secret, header).ok_or(ApiError::Unauthorized)?;
    let client = state.pool.get().await?;
    auth::load(&client, &claims.sub)
        .await?
        .ok_or(ApiError::Unauthorized)
}

fn pair_body(state: &AppState, pair: auth::Pair) -> TokenResponse {
    TokenResponse {
        access_token: pair.access,
        refresh_token: pair.refresh,
        token_type: "Bearer",
        expires_in: state.config.access_ttl.as_secs(),
    }
}

fn limit(state: &AppState, address: IpAddr) -> Result<(), ApiError> {
    match state.rate.check(address) {
        Decision::Allow => Ok(()),
        Decision::Wait(after_secs) => Err(ApiError::TooManyRequests { after_secs }),
    }
}

/// Заголовки кеша по форме адреса.
async fn cache_headers(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;
    let value = if is_fingerprinted(&path) {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    response
}

/// Помечен ли адрес отпечатком: `/b/<хеш>/…` либо `/wasm/<версия>/…`.
///
/// ⚠️ Отдельной функцией, потому что проверяется тестом: ошибись правило в одну
/// сторону — читатель год видит вчерашнюю страницу, в другую — стенд отдаёт
/// трёхмегабайтный модуль каждому заходу.
pub fn is_fingerprinted(path: &str) -> bool {
    let mut parts = path.trim_start_matches('/').split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("b"), Some(mark), Some(_)) => {
            mark.len() >= 6 && mark.chars().all(|c| c.is_ascii_hexdigit())
        }
        (Some("wasm"), Some(version), Some(_)) => {
            version.contains('.') && version.chars().all(|c| c.is_ascii_digit() || c == '.')
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fingerprinted_addresses_live_forever() {
        assert!(is_fingerprinted("/b/06baef9f2dab/app.js"));
        assert!(is_fingerprinted("/b/06baef9f2dab/font/x.woff2"));
        assert!(is_fingerprinted("/wasm/0.58.0/takt.wasm"));
        // Описи читают ради свежести — вечными им быть нельзя.
        assert!(!is_fingerprinted("/version.json"));
        assert!(!is_fingerprinted("/wasm/index.json"));
        assert!(!is_fingerprinted("/index.html"));
        assert!(!is_fingerprinted("/"));
        assert!(!is_fingerprinted("/b/短/app.js"), "не отпечаток");
        assert!(!is_fingerprinted("/b/06baef9f2dab"), "каталог без файла");
    }

    #[test]
    fn routes_are_listed_and_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for route in ROUTES {
            assert!(seen.insert(route), "маршрут {route:?} перечислен дважды");
            assert!(route.1.starts_with('/'), "путь без косой: {route:?}");
        }
        assert_eq!(seen.len(), 5, "остов 09a — пять маршрутов");
    }
}

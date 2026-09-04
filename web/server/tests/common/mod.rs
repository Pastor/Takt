//! Общий стенд проверок сервера (фича 0531, задачи 09a и 09b).
//!
//! ⚠️ Отдельным модулем, потому что наборов проверок два, а стенд им нужен
//! один: вторая копия разошлась бы с первой при первой же правке схемы.
//!
//! # Чем платим за PostgreSQL
//!
//! Решение заказчика 2026-09-04 заменило SQLite на PostgreSQL, и вместе с
//! базой в памяти ушла возможность проверять хранилище без сервера СУБД.
//! Отсюда политика, та же, что у прочих внешних инструментов проекта
//! (`iec2c`, `verilator`): **нет базы — мягкий пропуск, под
//! `PRECHECK_STRICT=1` — ошибка**. Решает это гейт `check-web-server.sh`; здесь
//! проверки просто не выполняются, и каждая говорит об этом словами.
//!
//! Строка подключения — `TAKT_WEB_TEST_DB`.
//!
//! # Изоляция
//!
//! Каждая проверка работает в **своей схеме** (`CREATE SCHEMA`), а не в своей
//! базе: схема создаётся мгновенно, а `search_path` уводит туда схему сервера
//! целиком. Соседние проверки друг друга не видят и идут параллельно.

#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as _;
use tower::ServiceExt as _;

use takt_web_server::config::Config;
use takt_web_server::db;
use takt_web_server::projects;
use takt_web_server::rate::Window;
use takt_web_server::routes::{self, AppState};

/// Живой стенд: роутер и его состояние.
pub struct Stand {
    pub app: axum::Router,
    pub schema: String,
    pub url: String,
}

/// Печатает причину пропуска: молча пропущенная проверка неотличима от
/// прошедшей.
pub fn skipped(what: &str) {
    eprintln!("пропуск ({what}): не задан TAKT_WEB_TEST_DB — базы нет");
}

impl Stand {
    /// Поднимает стенд в своей схеме; `None` — базы нет, проверка пропускается.
    pub async fn open(tag: &str) -> Option<Stand> {
        Self::open_with(tag, |_| {}).await
    }

    /// То же, но с правкой конфигурации: окно частоты, сроки токенов.
    pub async fn open_with(tag: &str, tune: impl FnOnce(&mut Config)) -> Option<Stand> {
        let url = std::env::var("TAKT_WEB_TEST_DB").ok()?;
        let schema = format!("t_{tag}");
        // Схема пересоздаётся: прогон, оборвавшийся посередине, не должен
        // мешать следующему.
        let admin = db::pool(&url).ok()?;
        let client = admin.get().await.ok()?;
        client
            .batch_execute(&format!(
                "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}"
            ))
            .await
            .ok()?;

        let scoped = scoped_url(&url, &schema);
        let pool = db::pool(&scoped).ok()?;
        let client = pool.get().await.ok()?;
        db::prepare(&client).await.ok()?;

        let mut config = Config::from_env().ok()?;
        config.database_url = scoped;
        config.jwt_secret = "секрет-проверки".to_string();
        // Окно частоты у проверок широкое: узкое било бы по своим, а его
        // предмет проверяется отдельно.
        config.rate_limit = 1000;
        tune(&mut config);
        let rate = Window::new(config.rate_window, config.rate_limit);
        let state = Arc::new(AppState {
            config,
            pool,
            rate,
            module_version: "0.58.0".to_string(),
            language_version: "0.17.0".to_string(),
        });
        Some(Stand {
            app: routes::router(state),
            schema,
            url,
        })
    }

    /// Строка подключения к схеме этого стенда.
    pub fn scoped(&self) -> String {
        scoped_url(&self.url, &self.schema)
    }

    /// Шлёт запрос от имени клиента с адресом.
    pub async fn call(&self, request: Request<Body>) -> (StatusCode, serde_json::Value) {
        let mut request = request;
        // Адрес нужен окну частоты: без него `ConnectInfo` не извлекается.
        request
            .extensions_mut()
            .insert(axum::extract::ConnectInfo(SocketAddr::from((
                [127, 0, 0, 1],
                40000,
            ))));
        let response = self.app.clone().oneshot(request).await.expect("ответ");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("тело")
            .to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, body)
    }

    pub async fn post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        self.call(json_request("POST", path, None, body)).await
    }

    pub async fn post_as(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        self.call(json_request("POST", path, Some(token), body))
            .await
    }

    pub async fn put_as(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        self.call(json_request("PUT", path, Some(token), body))
            .await
    }

    pub async fn patch_as(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        self.call(json_request("PATCH", path, Some(token), body))
            .await
    }

    pub async fn get_with(&self, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
        self.get_as(path, token).await
    }

    pub async fn get_as(&self, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
        self.call(
            Request::get(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("запрос"),
        )
        .await
    }

    pub async fn delete_as(&self, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
        self.call(
            Request::delete(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("запрос"),
        )
        .await
    }

    /// Набивает владельцу проекты напрямую в базу.
    ///
    /// ⚠️ Мимо ручки нарочно: сотня запросов ради проверки предела — это сотня
    /// запросов ни за чем, а предмет проверки в том, что предел СЧИТАЕТСЯ, а
    /// не в том, как проекты появились.
    pub async fn fill_projects(&self, login: &str, count: i64) {
        let pool = db::pool(&self.scoped()).expect("пул");
        let client = pool.get().await.expect("соединение");
        let owner: String = client
            .query_one(
                "SELECT id FROM users WHERE lower(login) = lower($1)",
                &[&login],
            )
            .await
            .expect("владелец")
            .get(0);
        for _ in 0..count {
            client
                .execute(
                    "INSERT INTO projects(id, owner_id, name, visibility, takt_lang,
                                          language_version, created_at, updated_at)
                     VALUES ($1, $2, 'набивка', 'private', '0.58.0', '0.17.0', 0, 0)",
                    &[&projects::new_id(), &owner],
                )
                .await
                .expect("проект");
        }
    }

    /// Убирает за собой: схема прогона не должна оставаться в базе.
    pub async fn drop_schema(&self) {
        if let Ok(pool) = db::pool(&self.url)
            && let Ok(client) = pool.get().await
        {
            let _ = client
                .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE", self.schema))
                .await;
        }
    }
}

fn scoped_url(url: &str, schema: &str) -> String {
    format!("{url}?options=-c%20search_path%3D{schema}")
}

fn json_request(
    method: &str,
    path: &str,
    token: Option<&str>,
    body: serde_json::Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder.body(Body::from(body.to_string())).expect("запрос")
}

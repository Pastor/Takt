//! Проверки сервера через HTTP (фича 0531, задача 09a).
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
//! базе: схема создаётся мгновенно, а `search_path` уводит туда всю схему
//! сервера целиком. Соседние проверки друг друга не видят и идут параллельно.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as _;
use tower::ServiceExt as _;

use takt_web_server::config::Config;
use takt_web_server::rate::Window;
use takt_web_server::routes::{self, AppState, ROUTES};
use takt_web_server::{auth, db};

/// Живой стенд: роутер и его состояние.
struct Stand {
    app: axum::Router,
    schema: String,
    url: String,
}

/// Поднимает стенд в своей схеме; `None` — базы нет, проверка пропускается.
async fn stand(tag: &str) -> Option<Stand> {
    let url = std::env::var("TAKT_WEB_TEST_DB").ok()?;
    let schema = format!("t_{tag}");
    // Схема пересоздаётся: прогон, оборвавшийся посередине, не должен мешать
    // следующему.
    let admin = db::pool(&url).ok()?;
    let client = admin.get().await.ok()?;
    client
        .batch_execute(&format!(
            "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}"
        ))
        .await
        .ok()?;

    let scoped = format!("{url}?options=-c%20search_path%3D{schema}");
    let pool = db::pool(&scoped).ok()?;
    let client = pool.get().await.ok()?;
    db::prepare(&client).await.ok()?;

    let mut config = Config::from_env().ok()?;
    config.database_url = scoped.clone();
    config.jwt_secret = "секрет-проверки".to_string();
    config.rate_limit = 1000;
    let rate = Window::new(config.rate_window, config.rate_limit);
    let state = Arc::new(AppState { config, pool, rate });
    Some(Stand {
        app: routes::router(state),
        schema,
        url,
    })
}

impl Stand {
    /// Шлёт запрос от имени клиента с адресом.
    async fn call(&self, request: Request<Body>) -> (StatusCode, serde_json::Value) {
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

    async fn post(&self, path: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
        self.call(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("запрос"),
        )
        .await
    }

    async fn get_with(&self, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
        self.call(
            Request::get(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .expect("запрос"),
        )
        .await
    }

    /// Убирает за собой: схема прогона не должна оставаться в базе.
    async fn drop_schema(&self) {
        if let Ok(pool) = db::pool(&self.url)
            && let Ok(client) = pool.get().await
        {
            let _ = client
                .batch_execute(&format!("DROP SCHEMA IF EXISTS {} CASCADE", self.schema))
                .await;
        }
    }
}

/// Печатает причину пропуска: молча пропущенная проверка неотличима от
/// прошедшей.
fn skipped(what: &str) {
    eprintln!("пропуск ({what}): не задан TAKT_WEB_TEST_DB — базы нет");
}

#[tokio::test]
async fn health_answers_only_when_the_database_answers() {
    let Some(stand) = stand("health").await else {
        return skipped("здоровье");
    };
    let (status, body) = stand
        .call(Request::get("/health").body(Body::empty()).expect("запрос"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    stand.drop_schema().await;
}

#[tokio::test]
async fn registration_gives_a_pair_and_the_login_is_taken_after() {
    let Some(stand) = stand("register").await else {
        return skipped("регистрация");
    };
    let (status, body) = stand
        .post(
            "/api/register",
            serde_json::json!({"login": "ivan", "password": "пароль-пароль"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(body["access_token"].is_string(), "{body}");
    assert!(body["refresh_token"].is_string(), "{body}");
    assert_eq!(body["token_type"], "Bearer");

    // ⚠️ Логин занят БЕЗ УЧЁТА РЕГИСТРА: иначе два владельца получили бы
    // неразличимые на глаз имена.
    let (status, body) = stand
        .post(
            "/api/register",
            serde_json::json!({"login": "IVAN", "password": "другой-пароль"}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"], "login_taken");
    stand.drop_schema().await;
}

#[tokio::test]
async fn bad_login_and_bad_password_answer_the_same() {
    // Разные ответы перечисляют заведённые логины.
    let Some(stand) = stand("oracle").await else {
        return skipped("оракул логинов");
    };
    stand
        .post(
            "/api/register",
            serde_json::json!({"login": "ivan", "password": "пароль-пароль"}),
        )
        .await;
    let (no_user, first) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "password", "login": "petr", "password": "пароль-пароль"}),
        )
        .await;
    let (bad_password, second) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "password", "login": "ivan", "password": "не-тот"}),
        )
        .await;
    assert_eq!(no_user, bad_password);
    assert_eq!(first, second, "ответы обязаны совпадать целиком");
    assert_eq!(first["error"], "invalid_grant");
    stand.drop_schema().await;
}

#[tokio::test]
async fn refresh_exchange_and_reuse_kills_the_family() {
    // ⚠️ Главное свойство цепочки: кража обнаруживается САМА. Владелец обменяет
    // свой токен, украденный предъявят вторым — и вход прекратится у обоих,
    // что заметно, в отличие от тихо работающей кражи.
    let Some(stand) = stand("refresh").await else {
        return skipped("обмен токенов");
    };
    let (_, registered) = stand
        .post(
            "/api/register",
            serde_json::json!({"login": "ivan", "password": "пароль-пароль"}),
        )
        .await;
    let first = registered["refresh_token"]
        .as_str()
        .expect("токен")
        .to_string();

    let (status, exchanged) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "refresh_token", "refresh_token": first}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{exchanged}");
    let second = exchanged["refresh_token"]
        .as_str()
        .expect("токен")
        .to_string();
    assert_ne!(second, first, "обмен обязан выдать новый токен");

    let (status, _) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "refresh_token", "refresh_token": first}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "одноразовый принят дважды");

    let (status, body) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "refresh_token", "refresh_token": second}),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "семейство обязано было погаснуть целиком: {body}"
    );
    stand.drop_schema().await;
}

#[tokio::test]
async fn revoke_says_the_same_about_a_stranger_token() {
    let Some(stand) = stand("revoke").await else {
        return skipped("гашение токена");
    };
    let (_, registered) = stand
        .post(
            "/api/register",
            serde_json::json!({"login": "ivan", "password": "пароль-пароль"}),
        )
        .await;
    let token = registered["refresh_token"]
        .as_str()
        .expect("токен")
        .to_string();

    let (stranger, _) = stand
        .post(
            "/api/revoke",
            serde_json::json!({"refresh_token": "нет-такого"}),
        )
        .await;
    let (own, _) = stand
        .post("/api/revoke", serde_json::json!({"refresh_token": token}))
        .await;
    assert_eq!(stranger, StatusCode::NO_CONTENT);
    assert_eq!(own, stranger, "ответ обязан быть одним: ручка не оракул");

    let (status, _) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "refresh_token", "refresh_token": token}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "погашенный токен принят");
    stand.drop_schema().await;
}

#[tokio::test]
async fn me_needs_a_token_and_reads_the_role_from_the_database() {
    // ⚠️ Роль читается ИЗ БАЗЫ, а не из токена: иначе снятие права
    // администратора действовало бы только после истечения часа.
    let Some(stand) = stand("me").await else {
        return skipped("сведения о себе");
    };
    let (_, registered) = stand
        .post(
            "/api/register",
            serde_json::json!({"login": "ivan", "password": "пароль-пароль"}),
        )
        .await;
    let access = registered["access_token"]
        .as_str()
        .expect("токен")
        .to_string();

    let (status, body) = stand.get_with("/api/me", &access).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["login"], "ivan");
    assert_eq!(body["role"], "user");

    let (status, _) = stand
        .call(Request::get("/api/me").body(Body::empty()).expect("запрос"))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "без токена");

    let (status, _) = stand.get_with("/api/me", "не-токен-вовсе").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "чужой токен");

    // Роль поднимают в базе — и тот же токен уже отвечает иначе.
    let pool = db::pool(&stand.url).expect("пул");
    let client = pool.get().await.expect("соединение");
    client
        .execute(
            &format!("UPDATE {}.users SET role = 'admin'", stand.schema),
            &[],
        )
        .await
        .expect("роль поднята");
    let (_, body) = stand.get_with("/api/me", &access).await;
    assert_eq!(
        body["role"], "admin",
        "роль читается из базы на каждый запрос"
    );
    stand.drop_schema().await;
}

#[tokio::test]
async fn rate_window_stops_the_flood_and_names_the_wait() {
    let Some(mut stand) = stand("rate").await else {
        return skipped("окно частоты");
    };
    // Стенд с узким окном: у прочих проверок оно широкое, чтобы не мешать.
    let url = stand.url.clone();
    let schema = stand.schema.clone();
    let scoped = format!("{url}?options=-c%20search_path%3D{schema}");
    let mut config = Config::from_env().expect("конфигурация");
    config.database_url = scoped.clone();
    config.jwt_secret = "секрет-проверки".to_string();
    config.rate_limit = 2;
    let pool = db::pool(&scoped).expect("пул");
    let rate = Window::new(config.rate_window, config.rate_limit);
    stand.app = routes::router(Arc::new(AppState { config, pool, rate }));

    for attempt in 1..=2 {
        let (status, _) = stand
            .post(
                "/api/token",
                serde_json::json!({"grant_type": "password", "login": "нет", "password": "нет-нет-нет"}),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "попытка {attempt} в окне");
    }
    let (status, body) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "password", "login": "нет", "password": "нет-нет-нет"}),
        )
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["error"], "too_many_requests");
    assert!(
        body["message"].as_str().expect("текст").contains("с"),
        "отказ обязан назвать, сколько ждать: {body}"
    );
    stand.drop_schema().await;
}

#[tokio::test]
async fn unknown_grant_type_answers_like_a_wrong_password() {
    let Some(stand) = stand("grant").await else {
        return skipped("вид выдачи");
    };
    let (status, body) = stand
        .post("/api/token", serde_json::json!({"grant_type": "магия"}))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["error"], "invalid_grant",
        "виды выдачи не перечисляются"
    );
    stand.drop_schema().await;
}

#[tokio::test]
async fn every_listed_route_answers_something() {
    // ⚠️ Список маршрутов объявлен в коде и проверяется здесь: маршрут,
    // заведённый мимо списка, появился бы у сервиса, не появившись в его
    // описании, — и о нём не узнал бы никто, кроме автора.
    let Some(stand) = stand("routes").await else {
        return skipped("список маршрутов");
    };
    for (method, path) in ROUTES {
        let request = Request::builder()
            .method(*method)
            .uri(*path)
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .expect("запрос");
        let (status, _) = stand.call(request).await;
        assert_ne!(
            status,
            StatusCode::NOT_FOUND,
            "{method} {path} перечислен, а сервис его не знает"
        );
    }
    // И обратное: путь мимо API уходит в статику, а не отвечает от неё.
    let (status, _) = stand
        .call(
            Request::get("/api/такого-нет")
                .body(Body::empty())
                .expect("запрос"),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "чужая ручка API");
    stand.drop_schema().await;
}

#[tokio::test]
async fn user_row_holds_nothing_personal() {
    // ⚠️ Предмет проверки — ОБЕЩАНИЕ, а не удобство: почты нет потому, что
    // восстановление пароля идёт сбросом администратора (решение заказчика
    // 2026-09-04), адреса нет потому, что этого требует A6. Колонка,
    // заведённая «на будущее», — персональные данные, которых никто не
    // собирался собирать.
    let Some(stand) = stand("schema").await else {
        return skipped("схема пользователей");
    };
    let pool = db::pool(&stand.url).expect("пул");
    let client = pool.get().await.expect("соединение");
    let rows = client
        .query(
            "SELECT column_name FROM information_schema.columns
             WHERE table_schema = $1 AND table_name = 'users' ORDER BY column_name",
            &[&stand.schema],
        )
        .await
        .expect("колонки");
    let have: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
    assert_eq!(
        have,
        vec!["created_at", "id", "login", "pass_hash", "role"],
        "состав колонок изменился"
    );
    for forbidden in [
        "email",
        "mail",
        "phone",
        "name",
        "ip",
        "address",
        "user_agent",
        "last_seen_at",
        "timezone",
    ] {
        assert!(
            !have.iter().any(|c| c == forbidden),
            "в схеме появилось '{forbidden}'"
        );
    }
    stand.drop_schema().await;
}

#[tokio::test]
async fn tokens_follow_the_user_they_belong_to() {
    // Связи в схеме есть, и каскад обязан работать: иначе токены удалённого
    // пользователя остались бы живыми ключами в базе.
    let Some(stand) = stand("cascade").await else {
        return skipped("каскадное удаление");
    };
    stand
        .post(
            "/api/register",
            serde_json::json!({"login": "ivan", "password": "пароль-пароль"}),
        )
        .await;
    let scoped = format!("{}?options=-c%20search_path%3D{}", stand.url, stand.schema);
    let pool = db::pool(&scoped).expect("пул");
    let client = pool.get().await.expect("соединение");
    let before: i64 = client
        .query_one("SELECT count(*) FROM refresh_tokens", &[])
        .await
        .expect("счёт")
        .get(0);
    assert_eq!(before, 1, "вход завёл токен");
    client
        .execute("DELETE FROM users", &[])
        .await
        .expect("удаление");
    let after: i64 = client
        .query_one("SELECT count(*) FROM refresh_tokens", &[])
        .await
        .expect("счёт")
        .get(0);
    assert_eq!(after, 0, "токены не ушли вслед за пользователем");
    stand.drop_schema().await;
}

#[tokio::test]
async fn foreign_schema_version_is_refused_by_name() {
    // Молчаливый переход между версиями схемы — это порча данных стенда.
    let Some(stand) = stand("version").await else {
        return skipped("версия схемы");
    };
    let scoped = format!("{}?options=-c%20search_path%3D{}", stand.url, stand.schema);
    let pool = db::pool(&scoped).expect("пул");
    let client = pool.get().await.expect("соединение");
    client
        .execute("UPDATE schema_version SET version = 99", &[])
        .await
        .expect("версия подменена");
    let error = db::prepare(&client).await.expect_err("чужая версия");
    let text = error.to_string();
    assert!(text.contains("99"), "{text}");
    assert!(text.contains(&db::SCHEMA_VERSION.to_string()), "{text}");
    stand.drop_schema().await;
}

#[tokio::test]
async fn password_change_kills_live_sessions() {
    let Some(stand) = stand("passwd").await else {
        return skipped("смена пароля");
    };
    let (_, registered) = stand
        .post(
            "/api/register",
            serde_json::json!({"login": "ivan", "password": "пароль-пароль"}),
        )
        .await;
    let token = registered["refresh_token"]
        .as_str()
        .expect("токен")
        .to_string();

    let scoped = format!("{}?options=-c%20search_path%3D{}", stand.url, stand.schema);
    let pool = db::pool(&scoped).expect("пул");
    let client = pool.get().await.expect("соединение");
    auth::set_password(&client, "ivan", "новый-пароль")
        .await
        .expect("смена");

    let (status, _) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "refresh_token", "refresh_token": token}),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "смена пароля обязана выгонять того, ради кого её делают"
    );
    let (status, _) = stand
        .post(
            "/api/token",
            serde_json::json!({"grant_type": "password", "login": "ivan", "password": "новый-пароль"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "новый пароль работает");
    stand.drop_schema().await;
}

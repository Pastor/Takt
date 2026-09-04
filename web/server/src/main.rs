//! Запуск сервера и команды администратора (фича 0531, задача 09a).
//!
//! Сам сервер живёт библиотекой (`lib.rs`): интеграционные тесты поднимают его
//! роутер, а до модулей бинарника дотянуться не могут.
//!
//! # Команды
//!
//! ```text
//! takt-web-server                           запустить сервер
//! takt-web-server admin <логин> <пароль>    завести администратора
//! takt-web-server passwd <логин> <пароль>   сменить пароль
//! ```
//!
//! Первый администратор и сброс пароля — командами: почта не хранится, и
//! восстановления по письму нет (решение заказчика 2026-09-04).

use std::net::SocketAddr;
use std::sync::Arc;

use takt_web_server::auth::{self, Role};
use takt_web_server::config::Config;
use takt_web_server::db;
use takt_web_server::rate::Window;
use takt_web_server::routes::{self, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("TAKT_WEB_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env()?;
    let pool = db::pool(&config.database_url)?;
    {
        let client = pool.get().await?;
        db::prepare(&client).await?;
    }

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("admin") => return command_admin(&pool, &arguments).await,
        Some("passwd") => return command_passwd(&pool, &arguments).await,
        Some(unknown) => anyhow::bail!(
            "неизвестная команда '{unknown}'. Известны: admin, passwd (без команды — запуск)"
        ),
        None => {}
    }

    if config.uses_dev_secret() {
        // ⚠️ Общеизвестный секрет подписи означает, что токен может подписать
        // кто угодно. Молчать об этом нельзя — но и падать нельзя: на своей
        // машине умолчание и есть удобство.
        tracing::warn!(
            "TAKT_WEB_JWT_SECRET не задан — работает умолчание для своей машины; \
             на стенде это означает, что access-токен может подписать кто угодно"
        );
    }

    let listen = config.listen;
    let rate = Window::new(config.rate_window, config.rate_limit);
    // Версии — из описи собранной статики: второго носителя у них нет, и
    // сервер не должен знать их числом (класс 0084).
    let (module_version, language_version) = versions(&config.static_dir);
    let state = Arc::new(AppState {
        config,
        pool,
        rate,
        module_version,
        language_version,
    });

    let listener = tokio::net::TcpListener::bind(listen).await?;
    tracing::info!(%listen, "сервер слушает");
    axum::serve(
        listener,
        routes::router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}

async fn command_admin(pool: &deadpool_postgres::Pool, arguments: &[String]) -> anyhow::Result<()> {
    let (Some(login), Some(password)) = (arguments.get(1), arguments.get(2)) else {
        anyhow::bail!("takt-web-server admin <логин> <пароль>");
    };
    let client = pool.get().await?;
    auth::register(&client, login, password, Role::Admin)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("администратор '{login}' заведён");
    Ok(())
}

async fn command_passwd(
    pool: &deadpool_postgres::Pool,
    arguments: &[String],
) -> anyhow::Result<()> {
    let (Some(login), Some(password)) = (arguments.get(1), arguments.get(2)) else {
        anyhow::bail!("takt-web-server passwd <логин> <пароль>");
    };
    let client = pool.get().await?;
    auth::set_password(&client, login, password)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("пароль '{login}' сменён; живые сеансы погашены");
    Ok(())
}

/// Читает версии из `version.json` собранной статики.
///
/// ⚠️ Умолчание — пустые строки, а не выдуманные числа: сервер, запущенный без
/// собранной статики, обязан это показывать, а не сообщать проекту версию,
/// модуля которой на диске нет.
fn versions(static_dir: &std::path::Path) -> (String, String) {
    let Ok(text) = std::fs::read_to_string(static_dir.join("version.json")) else {
        tracing::warn!("нет version.json в статике — новый проект не получит версию модуля");
        return (String::new(), String::new());
    };
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    (
        parsed["takt_lang"].as_str().unwrap_or_default().to_string(),
        parsed["language"].as_str().unwrap_or_default().to_string(),
    )
}

/// Останов по сигналу: соединения дорабатываются, а не рвутся.
async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("останов");
}

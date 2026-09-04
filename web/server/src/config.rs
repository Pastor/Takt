//! Конфигурация сервера из окружения (фича 0531, задача 09a).
//!
//! # Правила
//!
//! Один префикс — `TAKT_WEB_`; у каждого ключа умолчание, годное для запуска
//! на своей машине без единой переменной. Разбор числа, который не удался, —
//! **отказ с названным ключом и значением**, а не молчаливое умолчание:
//! опечатка в переменной иначе даёт стенд, работающий не так, как написано в
//! конфигурации.
//!
//! ⚠️ `TAKT_WEB_BASE_PATH` — префикс за обратным прокси. Он есть потому, что у
//! референса из его отсутствия родилась поломка: клиент ходил в API
//! **абсолютными** путями, и за прокси `/tamagotchi/` ссылки уводили в корень.
//! Здесь страница строит адреса относительно себя, а сервер знает префикс —
//! обе половины, и ни одна не полагается на другую.

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

/// Настройки запуска.
#[derive(Debug, Clone)]
pub struct Config {
    /// Адрес и порт прослушивания.
    pub listen: SocketAddr,
    /// Строка подключения к PostgreSQL.
    pub database_url: String,
    /// Каталог собранной статики (`web/dist`).
    pub static_dir: PathBuf,
    /// Префикс за обратным прокси: `/` либо `/takt`.
    pub base_path: String,
    /// Секрет подписи access-токенов.
    pub jwt_secret: String,
    /// Срок жизни access-токена.
    pub access_ttl: Duration,
    /// Срок жизни refresh-токена.
    pub refresh_ttl: Duration,
    /// Предел тела запроса в байтах.
    pub body_limit: usize,
    /// Окно ограничения частоты.
    pub rate_window: Duration,
    /// Сколько попыток входа и регистраций допускается в окне с одного адреса.
    pub rate_limit: u32,
}

/// Отказ разбора конфигурации: ключ назван, значение показано.
#[derive(Debug, thiserror::Error)]
#[error("переменная {key}: значение '{value}' не разбирается ({what})")]
pub struct ConfigError {
    key: &'static str,
    value: String,
    what: &'static str,
}

/// Секрет по умолчанию. Заметный нарочно: он обязан бросаться в глаза и в
/// конфигурации, и в предупреждении при запуске.
pub const DEV_SECRET: &str = "секрет-для-своей-машины-не-для-стенда";

/// Подключение по умолчанию — своя машина.
///
/// ⚠️ Хранилище — **PostgreSQL** (решение заказчика 2026-09-04, отменяет
/// SQLite из проработки). Умолчание годится для запуска на своей машине; на
/// стенде строка задаётся переменной, и пароля в дереве нет.
pub const DEV_DATABASE_URL: &str = "postgresql://localhost/takt_web";

impl Config {
    /// Читает конфигурацию из окружения.
    ///
    /// # Ошибки
    /// [`ConfigError`], если значение переменной не разбирается.
    pub fn from_env() -> Result<Self, ConfigError> {
        let host: IpAddr = parse("TAKT_WEB_HOST", "127.0.0.1", "адрес")?;
        let port: u16 = parse("TAKT_WEB_PORT", "8730", "порт")?;
        Ok(Self {
            listen: SocketAddr::new(host, port),
            database_url: var("TAKT_WEB_DB", DEV_DATABASE_URL),
            static_dir: var("TAKT_WEB_STATIC", "web/dist").into(),
            base_path: normalize_base(&var("TAKT_WEB_BASE_PATH", "/")),
            // ⚠️ Умолчание секрета годится ТОЛЬКО для своей машины, и сервер
            // говорит об этом при запуске: молчаливый общеизвестный секрет на
            // стенде означает, что подписать токен может кто угодно.
            jwt_secret: var("TAKT_WEB_JWT_SECRET", DEV_SECRET),
            access_ttl: Duration::from_secs(parse("TAKT_WEB_ACCESS_TTL", "3600", "секунды")?),
            refresh_ttl: Duration::from_secs(parse("TAKT_WEB_REFRESH_TTL", "2592000", "секунды")?),
            body_limit: parse("TAKT_WEB_BODY_LIMIT", "1048576", "байты")?,
            rate_window: Duration::from_secs(parse("TAKT_WEB_RATE_WINDOW", "60", "секунды")?),
            rate_limit: parse("TAKT_WEB_RATE_LIMIT", "10", "число")?,
        })
    }

    /// Секрет остался умолчанием — стенду это не годится.
    pub fn uses_dev_secret(&self) -> bool {
        self.jwt_secret == DEV_SECRET
    }
}

fn var(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn parse<T: std::str::FromStr>(
    key: &'static str,
    default: &str,
    what: &'static str,
) -> Result<T, ConfigError> {
    let value = var(key, default);
    value.parse().map_err(|_| ConfigError { key, value, what })
}

/// Приводит префикс к виду `/` либо `/имя` — без хвостовой косой черты.
///
/// ⚠️ Хвостовая косая — источник двойных `//` в собранных адресах: один и тот
/// же ресурс становится двумя разными путями, и правило кеша по форме адреса
/// перестаёт работать на одном из них.
pub fn normalize_base(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_path_is_normalized() {
        assert_eq!(normalize_base("/"), "/");
        assert_eq!(normalize_base(""), "/");
        assert_eq!(normalize_base("   "), "/");
        assert_eq!(normalize_base("/takt/"), "/takt");
        assert_eq!(normalize_base("takt"), "/takt");
        assert_eq!(normalize_base("/takt///"), "/takt");
    }

    #[test]
    fn defaults_start_without_a_single_variable() {
        let config = Config::from_env().expect("умолчания годны");
        assert_eq!(config.base_path, "/");
        assert!(config.uses_dev_secret(), "секрет по умолчанию опознаётся");
        assert_eq!(
            config.access_ttl.as_secs(),
            3600,
            "час — решение проработки"
        );
    }
}

//! Единый тип ошибок API (фича 0531, задача 09a; приём референса).
//!
//! # Зачем один тип
//!
//! Через границу HTTP едет JSON, и разобрать его на той стороне можно только
//! по заранее известной форме. Придумай каждая ручка свою — страница разбирала
//! бы десяток форм, и первая же незамеченная разница («ошибка в поле `error`»
//! против «в поле `message`») кончилась бы тем, что отказ показался успехом.
//!
//! Форма одна: `{"error": "код", "message": "текст"}`. Код — для машины
//! (страница по нему решает, что показать), текст — для человека.
//!
//! ⚠️ Ручка не должна быть **оракулом**: чужой закрытый проект отвечает `404`,
//! а не `403`, и `revoke` неизвестного токена отвечает «готово», а не «нет
//! такого». Иначе по ответам перечисляется то, чего спрашивающий не видел.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Ошибка уровня API.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Токен отсутствует, просрочен или недействителен.
    #[error("не авторизован")]
    Unauthorized,
    /// Роль не даёт доступа к этому ресурсу.
    #[error("недостаточно прав")]
    Forbidden,
    /// Ресурса нет — либо он есть, но спрашивающему его не видно.
    #[error("не найдено")]
    NotFound,
    /// Логин уже занят.
    #[error("логин уже занят")]
    LoginTaken,
    /// Неверная пара логин/пароль либо неподдерживаемый `grant_type`.
    #[error("неверные учётные данные")]
    InvalidCredentials,
    /// Слишком часто: окно ограничения исчерпано.
    #[error("слишком много попыток, повторите через {after_secs} с")]
    TooManyRequests {
        /// Через сколько секунд окно освободится.
        after_secs: u64,
    },
    /// Предел хранилища превышен: названы и предел, и факт.
    #[error("{message}")]
    LimitExceeded {
        /// Что превышено, сколько можно и сколько получено.
        message: String,
    },
    /// Запись устарела: у ресурса другая ревизия.
    #[error("{0}")]
    Conflict(String),
    /// Запрос не годится: причина названа.
    #[error("{0}")]
    BadRequest(String),
    /// Внутренняя ошибка. Подробности — в журнале, наружу они не едут.
    #[error("внутренняя ошибка")]
    Internal(#[from] anyhow::Error),
}

/// Тело ответа об ошибке.
#[derive(Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
    message: String,
}

impl ApiError {
    /// Код HTTP и машинный код ошибки.
    pub fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::LoginTaken => (StatusCode::CONFLICT, "login_taken"),
            // 400, как в OAuth 2.0: `invalid_grant` — это отказ выдачи токена,
            // а не «вы не авторизованы» (401 просил бы предъявить токен там,
            // где его как раз и получают).
            Self::InvalidCredentials => (StatusCode::BAD_REQUEST, "invalid_grant"),
            Self::TooManyRequests { .. } => (StatusCode::TOO_MANY_REQUESTS, "too_many_requests"),
            // 413 у ВСЕХ пределов, включая число файлов и число проектов:
            // клиент по одному коду показывает одно — «столько нельзя», — а
            // текст называет, чего именно и сколько.
            Self::LimitExceeded { .. } => (StatusCode::PAYLOAD_TOO_LARGE, "limit_exceeded"),
            // 409, а не 412: ревизию клиент шлёт в теле, а не заголовком
            // `If-Match`, и предусловия HTTP здесь нет — есть расхождение
            // состояний, о котором автору предстоит решить.
            Self::Conflict(_) => (StatusCode::CONFLICT, "revision_conflict"),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }
}

impl From<tokio_postgres::Error> for ApiError {
    fn from(error: tokio_postgres::Error) -> Self {
        Self::Internal(anyhow::Error::new(error))
    }
}

impl From<deadpool_postgres::PoolError> for ApiError {
    fn from(error: deadpool_postgres::PoolError) -> Self {
        Self::Internal(anyhow::Error::new(error))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        // Внутренняя ошибка пишется в журнал ЦЕЛИКОМ, а наружу уходит одним
        // словом: подробности внутренней ошибки — это подсказка нападающему.
        if let Self::Internal(ref cause) = self {
            tracing::error!(error = %cause, "внутренняя ошибка");
        }
        let body = ErrorBody {
            error: code,
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_has_its_own_code() {
        // Два разных отказа с одним кодом означают, что страница не сможет их
        // различить, а тексты она показывать обязана по-разному.
        let errors = [
            ApiError::Unauthorized,
            ApiError::Forbidden,
            ApiError::NotFound,
            ApiError::LoginTaken,
            ApiError::InvalidCredentials,
            ApiError::TooManyRequests { after_secs: 1 },
            ApiError::LimitExceeded {
                message: "предел 1, получено 2".into(),
            },
            ApiError::Conflict("ревизия 1, у ресурса 2".into()),
            ApiError::BadRequest("причина".into()),
            ApiError::Internal(anyhow::anyhow!("причина")),
        ];
        let mut seen = std::collections::BTreeSet::new();
        for error in &errors {
            let (_, code) = error.status_and_code();
            assert!(seen.insert(code), "код '{code}' встречается дважды");
            assert!(!error.to_string().is_empty(), "отказ без текста: {code}");
        }
    }

    #[test]
    fn internal_error_says_nothing_outward() {
        // Подробность внутренней ошибки — подсказка нападающему: наружу едет
        // одно слово, целиком она уходит в журнал.
        let error = ApiError::Internal(anyhow::anyhow!("путь /etc/secret не читается"));
        assert_eq!(error.to_string(), "внутренняя ошибка");
    }
}

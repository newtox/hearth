use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Bad(String),
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict(String),
    TooMany,
    Unavailable(String),
    Internal(String),
}

pub fn bad<T>(msg: &str) -> Result<T> {
    Err(AppError::Bad(msg.to_string()))
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::Bad(m) => (StatusCode::BAD_REQUEST, m),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Not signed in".into()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "You do not have permission to do that".into()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found".into()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m),
            AppError::TooMany => (StatusCode::TOO_MANY_REQUESTS, "Too many requests, wait a moment".into()),
            AppError::Unavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m),
            AppError::Internal(m) => {
                tracing::error!("{m}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::RowNotFound => AppError::NotFound,
            sqlx::Error::Database(d) if d.is_unique_violation() => AppError::Conflict("Already taken".into()),
            _ => AppError::Internal(e.to_string()),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

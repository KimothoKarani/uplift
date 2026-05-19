use sqlx;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    /// 400 - caller sent bad data
    BadRequest(String),
    /// 401 - not logged in
    Unauthorized,
    /// 403 - logged in but not allowed
    Forbidden,
    /// 404 - resource doesn't exist
    NotFound,
    /// 409 - conflict (e.g. duplicate property)
    Conflict(String),
    /// 500 - our fault
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".into()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden".into()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found".into()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::Internal(e) => {
                tracing::error!(error = %e, "internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".into())
            }
        };

        (status, Json(json!({"error": message}))).into_response()
    }
}

// Let ? work on anyhow::Error inside route handlers
impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        AppError::Internal(value)
    }
}

// Let ? work on sqlx errors directly
impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::RowNotFound => AppError::NotFound,
            other => AppError::Internal(anyhow::anyhow!(other))
        }
    }
}

// Let ? work on uplift_db errors
impl From<uplift_db::Error> for AppError {
    fn from(value: uplift_db::Error) -> Self {
        match value {
            uplift_db::Error::NotFound => AppError::NotFound,
            other => AppError::Internal(anyhow::anyhow!(other)),
        }
    }
}
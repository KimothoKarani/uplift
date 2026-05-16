use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("not found")]
    NotFound,

    #[error("crypto: {0}")]
    Crypto(String),

    #[error("serialization: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

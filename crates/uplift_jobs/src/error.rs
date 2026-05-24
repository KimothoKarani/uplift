use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] uplift_db::Error),

    #[error("core model error: {0}")]
    Core(#[from] uplift_core::Error),

    #[error("connector error: {0}")]
    Connector(#[from] uplift_connectors::Error),

    #[error("record not found")]
    NotFound,

    #[error("analysis {id} is in unexpected status '{status}'")]
    BadStatus { id: uuid::Uuid, status: String },

    #[error("no time series data available for property {property_id}, metric '{metric}'")]
    NoData {
        property_id: uuid::Uuid,
        metric: String,
    },

    #[error("token refresh failed for connection {connection_id}: {reason}")]
    TokenRefresh {
        connection_id: uuid::Uuid,
        reason: String,
    },

    #[error("email send failed: {0}")]
    Email(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

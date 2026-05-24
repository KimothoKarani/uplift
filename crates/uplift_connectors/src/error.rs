use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("OAuth error: {0}")]
    Auth(String),

    #[error("Google API error {status}: {message}")]
    Api { status: u16, message: String },

    #[error("failed to parse API response: {0}")]
    Parse(String),

    #[error("access token expired")]
    TokenExpired,

    #[error("no data returned for the requested data range")]
    NoData,
}

pub type Result<T> = std::result::Result<T, Error>;

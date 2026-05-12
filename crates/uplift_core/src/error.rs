use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("insufficient data: need at least {min} points, got {got}")]
    InsufficientData { min: usize, got: usize },

    #[error("invalid intervention date: {0}")]
    InvalidInterventionDate(String),

    #[error("decomposition failed: {0}")]
    DecompositionFailed(String),

    #[error("model fit failed: {0}")]
    ModelFitFailed(String),

    #[error("numerical error: {0}")]
    NumericalError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

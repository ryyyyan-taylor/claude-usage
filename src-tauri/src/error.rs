use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Claude credentials not found at {0}")]
    CredentialsNotFound(String),

    #[error("Claude CLI not found in PATH")]
    CliNotFound,

    #[error("Authentication required - credentials invalid or expired")]
    AuthRequired,

    #[error("Rate limited by API")]
    RateLimited,

    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::error::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;

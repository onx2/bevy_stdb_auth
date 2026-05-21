use thiserror::Error;

/// An authentication lifecycle error.
#[derive(Debug, Error)]
pub enum StdbAuthError {
    /// An HTTP request failed.
    #[error("auth HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    /// A JSON payload failed to decode.
    #[error("auth response decode failed: {0}")]
    Decode(#[from] serde_json::Error),
    /// The requested operation timed out.
    #[error("auth operation timed out")]
    Timeout,
    /// The requested operation is not supported by the current auth source.
    #[error("unsupported auth operation: {0}")]
    Unsupported(String),
    /// An internal authentication operation failed.
    #[error("auth operation failed: {0}")]
    Internal(String),
}

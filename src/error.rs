use std::fmt::Display;

/// An authentication lifecycle error.
#[derive(Clone, Debug)]
pub enum StdbAuthError {
    /// The requested operation is not supported by the current auth source.
    Unsupported(String),
    /// An internal authentication operation failed.
    Internal(String),
}

impl Display for StdbAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(message) => write!(f, "unsupported auth operation: {message}"),
            Self::Internal(message) => write!(f, "auth operation failed: {message}"),
        }
    }
}

impl std::error::Error for StdbAuthError {}

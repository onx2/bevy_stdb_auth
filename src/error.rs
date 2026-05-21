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
    /// The authentication configuration is invalid.
    #[error("invalid auth configuration: {0}")]
    InvalidConfig(String),
    /// A token endpoint response is invalid.
    #[error("invalid token response: {0}")]
    InvalidTokenResponse(String),
    /// An OIDC callback URL is invalid.
    #[error("invalid OIDC callback: {0}")]
    InvalidOidcCallback(String),
    /// The authentication provider returned an error.
    #[error("auth provider error: {0}")]
    Provider(String),
    /// The requested operation is not supported by the current auth source.
    #[error("unsupported auth operation: {0}")]
    Unsupported(String),
    /// An internal authentication operation failed.
    #[error("auth operation failed: {0}")]
    Internal(String),
}

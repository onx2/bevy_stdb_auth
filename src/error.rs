use thiserror::Error;

/// An error returned when an authentication command cannot be accepted.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum StdbAuthCommandError {
    /// Another authentication operation is already pending.
    #[error("another authentication operation is already pending")]
    PendingOperation,
    /// No authentication session is active.
    #[error("no authentication session is active")]
    NoSession,
    /// No authentication operation is pending.
    #[error("no authentication operation is pending")]
    NoPendingOperation,
    /// The active session cannot be refreshed.
    #[error("the active authentication session cannot be refreshed")]
    MissingRefreshToken,
    /// The active session does not include a client ID.
    #[error("the active authentication session does not include a client ID")]
    MissingClientId,
    /// The command is not supported by the active authentication source.
    #[error("unsupported authentication command: {0}")]
    Unsupported(String),
}

/// An authentication lifecycle error.
#[derive(Debug, Error)]
pub enum StdbAuthError {
    /// An authentication command could not be accepted.
    #[error("auth command rejected: {0}")]
    Command(#[from] StdbAuthCommandError),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_error_converts_to_auth_error() {
        let error = StdbAuthError::from(StdbAuthCommandError::NoSession);

        assert!(matches!(
            error,
            StdbAuthError::Command(StdbAuthCommandError::NoSession)
        ));
    }
}

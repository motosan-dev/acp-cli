use thiserror::Error;

#[derive(Debug, Error)]
pub enum AcpCliError {
    #[error("agent error: {0}")]
    Agent(String),

    #[error("usage error: {0}")]
    Usage(String),

    #[error("timeout after {0}s")]
    Timeout(u64),

    #[error("no session found for {agent} in {cwd}")]
    NoSession { agent: String, cwd: String },

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("interrupted")]
    Interrupted,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("acp connection failed: {0}")]
    Connection(String),
}

impl AcpCliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Agent(_) | Self::Io(_) | Self::Connection(_) => 1,
            Self::Usage(_) => 2,
            Self::Timeout(_) => 3,
            Self::NoSession { .. } => 4,
            Self::PermissionDenied(_) => 5,
            Self::Interrupted => 130,
        }
    }
}

/// Returns `true` for errors that are safe to retry (network/connection failures
/// before any agent output was produced).
///
/// Non-retriable errors include semantic failures (permission denied, session not
/// found, auth errors) and user-initiated interrupts.
pub fn is_transient(err: &AcpCliError) -> bool {
    matches!(err, AcpCliError::Connection(_))
}

pub type Result<T> = std::result::Result<T, AcpCliError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_error_is_transient() {
        assert!(is_transient(&AcpCliError::Connection("refused".into())));
    }

    #[test]
    fn agent_error_is_not_transient() {
        assert!(!is_transient(&AcpCliError::Agent(
            "permission denied".into()
        )));
    }

    #[test]
    fn timeout_is_not_transient() {
        // Timeout may indicate side effects already occurred; not retried by default.
        assert!(!is_transient(&AcpCliError::Timeout(30)));
    }

    #[test]
    fn interrupted_is_not_transient() {
        assert!(!is_transient(&AcpCliError::Interrupted));
    }

    #[test]
    fn io_error_is_not_transient() {
        assert!(!is_transient(&AcpCliError::Io(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "broken pipe"
        ))));
    }
}

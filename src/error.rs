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

pub type Result<T> = std::result::Result<T, AcpCliError>;

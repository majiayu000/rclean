use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("cannot scan {path}: {source}")]
    CanonicalizeRoot {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Generic(String),
}

impl From<String> for ScanError {
    fn from(s: String) -> Self {
        Self::Generic(s)
    }
}

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("plan io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("plan parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error(
        "unsupported action plan schema version {found}; this build supports {supported}. Re-run `rclean scan --write-plan <path>` to regenerate the plan"
    )]
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    #[error("{0}")]
    Generic(String),
}

impl From<String> for PlanError {
    fn from(s: String) -> Self {
        Self::Generic(s)
    }
}

#[derive(Debug, Error)]
pub enum CleanError {
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error("{0}")]
    Generic(String),
}

impl From<String> for CleanError {
    fn from(s: String) -> Self {
        Self::Generic(s)
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid size: {0}")]
    InvalidSize(String),
    #[error("invalid duration: {0}")]
    InvalidDuration(String),
    #[error("{0}")]
    Generic(String),
}

impl From<String> for ParseError {
    fn from(s: String) -> Self {
        Self::Generic(s)
    }
}

#[derive(Debug, Error)]
pub enum RcleanError {
    #[error(transparent)]
    Agent(#[from] crate::agent::AgentError),
    #[error(transparent)]
    Scan(#[from] ScanError),
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error(transparent)]
    Clean(#[from] CleanError),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("output serialization error: {0}")]
    Output(#[from] serde_json::Error),
    #[error("output io error: {0}")]
    OutputIo(#[from] std::io::Error),
    #[cfg(feature = "graveyard")]
    #[error(transparent)]
    Graveyard(#[from] crate::graveyard::GraveyardError),
}

impl RcleanError {
    pub(crate) fn output_io_kind(&self) -> Option<std::io::ErrorKind> {
        match self {
            Self::OutputIo(error) => Some(error.kind()),
            _ => None,
        }
    }
}

impl From<String> for RcleanError {
    fn from(s: String) -> Self {
        Self::Scan(ScanError::Generic(s))
    }
}

#[derive(Debug, Error)]
pub enum UserRuleError {
    #[error("rule id must not be empty")]
    EmptyId,
    #[error("rule '{id}': {message}")]
    InvalidCategory { id: String, message: String },
    #[error("rule '{id}': safety=caution requires at least one parent_markers entry")]
    CautionRequiresParentMarkers { id: String },
    #[error("rule '{id}': {source}")]
    InvalidGlob {
        id: String,
        #[source]
        source: globset::Error,
    },
    #[error(
        "rule '{id}': safety=blocked is not allowed for user rules (only builtin rules may produce blocked)"
    )]
    BlockedSafety { id: String },
    #[error("rule '{id}': invalid safety '{safety}' (use 'safe' or 'caution')")]
    InvalidSafety { id: String, safety: String },
}

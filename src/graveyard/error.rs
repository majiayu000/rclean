use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraveyardError {
    #[error("graveyard io at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("graveyard manifest parse: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("graveyard manifest lock contention after {attempts} retries")]
    ManifestLockContention { attempts: u32 },

    #[error("graveyard schema version {found} not supported (this build understands {supported})")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },

    #[error("graveyard restore target {path} already exists; refuse to overwrite")]
    RestoreTargetExists { path: PathBuf },

    #[error("graveyard restore target parent {path} is a symlink; refuse to traverse")]
    RestoreTargetParentIsSymlink { path: PathBuf },

    #[error("graveyard record id {0} not found")]
    GraveNotFound(String),

    #[error("{0}")]
    Generic(String),
}

impl From<String> for GraveyardError {
    fn from(s: String) -> Self {
        Self::Generic(s)
    }
}

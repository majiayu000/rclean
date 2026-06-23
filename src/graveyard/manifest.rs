use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::GraveyardError;

/// Stringly-typed grave id (see `super::id::generate`). Kept as a
/// type alias so a future swap to a real ULID crate is a one-line
/// change here.
pub type GraveId = String;

/// Schema version stored in every manifest record. Reader rejects
/// records with a different version so an out-of-date binary can't
/// silently misinterpret a newer grave's metadata.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// On-disk record (one per line in `manifest.jsonl`). See
/// `docs/specs/v0.1.x-roadmap.md` §4.7.2 for the full field
/// semantics and rationale.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestRecord {
    pub schema_version: u32,
    pub id: GraveId,
    pub deleted_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Original (non-canonicalized) path. Preserves the user's symlink
    /// intent so restore puts the directory back where they last saw it.
    pub original_path: PathBuf,
    pub size_bytes: u64,
    pub plan_id: Option<GraveId>,
    pub rule_id: String,
    pub category: String,
    pub safety_at_delete: String,
    pub risk_score_at_delete: f32,
    pub tool_version: String,
    /// Path relative to the graveyard root, e.g.
    /// `2026/05/16/14h23m07-01HXG..`.
    pub grave_path: PathBuf,
}

/// Iterator over records in a `manifest.jsonl` file. Skips malformed
/// lines with a warning rather than aborting — a partial-write at the
/// end of the file (e.g. SIGKILL mid-append) shouldn't lose the rest.
pub struct ManifestReader {
    inner: BufReader<File>,
}

impl ManifestReader {
    pub fn open(path: &Path) -> Result<Self, GraveyardError> {
        let file = File::open(path).map_err(|source| GraveyardError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            inner: BufReader::new(file),
        })
    }

    /// Reads every record, skipping (with a tracing warn) any line that
    /// fails to parse. Returns records in file order.
    pub fn read_all(self) -> Result<Vec<ManifestRecord>, GraveyardError> {
        let mut out = Vec::new();
        for (line_number, line) in self.inner.lines().enumerate() {
            let line = line.map_err(|source| GraveyardError::Io {
                path: PathBuf::from("<manifest>"),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ManifestRecord>(&line) {
                Ok(rec) if rec.schema_version != MANIFEST_SCHEMA_VERSION => {
                    return Err(GraveyardError::UnsupportedSchemaVersion {
                        found: rec.schema_version,
                        supported: MANIFEST_SCHEMA_VERSION,
                    });
                }
                Ok(rec) => out.push(rec),
                Err(err) => {
                    tracing::warn!(
                        line = line_number + 1,
                        error = %err,
                        "skipping malformed manifest line"
                    );
                }
            }
        }
        Ok(out)
    }
}

/// Append-only writer with an advisory cross-platform lock. Held only
/// for the duration of the append; restore / list don't need it.
pub struct RecordWriter {
    manifest_path: PathBuf,
    lock_path: PathBuf,
}

impl RecordWriter {
    pub fn new(graveyard_root: &Path) -> Self {
        Self {
            manifest_path: graveyard_root.join("manifest.jsonl"),
            lock_path: graveyard_root.join("manifest.jsonl.lock"),
        }
    }

    /// Acquire the advisory lock, rewrite the manifest with the new record,
    /// release the lock.
    /// Bounded retry (5 attempts × 100 ms) so a stuck lock surfaces
    /// as `ManifestLockContention` instead of hanging.
    pub fn append(&self, record: &ManifestRecord) -> Result<(), GraveyardError> {
        let mut attempts: u32 = 0;
        let _lock = loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.lock_path)
            {
                Ok(file) => break ScopedLock::new(file, self.lock_path.clone()),
                Err(_) if attempts < 5 => {
                    attempts += 1;
                    sleep(Duration::from_millis(100));
                }
                Err(_) => {
                    return Err(GraveyardError::ManifestLockContention { attempts });
                }
            }
        };

        let existing = match fs::read(&self.manifest_path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == ErrorKind::NotFound => Vec::new(),
            Err(source) => {
                return Err(GraveyardError::Io {
                    path: self.manifest_path.clone(),
                    source,
                });
            }
        };
        let serialized = serde_json::to_string(record)?;

        let parent = self
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let mut file = tempfile::Builder::new()
            .prefix(".manifest.jsonl.")
            .tempfile_in(parent)
            .map_err(|source| GraveyardError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        file.write_all(&existing)
            .map_err(|source| GraveyardError::Io {
                path: file.path().to_path_buf(),
                source,
            })?;
        if !existing.is_empty() && !existing.ends_with(b"\n") {
            file.write_all(b"\n").map_err(|source| GraveyardError::Io {
                path: file.path().to_path_buf(),
                source,
            })?;
        }
        writeln!(file, "{serialized}").map_err(|source| GraveyardError::Io {
            path: file.path().to_path_buf(),
            source,
        })?;
        file.as_file()
            .sync_data()
            .map_err(|source| GraveyardError::Io {
                path: file.path().to_path_buf(),
                source,
            })?;
        file.persist(&self.manifest_path)
            .map_err(|err| GraveyardError::Io {
                path: self.manifest_path.clone(),
                source: err.error,
            })?;
        Ok(())
    }
}

/// RAII handle that removes the lock file on drop. If the process
/// crashes between create_new and remove, the next writer hits the
/// retry loop, waits up to 500 ms total, then returns a clear
/// `ManifestLockContention` error — operator can `rm` the lock file
/// manually.
struct ScopedLock {
    _file: File,
    path: PathBuf,
}

impl ScopedLock {
    fn new(file: File, path: PathBuf) -> Self {
        Self { _file: file, path }
    }
}

impl Drop for ScopedLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn record(id: &str) -> ManifestRecord {
        let deleted_at = Utc::now();
        ManifestRecord {
            schema_version: MANIFEST_SCHEMA_VERSION,
            id: id.to_string(),
            deleted_at,
            expires_at: deleted_at + chrono::Duration::days(7),
            original_path: PathBuf::from(format!("/tmp/{id}")),
            size_bytes: 3,
            plan_id: None,
            rule_id: "test.rule".to_string(),
            category: "build".to_string(),
            safety_at_delete: "safe".to_string(),
            risk_score_at_delete: 0.0,
            tool_version: "0.0.0-test".to_string(),
            grave_path: PathBuf::from(format!("2026/05/16/120000-{id}")),
        }
    }

    #[test]
    fn append_preserves_existing_records() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let writer = RecordWriter::new(root.path());

        writer.append(&record("first"))?;
        writer.append(&record("second"))?;

        let records = ManifestReader::open(&root.path().join("manifest.jsonl"))?.read_all()?;
        let ids: Vec<_> = records.iter().map(|record| record.id.as_str()).collect();
        assert_eq!(ids, vec!["first", "second"]);
        Ok(())
    }
}

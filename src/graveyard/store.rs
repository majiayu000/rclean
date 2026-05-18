use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;

use super::GraveyardError;
use super::id;
use super::manifest::{
    GraveId, MANIFEST_SCHEMA_VERSION, ManifestReader, ManifestRecord, RecordWriter,
};

/// Default TTL for a grave before it becomes garbage-collectable.
/// SPEC §4.7.6: configurable via `RCLEAN_GRAVEYARD_TTL` once user
/// demand exists; hardcoded for now.
const DEFAULT_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Input describing a single delete operation handed to `Graveyard::bury`.
/// Snapshots safety/risk/rule metadata at delete time so the manifest
/// record captures the state *at* delete, not the state at restore.
#[derive(Debug, Clone)]
pub struct GraveInput<'a> {
    pub original_path: &'a Path,
    pub size_bytes: u64,
    pub plan_id: Option<GraveId>,
    pub rule_id: &'a str,
    pub category: &'a str,
    pub safety_at_delete: &'a str,
    pub risk_score_at_delete: f32,
    pub tool_version: &'a str,
}

/// A grave in memory — the on-disk payload at `payload_path` plus the
/// manifest record that points at it.
#[derive(Debug, Clone)]
pub struct Grave {
    pub record: ManifestRecord,
    pub payload_path: PathBuf,
}

/// Filesystem-backed graveyard rooted at one directory.
pub struct Graveyard {
    root: PathBuf,
}

impl Graveyard {
    /// Open or create a graveyard at `root`. The directory is created
    /// lazily on first `bury`; `open` itself only stores the path so
    /// `cargo install --no-default-features` users never see the dir.
    pub fn open(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Move `input.original_path` into the graveyard and append a
    /// manifest record. Returns the resulting `Grave`.
    ///
    /// Cross-FS handling: if `fs::rename` returns `EXDEV` we fall back
    /// to `copy + remove`. Slow, but correct.
    pub fn bury(&self, input: GraveInput<'_>) -> Result<Grave, GraveyardError> {
        fs::create_dir_all(&self.root).map_err(|source| GraveyardError::Io {
            path: self.root.clone(),
            source,
        })?;

        let deleted_at = Utc::now();
        let expires_at = deleted_at + chrono::Duration::from_std(DEFAULT_TTL).unwrap();
        let id: GraveId = id::generate();

        // YYYY/MM/DD/HHmmSS-<id>/
        let date_subdir = deleted_at.format("%Y/%m/%d").to_string();
        let leaf = format!("{}-{}", deleted_at.format("%H%M%S"), id);
        let grave_dir = self.root.join(&date_subdir).join(&leaf);
        let payload_path = grave_dir.join("payload");
        let grave_path_rel = PathBuf::from(&date_subdir).join(&leaf);

        fs::create_dir_all(&grave_dir).map_err(|source| GraveyardError::Io {
            path: grave_dir.clone(),
            source,
        })?;

        move_into(input.original_path, &payload_path)?;

        let record = ManifestRecord {
            schema_version: MANIFEST_SCHEMA_VERSION,
            id,
            deleted_at,
            expires_at,
            original_path: input.original_path.to_path_buf(),
            size_bytes: input.size_bytes,
            plan_id: input.plan_id,
            rule_id: input.rule_id.to_string(),
            category: input.category.to_string(),
            safety_at_delete: input.safety_at_delete.to_string(),
            risk_score_at_delete: input.risk_score_at_delete,
            tool_version: input.tool_version.to_string(),
            grave_path: grave_path_rel,
        };

        // Write the per-grave meta.json next to the payload — lets a
        // human inspect one grave with just `cat meta.json` without
        // grepping the global manifest.
        let meta_path = grave_dir.join("meta.json");
        let meta_json = serde_json::to_string_pretty(&record)?;
        fs::write(&meta_path, meta_json).map_err(|source| GraveyardError::Io {
            path: meta_path,
            source,
        })?;

        RecordWriter::new(&self.root).append(&record)?;

        Ok(Grave {
            record,
            payload_path,
        })
    }

    /// Reads the manifest and returns every record in file order.
    /// Missing manifest = empty graveyard, not an error.
    pub fn list(&self) -> Result<Vec<ManifestRecord>, GraveyardError> {
        let manifest_path = self.root.join("manifest.jsonl");
        if !manifest_path.exists() {
            return Ok(Vec::new());
        }
        ManifestReader::open(&manifest_path)?.read_all()
    }
}

/// Move `src` to `dst`. Falls back to copy + remove if rename fails
/// with EXDEV (cross-filesystem). Other errors propagate.
fn move_into(src: &Path, dst: &Path) -> Result<(), GraveyardError> {
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(err) if cross_fs(&err) => {
            tracing::warn!(
                src = %src.display(),
                dst = %dst.display(),
                "graveyard: cross-filesystem rename, falling back to copy"
            );
            copy_dir_all(src, dst)?;
            fs::remove_dir_all(src).map_err(|source| GraveyardError::Io {
                path: src.to_path_buf(),
                source,
            })?;
            Ok(())
        }
        Err(source) => Err(GraveyardError::Io {
            path: src.to_path_buf(),
            source,
        }),
    }
}

/// EXDEV detection — Unix raw_os_error is 18 on Linux/macOS for
/// cross-device link. Windows reports a different code; fall back to
/// `ErrorKind::Other` matching for cross-platform safety.
fn cross_fs(err: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        err.raw_os_error() == Some(18)
    }
    #[cfg(not(unix))]
    {
        // On Windows, MoveFileExW with a cross-volume move can return
        // ERROR_NOT_SAME_DEVICE (17). Match by raw OS error to keep
        // the test surface narrow.
        err.raw_os_error() == Some(17)
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), GraveyardError> {
    fs::create_dir_all(dst).map_err(|source| GraveyardError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    for entry in fs::read_dir(src).map_err(|source| GraveyardError::Io {
        path: src.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| GraveyardError::Io {
            path: src.to_path_buf(),
            source,
        })?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type().map_err(|source| GraveyardError::Io {
            path: from.clone(),
            source,
        })?;
        if ft.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|source| GraveyardError::Io { path: from, source })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_input<'a>(path: &'a Path) -> GraveInput<'a> {
        GraveInput {
            original_path: path,
            size_bytes: 3,
            plan_id: None,
            rule_id: "test.rule",
            category: "build",
            safety_at_delete: "safe",
            risk_score_at_delete: 0.0,
            tool_version: "0.0.0-test",
        }
    }

    #[test]
    fn bury_moves_payload_and_writes_manifest() {
        let workspace = TempDir::new().unwrap();
        let graveyard = TempDir::new().unwrap();
        let yard = Graveyard::open(graveyard.path().to_path_buf());

        let victim = workspace.path().join("node_modules");
        fs::create_dir(&victim).unwrap();
        fs::write(victim.join("blob"), b"abc").unwrap();

        let grave = yard.bury(make_input(&victim)).unwrap();

        assert!(!victim.exists(), "original path should have been moved");
        assert!(grave.payload_path.is_dir(), "payload should exist");
        assert!(
            grave.payload_path.join("blob").is_file(),
            "payload contents preserved"
        );

        // meta.json + manifest.jsonl + manifest.jsonl.lock removed
        let meta = grave.payload_path.parent().unwrap().join("meta.json");
        assert!(meta.is_file());
        assert!(graveyard.path().join("manifest.jsonl").is_file());
        assert!(
            !graveyard.path().join("manifest.jsonl.lock").exists(),
            "lock should be cleaned up on drop"
        );
    }

    #[test]
    fn list_reads_back_what_bury_wrote() {
        let workspace = TempDir::new().unwrap();
        let graveyard = TempDir::new().unwrap();
        let yard = Graveyard::open(graveyard.path().to_path_buf());

        let victim = workspace.path().join("target");
        fs::create_dir(&victim).unwrap();
        fs::write(victim.join("x"), b"y").unwrap();
        let grave = yard.bury(make_input(&victim)).unwrap();

        let records = yard.list().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, grave.record.id);
        assert_eq!(records[0].original_path, victim);
        assert_eq!(records[0].rule_id, "test.rule");
        assert_eq!(records[0].schema_version, MANIFEST_SCHEMA_VERSION);
    }

    #[test]
    fn list_returns_empty_for_fresh_graveyard() {
        let graveyard = TempDir::new().unwrap();
        let yard = Graveyard::open(graveyard.path().to_path_buf());
        assert!(yard.list().unwrap().is_empty());
    }

    #[test]
    fn multiple_burials_each_get_a_unique_grave_path() {
        let workspace = TempDir::new().unwrap();
        let graveyard = TempDir::new().unwrap();
        let yard = Graveyard::open(graveyard.path().to_path_buf());

        for i in 0..3 {
            let victim = workspace.path().join(format!("v{i}"));
            fs::create_dir(&victim).unwrap();
            fs::write(victim.join("f"), b"z").unwrap();
            yard.bury(make_input(&victim)).unwrap();
        }

        let records = yard.list().unwrap();
        assert_eq!(records.len(), 3);
        let unique: std::collections::HashSet<_> = records.iter().map(|r| r.id.clone()).collect();
        assert_eq!(unique.len(), 3, "every grave id must be unique");
    }
}

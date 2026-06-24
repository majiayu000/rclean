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
        let meta_json = serde_json::to_string_pretty(&record)?;

        fs::create_dir_all(&grave_dir).map_err(|source| GraveyardError::Io {
            path: grave_dir.clone(),
            source,
        })?;

        move_into(input.original_path, &payload_path)?;

        // Write the per-grave meta.json next to the payload — lets a
        // human inspect one grave with just `cat meta.json` without
        // grepping the global manifest.
        let meta_path = grave_dir.join("meta.json");
        if let Err(err) = fs::write(&meta_path, meta_json).map_err(|source| GraveyardError::Io {
            path: meta_path,
            source,
        }) {
            return Err(rollback_bury(
                input.original_path,
                &payload_path,
                &grave_dir,
                err,
            ));
        }

        if let Err(err) = RecordWriter::new(&self.root).append(&record) {
            return Err(rollback_bury(
                input.original_path,
                &payload_path,
                &grave_dir,
                err,
            ));
        }

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

    /// Restore the grave with the given id back to its `original_path`
    /// (or `override_target` if provided). Returns the record that was
    /// restored. Enforces SPEC §4.7.5 edge cases:
    ///
    ///   1. Target path already exists → `RestoreTargetExists`.
    ///   2. Target parent is a symlink → `RestoreTargetParentIsSymlink`.
    ///   3. Target parent missing → re-created with default perms.
    ///   4. Cross-FS rename → copy + remove fallback.
    pub fn restore_by_id(
        &self,
        id: &str,
        override_target: Option<&Path>,
    ) -> Result<ManifestRecord, GraveyardError> {
        let records = self.list()?;
        let record = records
            .iter()
            .find(|r| r.id == id)
            .cloned()
            .ok_or_else(|| GraveyardError::GraveNotFound(id.to_string()))?;

        let target = override_target
            .map(Path::to_path_buf)
            .unwrap_or_else(|| record.original_path.clone());

        if target.exists() {
            return Err(GraveyardError::RestoreTargetExists { path: target });
        }
        if let Some(parent) = target.parent()
            && parent.exists()
            && parent
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false)
        {
            return Err(GraveyardError::RestoreTargetParentIsSymlink {
                path: parent.to_path_buf(),
            });
        }
        if let Some(parent) = target.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).map_err(|source| GraveyardError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let payload = self.root.join(&record.grave_path).join("payload");
        move_into(&payload, &target)?;

        // The grave directory still holds meta.json; drop it now that
        // the payload is gone. We deliberately don't fail the whole
        // restore if cleanup hiccups — the user already got their
        // data back; manifest GC will sweep the empty dir later.
        let grave_dir = self.root.join(&record.grave_path);
        if let Err(err) = fs::remove_dir_all(&grave_dir) {
            tracing::warn!(
                path = %grave_dir.display(),
                error = %err,
                "graveyard: failed to clean up empty grave directory after restore"
            );
        }

        rewrite_manifest_without(&self.root, &records, |r| r.id != id)?;
        Ok(record)
    }

    /// Remove every grave whose `expires_at` is before `now`. Returns
    /// the records that were collected (so callers can print a
    /// summary).
    pub fn gc(&self, dry_run: bool) -> Result<Vec<ManifestRecord>, GraveyardError> {
        let records = self.list()?;
        let now = Utc::now();
        let (expired, alive): (Vec<_>, Vec<_>) =
            records.iter().cloned().partition(|r| r.expires_at < now);

        if dry_run {
            return Ok(expired);
        }

        for record in &expired {
            let grave_dir = self.root.join(&record.grave_path);
            if let Err(err) = fs::remove_dir_all(&grave_dir) {
                tracing::warn!(
                    path = %grave_dir.display(),
                    error = %err,
                    "graveyard: gc failed to remove expired grave dir"
                );
            }
        }

        rewrite_manifest_atomic(&self.root, &alive)?;
        Ok(expired)
    }
}

/// Atomic-ish manifest rewrite: write to a sibling temp file, then
/// rename over the real manifest. The lock file protects against a
/// concurrent `bury()` writer; rewrite holds the same lock for the
/// full duration.
fn rewrite_manifest_atomic(root: &Path, kept: &[ManifestRecord]) -> Result<(), GraveyardError> {
    use std::io::Write;

    let manifest = root.join("manifest.jsonl");
    let tmp = root.join("manifest.jsonl.tmp");
    let lock_path = root.join("manifest.jsonl.lock");

    // Acquire the same advisory lock used by RecordWriter::append so a
    // concurrent bury isn't reading half-written state.
    let mut attempts: u32 = 0;
    let _lock = loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(file) => break LockGuard::new(file, lock_path.clone()),
            Err(_) if attempts < 5 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(_) => return Err(GraveyardError::ManifestLockContention { attempts }),
        }
    };

    let mut file = std::fs::File::create(&tmp).map_err(|source| GraveyardError::Io {
        path: tmp.clone(),
        source,
    })?;
    for record in kept {
        let line = serde_json::to_string(record)?;
        writeln!(file, "{line}").map_err(|source| GraveyardError::Io {
            path: tmp.clone(),
            source,
        })?;
    }
    file.sync_data().map_err(|source| GraveyardError::Io {
        path: tmp.clone(),
        source,
    })?;
    drop(file);

    fs::rename(&tmp, &manifest).map_err(|source| GraveyardError::Io {
        path: manifest,
        source,
    })?;
    Ok(())
}

fn rewrite_manifest_without<F>(
    root: &Path,
    records: &[ManifestRecord],
    keep: F,
) -> Result<(), GraveyardError>
where
    F: Fn(&ManifestRecord) -> bool,
{
    let kept: Vec<ManifestRecord> = records.iter().filter(|r| keep(r)).cloned().collect();
    rewrite_manifest_atomic(root, &kept)
}

fn rollback_bury(
    original_path: &Path,
    payload_path: &Path,
    grave_dir: &Path,
    original_failure: GraveyardError,
) -> GraveyardError {
    if let Err(rollback_failure) = move_into(payload_path, original_path) {
        return rollback_failed(
            original_path,
            payload_path,
            grave_dir,
            original_failure,
            "moving payload back to the original path",
            rollback_failure,
        );
    }

    if let Err(source) = fs::remove_dir_all(grave_dir) {
        return rollback_failed(
            original_path,
            payload_path,
            grave_dir,
            original_failure,
            "cleaning orphan grave directory after payload restore",
            GraveyardError::Io {
                path: grave_dir.to_path_buf(),
                source,
            },
        );
    }

    original_failure
}

fn rollback_failed(
    original_path: &Path,
    payload_path: &Path,
    grave_dir: &Path,
    original_failure: GraveyardError,
    rollback_step: &str,
    rollback_failure: GraveyardError,
) -> GraveyardError {
    GraveyardError::Generic(format!(
        "graveyard bury failed after moving payload from {} to {}; original failure: {}; rollback failed while {} for grave dir {}: {}",
        original_path.display(),
        payload_path.display(),
        original_failure,
        rollback_step,
        grave_dir.display(),
        rollback_failure
    ))
}

/// Local copy of the RAII lock guard so we can use it from this file
/// without exposing the manifest module's internal type.
struct LockGuard {
    _file: std::fs::File,
    path: PathBuf,
}

impl LockGuard {
    fn new(file: std::fs::File, path: PathBuf) -> Self {
        Self { _file: file, path }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
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

    #[test]
    fn bury_rolls_back_payload_and_grave_dir_when_manifest_append_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let workspace = TempDir::new()?;
        let graveyard = TempDir::new()?;
        let yard = Graveyard::open(graveyard.path().to_path_buf());

        let victim = workspace.path().join("node_modules");
        fs::create_dir(&victim)?;
        fs::write(victim.join("blob"), b"abc")?;

        fs::write(graveyard.path().join("manifest.jsonl.lock"), b"stale")?;

        let err = match yard.bury(make_input(&victim)) {
            Ok(_) => panic!("bury unexpectedly succeeded despite manifest lock contention"),
            Err(err) => err,
        };

        assert!(matches!(
            err,
            GraveyardError::ManifestLockContention { attempts: 5 }
        ));
        assert!(victim.is_dir(), "original path should be restored");
        assert_eq!(fs::read(victim.join("blob"))?, b"abc");
        assert!(
            !graveyard.path().join("manifest.jsonl").exists(),
            "failed manifest append should not create a manifest"
        );
        assert!(
            grave_leaf_dirs(graveyard.path())?.is_empty(),
            "rollback should remove orphan grave directories"
        );
        Ok(())
    }

    fn grave_leaf_dirs(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut out = Vec::new();
        collect_grave_leaf_dirs(root, root, &mut out)?;
        Ok(out)
    }

    fn collect_grave_leaf_dirs(
        root: &Path,
        dir: &Path,
        out: &mut Vec<PathBuf>,
    ) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let depth = relative.components().count();
            if depth == 4 {
                out.push(path);
            } else {
                collect_grave_leaf_dirs(root, &path, out)?;
            }
        }
        Ok(())
    }
}

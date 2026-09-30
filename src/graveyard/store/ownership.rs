use std::fs;
use std::path::{Path, PathBuf};

use super::{GraveyardError, ManifestRecord, contained_grave_dir};

/// Pin mutations to the leaf that `bury` creates for this record. Root
/// containment alone also admits shared date directories and other graves.
pub(super) fn owned_grave_dir(
    root: &Path,
    record: &ManifestRecord,
) -> Result<PathBuf, GraveyardError> {
    let resolved = contained_grave_dir(root, &record.grave_path)?;
    let not_owned = || GraveyardError::GravePathNotOwned {
        path: record.grave_path.clone(),
        id: record.id.clone(),
    };
    let expected = PathBuf::from(record.deleted_at.format("%Y/%m/%d").to_string()).join(format!(
        "{}-{}",
        record.deleted_at.format("%H%M%S"),
        record.id
    ));
    if record.grave_path != expected {
        return Err(not_owned());
    }

    // Inspect every component without following links, including Windows
    // junctions. A link inside the root can still select a different grave.
    let joined = root.join(&record.grave_path);
    let mut current = root.to_path_buf();
    for component in record.grave_path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if crate::scan::dangerous_link_kind(&metadata).is_some() => {
                return Err(not_owned());
            }
            // Let GC retain non-directory graves and report the original
            // remove_dir_all error rather than an incidental metadata error.
            Ok(metadata) if current == joined && !metadata.is_dir() => return Ok(resolved),
            Ok(_) => {}
            // GC can retry after deletion succeeded but manifest rewrite failed.
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(resolved),
            Err(source) => {
                return Err(GraveyardError::Io {
                    path: current,
                    source,
                });
            }
        }
    }

    // Bury writes this independent record beside the payload. Comparing the
    // complete record also binds restore destinations and GC expiry decisions;
    // self-consistent manifest fields alone cannot establish ownership.
    let meta_path = resolved.join("meta.json");
    let metadata = fs::symlink_metadata(&meta_path).map_err(|source| GraveyardError::Io {
        path: meta_path.clone(),
        source,
    })?;
    if crate::scan::dangerous_link_kind(&metadata).is_some() {
        return Err(not_owned());
    }
    let bytes = fs::read(&meta_path).map_err(|source| GraveyardError::Io {
        path: meta_path,
        source,
    })?;
    let stored: ManifestRecord = serde_json::from_slice(&bytes)?;
    if stored != *record {
        return Err(not_owned());
    }
    Ok(resolved)
}

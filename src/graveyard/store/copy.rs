use std::fs;
use std::path::Path;

use super::GraveyardError;

pub(super) fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), GraveyardError> {
    // Only clean a destination that this copy created, never an existing tree.
    fs::create_dir(dst).map_err(|source| GraveyardError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    if let Err(err) = copy_dir_contents(src, dst) {
        if let Err(cleanup_error) = fs::remove_dir_all(dst) {
            tracing::error!(
                path = %dst.display(),
                error = %cleanup_error,
                "graveyard: failed to remove partial cross-filesystem copy"
            );
        }
        return Err(err);
    }
    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), GraveyardError> {
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
        if ft.is_symlink() {
            let target = fs::read_link(&from).map_err(|source| GraveyardError::Io {
                path: from.clone(),
                source,
            })?;
            #[cfg(unix)]
            let result = std::os::unix::fs::symlink(&target, &to);
            #[cfg(windows)]
            let result = {
                use std::os::windows::fs::FileTypeExt;
                if ft.is_symlink_dir() {
                    std::os::windows::fs::symlink_dir(&target, &to)
                } else {
                    std::os::windows::fs::symlink_file(&target, &to)
                }
            };
            result.map_err(|source| GraveyardError::Io { path: to, source })?;
        } else if ft.is_dir() {
            fs::create_dir(&to).map_err(|source| GraveyardError::Io {
                path: to.clone(),
                source,
            })?;
            copy_dir_contents(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|source| GraveyardError::Io { path: from, source })?;
        }
    }
    Ok(())
}

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::scan::dangerous_link_kind;

use super::GraveyardError;

/// Check every existing prefix before creating any missing directories.
/// `absolute` anchors relative destinations without resolving their symlinks.
pub(super) fn prepare_restore_parent(parent: &Path) -> Result<(), GraveyardError> {
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let parent = std::path::absolute(parent).map_err(|source| GraveyardError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    let mut ancestor = PathBuf::new();
    let mut missing = Vec::new();
    for component in parent.components() {
        if component == Component::ParentDir {
            // PathBuf::push would normalize `..` in Windows verbatim paths
            // before we could check the traversed prefix.
            ancestor
                .as_mut_os_string()
                .push(std::path::MAIN_SEPARATOR_STR);
            ancestor.as_mut_os_string().push(component.as_os_str());
        } else {
            ancestor.push(component);
        }
        // A Windows prefix is not a complete absolute path until its root.
        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&ancestor) {
            Ok(metadata)
                if metadata.file_type().is_symlink()
                    || (metadata.is_dir() && dangerous_link_kind(&metadata).is_some()) =>
            {
                return Err(GraveyardError::RestoreTargetParentIsSymlink { path: ancestor });
            }
            Ok(_) => {}
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                if component != Component::ParentDir && !missing.contains(&ancestor) {
                    missing.push(ancestor.clone());
                }
            }
            Err(source) => {
                return Err(GraveyardError::Io {
                    path: ancestor,
                    source,
                });
            }
        }
        if component == Component::ParentDir {
            // Check the traversed prefix before reducing `..`: a symlink or
            // non-directory must not disappear from validation. A missing
            // prefix still needs creating for the literal destination path.
            ancestor.pop();
            ancestor.pop();
        }
    }
    for directory in missing {
        if let Err(source) = fs::create_dir(&directory) {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                // Case aliases can refer to a prefix created earlier in this loop.
                let metadata =
                    fs::symlink_metadata(&directory).map_err(|source| GraveyardError::Io {
                        path: directory.clone(),
                        source,
                    })?;
                if metadata.file_type().is_symlink()
                    || (metadata.is_dir() && dangerous_link_kind(&metadata).is_some())
                {
                    return Err(GraveyardError::RestoreTargetParentIsSymlink { path: directory });
                }
                if metadata.is_dir() {
                    continue;
                }
            }
            return Err(GraveyardError::Io {
                path: directory,
                source,
            });
        }
    }
    Ok(())
}

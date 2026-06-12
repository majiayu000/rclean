use std::path::Path;

pub(crate) fn path_file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

pub(crate) fn path_file_name_string(path: &Path) -> Option<String> {
    path_file_name(path).map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_utf8_file_name() {
        assert_eq!(path_file_name(Path::new("/path/to/target")), Some("target"));
        assert_eq!(
            path_file_name_string(Path::new("/path/to/target")),
            Some("target".to_string())
        );
        assert_eq!(path_file_name(Path::new("relative/path")), Some("path"));
    }

    #[test]
    fn returns_none_when_file_name_is_absent() {
        assert_eq!(path_file_name(Path::new("/")), None);
        assert_eq!(path_file_name_string(Path::new("/")), None);
    }

    #[test]
    #[cfg(unix)]
    fn returns_none_for_non_utf8_file_name() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        assert_eq!(path_file_name(Path::new(OsStr::from_bytes(b"\xff"))), None);
        assert_eq!(
            path_file_name_string(Path::new(OsStr::from_bytes(b"\xff"))),
            None
        );
    }
}

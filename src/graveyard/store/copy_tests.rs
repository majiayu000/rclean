use super::*;
use tempfile::TempDir;

#[cfg(any(unix, windows))]
fn symlink(target: &Path, link: &Path, directory: bool) {
    #[cfg(unix)]
    {
        let _ = directory;
        std::os::unix::fs::symlink(target, link).unwrap();
    }
    #[cfg(windows)]
    if directory {
        std::os::windows::fs::symlink_dir(target, link).unwrap();
    } else {
        std::os::windows::fs::symlink_file(target, link).unwrap();
    }
}

#[cfg(any(unix, windows))]
#[test]
fn copy_preserves_file_symlinks_without_copying_external_bytes() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir_all(src.join("nested")).unwrap();
    let external = temp.path().join("external-file");
    fs::write(&external, b"external sentinel").unwrap();
    fs::write(src.join("blob"), b"payload bytes").unwrap();
    symlink(&external, &src.join("absolute-link"), false);
    symlink(
        Path::new("../blob"),
        &src.join("nested/relative-link"),
        false,
    );

    copy_dir_all(&src, &dst).unwrap();

    for name in ["absolute-link", "nested/relative-link"] {
        assert!(dst.join(name).symlink_metadata().unwrap().is_symlink());
        assert_eq!(
            fs::read_link(dst.join(name)).unwrap(),
            fs::read_link(src.join(name)).unwrap()
        );
    }
    assert_eq!(fs::read(dst.join("blob")).unwrap(), b"payload bytes");
    assert_eq!(fs::read(&external).unwrap(), b"external sentinel");
    assert!(
        src.exists(),
        "copy must leave the source for move_into to remove"
    );
}

#[cfg(any(unix, windows))]
#[test]
fn copy_preserves_directory_symlinks_without_recursing() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir(&src).unwrap();
    let external = temp.path().join("external-dir");
    fs::create_dir(&external).unwrap();
    fs::write(external.join("sentinel"), b"external bytes").unwrap();
    symlink(&external, &src.join("directory-link"), true);
    symlink(Path::new("."), &src.join("cycle"), true);

    copy_dir_all(&src, &dst).unwrap();

    for name in ["directory-link", "cycle"] {
        let ft = dst.join(name).symlink_metadata().unwrap().file_type();
        assert!(ft.is_symlink());
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            assert!(ft.is_symlink_dir());
        }
        assert_eq!(
            fs::read_link(dst.join(name)).unwrap(),
            fs::read_link(src.join(name)).unwrap()
        );
    }
    assert_eq!(
        fs::read(external.join("sentinel")).unwrap(),
        b"external bytes"
    );
}

#[cfg(any(unix, windows))]
#[test]
fn copy_preserves_dangling_file_and_directory_symlinks() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir(&src).unwrap();
    symlink(Path::new("missing-file"), &src.join("file-link"), false);
    symlink(Path::new("missing-dir"), &src.join("dir-link"), true);

    copy_dir_all(&src, &dst).unwrap();

    for name in ["file-link", "dir-link"] {
        let ft = dst.join(name).symlink_metadata().unwrap().file_type();
        assert!(ft.is_symlink());
        assert!(!dst.join(name).exists());
        assert_eq!(
            fs::read_link(dst.join(name)).unwrap(),
            fs::read_link(src.join(name)).unwrap()
        );
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            assert_eq!(ft.is_symlink_dir(), name == "dir-link");
        }
    }
}

#[cfg(unix)]
#[test]
fn copy_failure_removes_partial_destination_and_keeps_source() {
    use std::os::unix::net::UnixListener;

    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir_all(src.join("nested")).unwrap();
    fs::write(src.join("blob"), b"payload bytes").unwrap();
    let socket = src.join("nested/socket");
    let _listener = UnixListener::bind(&socket).unwrap();

    let err = copy_dir_all(&src, &dst).unwrap_err();

    assert!(matches!(err, GraveyardError::Io { path, .. } if path == socket));
    assert!(
        !dst.exists(),
        "failed copy must remove its partial destination"
    );
    assert_eq!(fs::read(src.join("blob")).unwrap(), b"payload bytes");
    assert!(socket.symlink_metadata().is_ok());
}

#[test]
fn copy_refuses_existing_destination_without_removing_its_data() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("blob"), b"payload bytes").unwrap();
    fs::create_dir(&dst).unwrap();
    fs::write(dst.join("sentinel"), b"existing bytes").unwrap();

    let err = copy_dir_all(&src, &dst).unwrap_err();

    assert!(matches!(err, GraveyardError::Io { path, source }
        if path == dst && source.kind() == std::io::ErrorKind::AlreadyExists));
    assert_eq!(fs::read(dst.join("sentinel")).unwrap(), b"existing bytes");
    assert!(!dst.join("blob").exists());
    assert_eq!(fs::read(src.join("blob")).unwrap(), b"payload bytes");
}

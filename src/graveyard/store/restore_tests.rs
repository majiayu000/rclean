use super::*;
use tempfile::TempDir;

fn bury_payload(yard: &Graveyard, path: &Path) -> Grave {
    fs::create_dir_all(path).unwrap();
    fs::write(path.join("blob"), b"abc").unwrap();
    yard.bury(make_input(path)).unwrap()
}

#[cfg(any(unix, windows))]
fn symlink_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(target, link).unwrap();
}

#[cfg(any(unix, windows))]
fn assert_ancestor_symlink_refused(parent_exists: bool, override_target: bool) {
    let temp = TempDir::new().unwrap();
    // Keep system aliases such as macOS /var out of this fixture so the
    // refusal must identify the symlink we introduce below.
    let root = temp.path().canonicalize().unwrap();
    let yard = Graveyard::open(root.join("graveyard"));
    let link = root.join("link");
    let target = link.join("nested").join("payload");
    let original = if override_target {
        root.join("original")
    } else {
        target.clone()
    };
    let grave = bury_payload(&yard, &original);
    if !override_target {
        fs::remove_dir_all(&link).unwrap();
    }

    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"unchanged").unwrap();
    if parent_exists {
        fs::create_dir(outside.join("nested")).unwrap();
    }
    symlink_dir(&outside, &link);
    let manifest_before = fs::read(yard.root().join("manifest.jsonl")).unwrap();

    let err = yard
        .restore_by_id(
            &grave.record.id,
            override_target.then_some(target.as_path()),
        )
        .expect_err("restore must refuse an ancestor symlink");

    match err {
        GraveyardError::RestoreTargetParentIsSymlink { path } => assert_eq!(path, link),
        other => panic!("unexpected restore error: {other}"),
    }
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"unchanged");
    assert_eq!(outside.join("nested").exists(), parent_exists);
    assert!(!outside.join("nested").join("payload").exists());
    assert_eq!(fs::read(grave.payload_path.join("blob")).unwrap(), b"abc");
    assert_eq!(
        fs::read(yard.root().join("manifest.jsonl")).unwrap(),
        manifest_before
    );
}

#[test]
#[cfg(any(unix, windows))]
fn restore_original_refuses_ancestor_symlink_with_missing_parent() {
    assert_ancestor_symlink_refused(false, false);
}

#[test]
#[cfg(any(unix, windows))]
fn restore_original_refuses_ancestor_symlink_with_existing_parent() {
    assert_ancestor_symlink_refused(true, false);
}

#[test]
#[cfg(any(unix, windows))]
fn restore_override_refuses_ancestor_symlink_with_missing_parent() {
    assert_ancestor_symlink_refused(false, true);
}

#[test]
#[cfg(any(unix, windows))]
fn restore_override_refuses_ancestor_symlink_with_existing_parent() {
    assert_ancestor_symlink_refused(true, true);
}

#[test]
#[cfg(any(unix, windows))]
fn restore_refuses_dangling_parent_symlink_without_creating_its_target() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let yard = Graveyard::open(root.join("graveyard"));
    let grave = bury_payload(&yard, &root.join("original"));
    let missing = root.join("missing");
    let link = root.join("link");
    symlink_dir(&missing, &link);

    let err = yard
        .restore_by_id(&grave.record.id, Some(&link.join("payload")))
        .expect_err("dangling parent symlink must be refused");
    assert!(matches!(
        err,
        GraveyardError::RestoreTargetParentIsSymlink { path } if path == link
    ));
    assert!(!missing.exists());
    assert_eq!(fs::read(grave.payload_path.join("blob")).unwrap(), b"abc");
    assert_eq!(yard.list().unwrap().len(), 1);
}

#[test]
fn restore_recreates_missing_parents_without_symlinks() {
    for override_target in [false, true] {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let yard = Graveyard::open(root.join("graveyard"));
        let original = root.join("workspace").join("nested").join("payload");
        let grave = bury_payload(&yard, &original);
        fs::remove_dir_all(root.join("workspace")).unwrap();
        let target = if override_target {
            root.join("alternate").join("nested").join("payload")
        } else {
            original.clone()
        };

        yard.restore_by_id(
            &grave.record.id,
            override_target.then_some(target.as_path()),
        )
        .unwrap();

        assert_eq!(fs::read(target.join("blob")).unwrap(), b"abc");
        assert!(!grave.payload_path.exists());
        assert!(yard.list().unwrap().is_empty());
    }
}

#[test]
fn restore_reports_parent_metadata_errors_without_moving_payload() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let yard = Graveyard::open(root.join("graveyard"));
    let grave = bury_payload(&yard, &root.join("original"));
    let blocker = root.join("file");
    fs::write(&blocker, b"keep").unwrap();
    let parent = blocker.join("nested");
    let manifest_before = fs::read(yard.root().join("manifest.jsonl")).unwrap();

    let err = yard
        .restore_by_id(&grave.record.id, Some(&parent.join("payload")))
        .expect_err("non-directory ancestor must fail explicitly");
    assert!(matches!(
        err,
        GraveyardError::Io { path, .. } if path == parent
    ));
    assert_eq!(fs::read(&blocker).unwrap(), b"keep");
    assert_eq!(fs::read(grave.payload_path.join("blob")).unwrap(), b"abc");
    assert_eq!(
        fs::read(yard.root().join("manifest.jsonl")).unwrap(),
        manifest_before
    );
}

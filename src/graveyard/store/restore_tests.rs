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
fn assert_ancestor_symlink_refused(
    parent_exists: bool,
    override_target: bool,
    link_dir: fn(&Path, &Path),
) {
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
    link_dir(&outside, &link);
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
    assert_ancestor_symlink_refused(false, false, symlink_dir);
}

#[test]
#[cfg(any(unix, windows))]
fn restore_original_refuses_ancestor_symlink_with_existing_parent() {
    assert_ancestor_symlink_refused(true, false, symlink_dir);
}

#[test]
#[cfg(any(unix, windows))]
fn restore_override_refuses_ancestor_symlink_with_missing_parent() {
    assert_ancestor_symlink_refused(false, true, symlink_dir);
}

#[test]
#[cfg(any(unix, windows))]
fn restore_override_refuses_ancestor_symlink_with_existing_parent() {
    assert_ancestor_symlink_refused(true, true, symlink_dir);
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

#[test]
#[cfg(windows)]
fn restore_refuses_junction_ancestors() {
    fn junction_dir(target: &Path, link: &Path) {
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "mklink failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for (parent_exists, override_target) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        assert_ancestor_symlink_refused(parent_exists, override_target, junction_dir);
    }
}

#[test]
#[cfg(unix)]
fn restore_accepts_missing_prefix_followed_by_parent_components() {
    for (relative, resolved) in [
        ("missing/../restored", "restored"),
        (
            "missing/../existing/nested/restored",
            "existing/nested/restored",
        ),
        ("missing/../missing/../restored", "restored"),
    ] {
        for override_target in [false, true] {
            let temp = TempDir::new().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let yard = Graveyard::open(root.join("graveyard"));
            fs::create_dir(root.join("existing")).unwrap();
            fs::create_dir(root.join("missing")).unwrap();
            let target = root.join(relative);
            let original = if override_target {
                root.join("original")
            } else {
                target.clone()
            };
            let grave = bury_payload(&yard, &original);
            fs::remove_dir_all(root.join("missing")).unwrap();

            yard.restore_by_id(
                &grave.record.id,
                override_target.then_some(target.as_path()),
            )
            .unwrap();

            assert_eq!(fs::read(root.join(resolved).join("blob")).unwrap(), b"abc");
            assert!(!grave.payload_path.exists());
            assert!(yard.list().unwrap().is_empty());
        }
    }
}

#[test]
#[cfg(any(unix, windows))]
fn restore_accepts_case_aliases_of_missing_parents() {
    for override_target in [false, true] {
        let temp = TempDir::new().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let yard = Graveyard::open(root.join("graveyard"));
        fs::create_dir(root.join("Missing")).unwrap();
        // Preserve parent components in Windows verbatim paths.
        let mut literal = root.clone().into_os_string();
        literal.push(std::path::MAIN_SEPARATOR_STR);
        literal.push("Missing/../missing/restored".replace('/', std::path::MAIN_SEPARATOR_STR));
        let target = PathBuf::from(literal);
        let original = if override_target {
            root.join("original")
        } else {
            target.clone()
        };
        let grave = bury_payload(&yard, &original);
        for name in ["Missing", "missing"] {
            let directory = root.join(name);
            if directory.exists() {
                fs::remove_dir_all(directory).unwrap();
            }
        }

        yard.restore_by_id(
            &grave.record.id,
            override_target.then_some(target.as_path()),
        )
        .unwrap();

        assert_eq!(
            fs::read(root.join("missing/restored/blob")).unwrap(),
            b"abc"
        );
        assert!(!grave.payload_path.exists());
        assert!(yard.list().unwrap().is_empty());
    }
}

#[test]
#[cfg(any(unix, windows))]
fn restore_refuses_target_revealed_by_creating_missing_parent() {
    for override_target in [false, true] {
        for occupied_by in ["empty_directory", "directory", "file", "dangling_symlink"] {
            let temp = TempDir::new().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let yard = Graveyard::open(root.join("graveyard"));
            fs::create_dir(root.join("missing")).unwrap();
            let mut literal = root.clone().into_os_string();
            literal.push(std::path::MAIN_SEPARATOR_STR);
            literal.push("missing/../existing".replace('/', std::path::MAIN_SEPARATOR_STR));
            let target = PathBuf::from(literal);
            let original = if override_target {
                root.join("original")
            } else {
                target.clone()
            };
            let grave = bury_payload(&yard, &original);
            fs::remove_dir(root.join("missing")).unwrap();
            let existing = root.join("existing");
            match occupied_by {
                "empty_directory" => fs::create_dir(&existing).unwrap(),
                "directory" => {
                    fs::create_dir(&existing).unwrap();
                    fs::write(existing.join("blob"), b"keep").unwrap();
                }
                "file" => fs::write(&existing, b"keep").unwrap(),
                "dangling_symlink" => symlink_dir(&root.join("absent"), &existing),
                _ => unreachable!(),
            }
            let manifest_before = fs::read(yard.root().join("manifest.jsonl")).unwrap();
            assert!(!target.exists());

            let err = yard
                .restore_by_id(
                    &grave.record.id,
                    override_target.then_some(target.as_path()),
                )
                .expect_err("restore must not overwrite a newly reachable target");

            assert!(matches!(
                err,
                GraveyardError::RestoreTargetExists { path } if path == target
            ));
            match occupied_by {
                "empty_directory" => assert_eq!(fs::read_dir(&existing).unwrap().count(), 0),
                "directory" => assert_eq!(fs::read(existing.join("blob")).unwrap(), b"keep"),
                "file" => assert_eq!(fs::read(&existing).unwrap(), b"keep"),
                "dangling_symlink" => {
                    assert_eq!(fs::read_link(&existing).unwrap(), root.join("absent"));
                    assert!(!root.join("absent").exists());
                }
                _ => unreachable!(),
            }
            assert_eq!(fs::read(grave.payload_path.join("blob")).unwrap(), b"abc");
            assert_eq!(
                fs::read(yard.root().join("manifest.jsonl")).unwrap(),
                manifest_before
            );
        }
    }
}

#[test]
#[cfg(any(unix, windows))]
fn restore_refuses_symlink_before_parent_component() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let yard = Graveyard::open(root.join("graveyard"));
    let grave = bury_payload(&yard, &root.join("original"));
    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    let link = root.join("link");
    symlink_dir(&outside, &link);
    let manifest_before = fs::read(yard.root().join("manifest.jsonl")).unwrap();
    for relative in ["link/../restored", "missing/../link/../restored"] {
        // Build the literal path without PathBuf::join normalizing `..` on Windows.
        let mut target = root.clone().into_os_string();
        target.push(std::path::MAIN_SEPARATOR_STR);
        target.push(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        let err = yard
            .restore_by_id(&grave.record.id, Some(Path::new(&target)))
            .expect_err("a parent component must not erase a link from validation");
        assert!(matches!(
            err,
            GraveyardError::RestoreTargetParentIsSymlink { path } if path == link
        ));
        assert!(!root.join("restored").exists());
        assert!(!root.join("missing").exists());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        assert_eq!(fs::read(grave.payload_path.join("blob")).unwrap(), b"abc");
        assert_eq!(
            fs::read(yard.root().join("manifest.jsonl")).unwrap(),
            manifest_before
        );
    }
}

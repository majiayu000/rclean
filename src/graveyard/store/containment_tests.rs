use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use tempfile::TempDir;

use super::*;

fn bury_sample(yard: &Graveyard, original: &Path) -> Grave {
    fs::create_dir_all(original).unwrap();
    fs::write(original.join("blob"), b"abc").unwrap();
    yard.bury(make_input(original)).unwrap()
}

fn write_records(root: &Path, records: &[ManifestRecord]) {
    let mut body = String::new();
    for record in records {
        body.push_str(&serde_json::to_string(record).unwrap());
        body.push('\n');
    }
    fs::write(root.join("manifest.jsonl"), body).unwrap();
}

fn set_record_paths(yard: &Graveyard, id: &str, grave_path: PathBuf, expire: bool) {
    let mut records = yard.list().unwrap();
    let mut found = false;
    for record in &mut records {
        if record.id == id {
            record.grave_path = grave_path.clone();
            if expire {
                record.expires_at = Utc::now() - chrono::Duration::days(1);
            }
            found = true;
        }
    }
    assert!(found, "record id must exist");
    write_records(yard.root(), &records);
}

fn assert_escape_err(err: GraveyardError, grave_path: &Path) {
    match err {
        GraveyardError::GravePathEscapesRoot { path } => {
            assert_eq!(path, grave_path);
        }
        other => panic!("unexpected error for {}: {other}", grave_path.display()),
    }
}

fn assert_escapes(root: &Path, grave_path: &Path) {
    assert_escape_err(
        contained_grave_dir(root, grave_path).expect_err("escaped grave_path must fail"),
        grave_path,
    );
}

#[test]
fn contained_relative_grave_path_stays_under_root() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("graveyard");
    let relative = PathBuf::from("2026").join("05").join("16").join("leaf");
    fs::create_dir_all(root.join(&relative)).unwrap();

    let resolved = contained_grave_dir(&root, &relative).unwrap();
    let canonical_root = root.canonicalize().unwrap();
    assert!(resolved.starts_with(&canonical_root));
    assert_ne!(resolved, canonical_root);
}

#[test]
fn missing_relative_grave_path_is_still_contained() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("graveyard");
    fs::create_dir(&root).unwrap();
    let relative = PathBuf::from("2026").join("05").join("16").join("missing");

    let resolved = contained_grave_dir(&root, &relative).unwrap();
    let canonical_root = root.canonicalize().unwrap();
    assert!(resolved.starts_with(&canonical_root));
    assert_ne!(resolved, canonical_root);
}

#[test]
fn absolute_parent_empty_and_root_grave_paths_are_rejected() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("graveyard");
    let outside = temp.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();

    assert_escapes(&root, &outside);
    assert_escapes(&root, Path::new(".."));
    assert_escapes(&root, Path::new("../outside"));
    assert_escapes(&root, Path::new("2026/../../outside"));
    assert_escapes(&root, Path::new(""));
    assert_escapes(&root, Path::new("."));
}

#[test]
#[cfg(unix)]
fn symlink_escape_is_rejected_even_when_components_look_relative() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("graveyard");
    let outside = temp.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();

    assert_escapes(&root, Path::new("link"));
    assert_escapes(&root, Path::new("link/missing"));
}

#[test]
fn restore_rejects_escaped_grave_path_before_move() {
    let temp = TempDir::new().unwrap();
    let yard = Graveyard::open(temp.path().join("graveyard"));
    let original = temp.path().join("workspace").join("node_modules");
    let grave = bury_sample(&yard, &original);

    let outside = temp.path().join("outside");
    fs::create_dir_all(outside.join("payload")).unwrap();
    fs::write(outside.join("payload").join("secret"), b"keep").unwrap();

    for grave_path in [
        outside.clone(),
        PathBuf::from("..").join("outside"),
        PathBuf::from("2026").join("..").join("..").join("outside"),
    ] {
        set_record_paths(&yard, &grave.record.id, grave_path.clone(), false);
        assert_escape_err(
            yard.restore_by_id(&grave.record.id, None)
                .expect_err("restore must fail closed on escaped grave_path"),
            &grave_path,
        );
        assert_eq!(
            fs::read(outside.join("payload").join("secret")).unwrap(),
            b"keep"
        );
        assert!(!original.exists(), "escaped restore must not create target");
        assert_eq!(yard.list().unwrap().len(), 1);
    }
}

#[test]
fn gc_rejects_escaped_grave_path_before_delete() {
    let temp = TempDir::new().unwrap();
    let yard = Graveyard::open(temp.path().join("graveyard"));
    let original = temp.path().join("workspace").join("node_modules");
    let grave = bury_sample(&yard, &original);

    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"secret").unwrap();

    for grave_path in [
        outside.clone(),
        PathBuf::from("..").join("outside"),
        PathBuf::from("2026").join("..").join("..").join("outside"),
    ] {
        set_record_paths(&yard, &grave.record.id, grave_path.clone(), true);
        assert_escape_err(
            yard.gc(true)
                .expect_err("gc dry-run must fail closed on escaped grave_path"),
            &grave_path,
        );
        assert_escape_err(
            yard.gc(false)
                .expect_err("gc must fail closed on escaped grave_path"),
            &grave_path,
        );
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"secret");
        assert_eq!(yard.list().unwrap().len(), 1);
    }
}

#[test]
fn gc_still_collects_contained_expired_graves() {
    let temp = TempDir::new().unwrap();
    let yard = Graveyard::open(temp.path().join("graveyard"));
    let original = temp.path().join("workspace").join("node_modules");
    let grave = bury_sample(&yard, &original);
    let grave_dir = yard.root().join(&grave.record.grave_path);
    set_record_paths(
        &yard,
        &grave.record.id,
        grave.record.grave_path.clone(),
        true,
    );

    let collected = yard.gc(false).unwrap();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].id, grave.record.id);
    assert!(!grave_dir.exists());
    assert!(yard.list().unwrap().is_empty());
}

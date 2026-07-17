use super::*;
use crate::model::Category;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn draft(path: PathBuf, safety: Safety) -> CandidateDraft {
    CandidateDraft {
        path,
        name: "target".to_string(),
        rule_id: "rust.target".to_string(),
        category: Category::Build,
        safety,
        reasons: Vec::new(),
        warnings: Vec::new(),
        restore_hint: "cargo build".to_string(),
    }
}

#[test]
fn source_size_index_rolls_up_descendants_without_sibling_contamination() {
    let project_a = PathBuf::from("workspace/project_a");
    let project_b = PathBuf::from("workspace/project_b");
    let mut sizes = DirSizes::new();
    sizes.insert(project_a.join("src"), 10);
    sizes.insert(project_a.join("tests"), 5);
    sizes.insert(project_b.join("src"), 7);

    let index = SourceSizeIndex::from_dir_sizes(&sizes);

    assert_eq!(index.bytes_under(&project_a), 15);
    assert_eq!(index.bytes_under(&project_b), 7);
    assert_eq!(index.bytes_under(Path::new("workspace")), 22);
    assert_eq!(index.bytes_under(Path::new("workspace/missing")), 0);
}

#[test]
fn source_size_index_rolls_up_nested_and_sibling_dirs() {
    let project = PathBuf::from("workspace/app");
    let sibling = PathBuf::from("workspace/other");
    let mut sizes = DirSizes::new();
    sizes.insert(project.join("src"), 2);
    sizes.insert(project.join("src/routes"), 3);
    sizes.insert(project.join("src/routes/api"), 5);
    sizes.insert(project.join("assets/images"), 7);
    sizes.insert(sibling.join("src"), 11);

    let index = SourceSizeIndex::from_dir_sizes(&sizes);

    assert_eq!(index.bytes_under(&project), 17);
    assert_eq!(index.bytes_under(&project.join("src")), 10);
    assert_eq!(index.bytes_under(&project.join("src/routes")), 8);
    assert_eq!(index.bytes_under(&project.join("assets")), 7);
    assert_eq!(index.bytes_under(&sibling), 11);
    assert_eq!(index.bytes_under(Path::new("workspace")), 28);
    assert_eq!(index.bytes_under(Path::new("workspace/app/src/views")), 0);
}

#[test]
fn parallel_walk_matches_serial_walk_for_nested_tree() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("root.bin"), [0; 3]).unwrap();
    fs::create_dir_all(temp.path().join("a/b")).unwrap();
    fs::write(temp.path().join("a/b/leaf.bin"), [0; 5]).unwrap();
    fs::create_dir(temp.path().join("c")).unwrap();
    fs::write(temp.path().join("c/leaf.bin"), [0; 7]).unwrap();

    let parallel = dir_size_walk_parallel(temp.path());
    let serial = dir_size_walkdir(temp.path());

    assert_eq!(parallel.bytes, serial.bytes);
    assert_eq!(parallel.bytes, 15);
    assert!(parallel.warnings.is_empty());
    assert!(serial.warnings.is_empty());
}

#[test]
fn missing_candidate_root_returns_metadata_warning() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("missing-target");

    let outcome = dir_size(&missing, false);

    assert_eq!(outcome.bytes, 0);
    assert_eq!(outcome.warnings.len(), 1);
    assert!(matches!(
        &outcome.warnings[0],
        ScanWarning::MetadataError { path, .. } if path == &missing
    ));
}

#[test]
fn blocked_candidate_is_not_sized_or_warned() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("blocked-target");
    let drafts = vec![draft(missing, Safety::Blocked)];
    let source_sizes = SourceSizeIndex::from_dir_sizes(&DirSizes::new());

    let summary = summarize(temp.path(), &drafts, &source_sizes, false);

    assert_eq!(summary.candidate_bytes, vec![0]);
    assert!(summary.warnings.is_empty());
}

#[test]
fn multi_root_walk_preserves_readable_bytes_and_stable_warnings() {
    let temp = TempDir::new().unwrap();
    let readable = temp.path().join("readable");
    let missing_a = temp.path().join("missing-a");
    let missing_b = temp.path().join("missing-b");
    fs::create_dir(&readable).unwrap();
    fs::write(readable.join("kept.bin"), [0; 11]).unwrap();
    let roots = vec![missing_b.clone(), readable, missing_a.clone()];

    let expected = dir_size_roots(&roots);
    assert_eq!(expected.bytes, 11);
    assert_eq!(expected.warnings.len(), 2);
    assert!(matches!(
        &expected.warnings[0],
        ScanWarning::WalkError { path: Some(path), .. } if path == &missing_a
    ));
    assert!(matches!(
        &expected.warnings[1],
        ScanWarning::WalkError { path: Some(path), .. } if path == &missing_b
    ));

    for _ in 0..10 {
        assert_eq!(dir_size_roots(&roots), expected);
    }
}

#[cfg(unix)]
#[test]
fn parallel_walk_preserves_partial_bytes_and_sorts_permission_warnings() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("kept.bin"), [0; 7]).unwrap();
    let denied_a = temp.path().join("denied-a");
    let denied_b = temp.path().join("denied-b");
    fs::create_dir(&denied_a).unwrap();
    fs::create_dir(&denied_b).unwrap();

    let original_a = fs::metadata(&denied_a).unwrap().permissions().mode();
    let original_b = fs::metadata(&denied_b).unwrap().permissions().mode();
    let mut denied_permissions = fs::metadata(&denied_a).unwrap().permissions();
    denied_permissions.set_mode(0o000);
    fs::set_permissions(&denied_a, denied_permissions.clone()).unwrap();
    fs::set_permissions(&denied_b, denied_permissions).unwrap();

    let outcomes: Vec<SizeOutcome> = (0..10)
        .map(|_| dir_size_walk_parallel(temp.path()))
        .collect();

    let mut permissions_a = fs::metadata(&denied_a).unwrap().permissions();
    permissions_a.set_mode(original_a);
    fs::set_permissions(&denied_a, permissions_a).unwrap();
    let mut permissions_b = fs::metadata(&denied_b).unwrap().permissions();
    permissions_b.set_mode(original_b);
    fs::set_permissions(&denied_b, permissions_b).unwrap();

    let expected = &outcomes[0];
    assert_eq!(expected.bytes, 7);
    assert_eq!(expected.warnings.len(), 2);
    assert!(matches!(
        &expected.warnings[0],
        ScanWarning::WalkError { path: Some(path), .. } if path == &denied_a
    ));
    assert!(matches!(
        &expected.warnings[1],
        ScanWarning::WalkError { path: Some(path), .. } if path == &denied_b
    ));
    assert!(outcomes.iter().all(|outcome| outcome == expected));
}

#[test]
fn saturating_atomic_add_caps_at_u64_max() {
    let total = AtomicU64::new(u64::MAX - 2);

    saturating_atomic_add(&total, 5);
    assert_eq!(total.load(Ordering::Relaxed), u64::MAX);

    saturating_atomic_add(&total, 1);
    assert_eq!(total.load(Ordering::Relaxed), u64::MAX);
}

#[test]
fn wide_directory_uses_parallel_root_and_counts_all_files() {
    let temp = TempDir::new().unwrap();
    for i in 0..=PARALLEL_DIRECT_ENTRY_THRESHOLD {
        fs::write(temp.path().join(format!("file_{i:04}.bin")), [0; 2]).unwrap();
    }

    let partition = partition_parallel_roots(temp.path());

    assert_eq!(partition.outcome.bytes, 0);
    assert!(partition.outcome.warnings.is_empty());
    assert_eq!(partition.roots, vec![temp.path().to_path_buf()]);
    let outcome = dir_size(temp.path(), false);
    assert_eq!(
        outcome.bytes,
        ((PARALLEL_DIRECT_ENTRY_THRESHOLD + 1) * 2) as u64
    );
    assert!(outcome.warnings.is_empty());
}

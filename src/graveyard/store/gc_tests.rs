use super::*;
use tempfile::TempDir;

#[test]
fn gc_delete_failure_keeps_failed_records_and_collects_successes() {
    let temp = TempDir::new().unwrap();
    let yard = Graveyard::open(temp.path().join("graveyard"));
    let mut records = Vec::new();
    for name in ["failed", "collected", "missing", "failed_second", "alive"] {
        let original = temp.path().join(name);
        fs::create_dir(&original).unwrap();
        fs::write(original.join("blob"), b"abc").unwrap();
        let grave = yard.bury(make_input(&original)).unwrap();
        let mut record = grave.record;
        if name != "alive" {
            record.expires_at = Utc::now() - chrono::Duration::days(1);
        }
        records.push(record);
    }
    rewrite_manifest_atomic(yard.root(), &records).unwrap();
    let failed_dir = yard.root().join(&records[0].grave_path);
    let collected_dir = yard.root().join(&records[1].grave_path);
    let missing_dir = yard.root().join(&records[2].grave_path);
    let failed_second_dir = yard.root().join(&records[3].grave_path);
    let alive_dir = yard.root().join(&records[4].grave_path);
    fs::remove_dir_all(&failed_dir).unwrap();
    fs::write(&failed_dir, b"keep").unwrap();
    let expected_kind = fs::remove_dir_all(&failed_dir).unwrap_err().kind();
    fs::remove_dir_all(&missing_dir).unwrap();
    fs::remove_dir_all(&failed_second_dir).unwrap();
    fs::write(&failed_second_dir, b"keep second").unwrap();
    let before = fs::read(yard.root().join("manifest.jsonl")).unwrap();

    assert_eq!(yard.gc(true).unwrap().len(), 4);
    assert_eq!(
        fs::read(yard.root().join("manifest.jsonl")).unwrap(),
        before
    );
    assert!(collected_dir.is_dir());

    match yard.gc(false).expect_err("failed deletion must fail gc") {
        GraveyardError::Io { path, source } => {
            assert_eq!(path, failed_dir.canonicalize().unwrap());
            assert_eq!(source.kind(), expected_kind);
        }
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(fs::read(&failed_dir).unwrap(), b"keep");
    assert_eq!(fs::read(&failed_second_dir).unwrap(), b"keep second");
    assert!(!collected_dir.exists());
    assert_eq!(fs::read(alive_dir.join("payload/blob")).unwrap(), b"abc");
    let remaining = yard.list().unwrap();
    assert_eq!(remaining.len(), 3);
    for record in [&records[0], &records[3], &records[4]] {
        let kept = remaining.iter().find(|kept| kept.id == record.id).unwrap();
        assert_eq!(
            serde_json::to_value(kept).unwrap(),
            serde_json::to_value(record).unwrap()
        );
    }

    // Once the filesystem failures are repaired, both records can be retried.
    fs::remove_file(&failed_dir).unwrap();
    fs::create_dir(&failed_dir).unwrap();
    fs::remove_file(&failed_second_dir).unwrap();
    fs::create_dir(&failed_second_dir).unwrap();
    assert_eq!(yard.gc(false).unwrap().len(), 2);
    assert_eq!(yard.list().unwrap()[0].id, records[4].id);
}

#[test]
fn gc_retries_after_manifest_rewrite_failure() {
    let temp = TempDir::new().unwrap();
    let yard = Graveyard::open(temp.path().join("graveyard"));
    let original = temp.path().join("node_modules");
    fs::create_dir(&original).unwrap();
    fs::write(original.join("blob"), b"abc").unwrap();
    let mut record = yard.bury(make_input(&original)).unwrap().record;
    record.expires_at = Utc::now() - chrono::Duration::days(1);
    rewrite_manifest_atomic(yard.root(), &[record.clone()]).unwrap();
    let manifest = yard.root().join("manifest.jsonl");
    let before = fs::read(&manifest).unwrap();
    let lock = yard.root().join("manifest.jsonl.lock");
    fs::write(&lock, b"contended").unwrap();

    assert!(matches!(
        yard.gc(false),
        Err(GraveyardError::ManifestLockContention { attempts: 5 })
    ));
    assert!(!yard.root().join(&record.grave_path).exists());
    assert_eq!(fs::read(&manifest).unwrap(), before);
    fs::remove_file(lock).unwrap();

    let collected = yard.gc(false).expect("retry must collect an absent grave");
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].id, record.id);
    assert!(yard.list().unwrap().is_empty());
}

#[test]
fn gc_keeps_record_when_not_found_is_only_a_deleted_descendant() {
    let temp = TempDir::new().unwrap();
    let yard = Graveyard::open(temp.path().join("graveyard"));
    let original = temp.path().join("node_modules");
    fs::create_dir(&original).unwrap();
    fs::write(original.join("blob"), b"keep").unwrap();
    fs::write(original.join("disappearing"), b"gone").unwrap();
    let mut record = yard.bury(make_input(&original)).unwrap().record;
    record.expires_at = Utc::now() - chrono::Duration::days(1);
    rewrite_manifest_atomic(yard.root(), &[record.clone()]).unwrap();
    let grave_dir = yard.root().join(&record.grave_path);

    let result = yard.gc_with_remover(false, |path| {
        let descendant = path.join("payload/disappearing");
        fs::remove_file(&descendant)?;
        fs::remove_file(descendant)
    });

    match result.expect_err("a missing descendant must not orphan surviving data") {
        GraveyardError::Io { path, source } => {
            assert_eq!(path, grave_dir.canonicalize().unwrap());
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(fs::read(grave_dir.join("payload/blob")).unwrap(), b"keep");
    assert!(!grave_dir.join("payload/disappearing").exists());
    assert_eq!(
        serde_json::to_value(&yard.list().unwrap()[0]).unwrap(),
        serde_json::to_value(&record).unwrap()
    );
    assert_eq!(yard.gc(false).unwrap().len(), 1);
    assert!(yard.list().unwrap().is_empty());
    assert!(!grave_dir.exists());
}

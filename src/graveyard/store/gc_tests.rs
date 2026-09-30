use super::*;
use tempfile::TempDir;

#[test]
fn gc_delete_failure_keeps_failed_records_and_collects_successes() {
    let temp = TempDir::new().unwrap();
    let yard = Graveyard::open(temp.path().join("graveyard"));
    let mut records = Vec::new();
    for name in ["failed", "collected", "missing", "alive"] {
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
    let alive_dir = yard.root().join(&records[3].grave_path);
    fs::remove_dir_all(&failed_dir).unwrap();
    fs::write(&failed_dir, b"keep").unwrap();
    let expected_kind = fs::remove_dir_all(&failed_dir).unwrap_err().kind();
    fs::remove_dir_all(&missing_dir).unwrap();
    let before = fs::read(yard.root().join("manifest.jsonl")).unwrap();

    assert_eq!(yard.gc(true).unwrap().len(), 3);
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
    assert!(!collected_dir.exists());
    assert_eq!(fs::read(alive_dir.join("payload/blob")).unwrap(), b"abc");
    let remaining = yard.list().unwrap();
    assert_eq!(remaining.len(), 3);
    for record in [&records[0], &records[2], &records[3]] {
        let kept = remaining.iter().find(|kept| kept.id == record.id).unwrap();
        assert_eq!(
            serde_json::to_value(kept).unwrap(),
            serde_json::to_value(record).unwrap()
        );
    }

    // Once the filesystem failures are repaired, both records can be retried.
    fs::remove_file(&failed_dir).unwrap();
    fs::create_dir(&failed_dir).unwrap();
    fs::create_dir(&missing_dir).unwrap();
    assert_eq!(yard.gc(false).unwrap().len(), 2);
    assert_eq!(yard.list().unwrap()[0].id, records[3].id);
}

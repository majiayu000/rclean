use std::panic::{self, AssertUnwindSafe};

use super::*;

#[test]
fn into_inner_reports_poisoned_scratch_without_panicking() {
    let scratch = WalkScratch::new();
    let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = match scratch.sizes.lock() {
            Ok(guard) => guard,
            Err(err) => panic!("unexpected pre-existing poison: {err}"),
        };
        panic!("poison sizes");
    }));
    assert!(poison_result.is_err());

    let err = match scratch.into_inner() {
        Ok(_) => panic!("poisoned scratch must error"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("walk scratch sizes mutex poisoned")
    );
}

#[test]
fn local_drop_marks_poisoned_scratch_without_panicking() {
    let scratch = WalkScratch::new();
    let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = match scratch.sizes.lock() {
            Ok(guard) => guard,
            Err(err) => panic!("unexpected pre-existing poison: {err}"),
        };
        panic!("poison sizes");
    }));
    assert!(poison_result.is_err());

    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut local = WalkLocal::new(&scratch);
        local.add_file_size(Path::new("/tmp/project"), 7);
    }));

    assert!(result.is_ok());
    assert!(scratch.is_poisoned());
    let err = match scratch.into_inner() {
        Ok(_) => panic!("poisoned scratch must error"),
        Err(err) => err,
    };
    assert!(
        err.to_string()
            .contains("walk scratch accumulator poisoned")
    );
}

#[test]
fn into_inner_reports_poisoned_drafts_without_panicking() {
    let scratch = WalkScratch::new();
    let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = match scratch.drafts_by_project.lock() {
            Ok(guard) => guard,
            Err(err) => panic!("unexpected pre-existing poison: {err}"),
        };
        panic!("poison drafts");
    }));
    assert!(poison_result.is_err());

    let err = match scratch.into_inner() {
        Ok(_) => panic!("poisoned scratch must error"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("walk scratch drafts mutex poisoned")
    );
}

#[test]
fn local_drop_marks_poisoned_drafts_without_panicking() {
    let scratch = WalkScratch::new();
    let poison_result = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = match scratch.drafts_by_project.lock() {
            Ok(guard) => guard,
            Err(err) => panic!("unexpected pre-existing poison: {err}"),
        };
        panic!("poison drafts");
    }));
    assert!(poison_result.is_err());

    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut local = WalkLocal::new(&scratch);
        let draft = CandidateDraft {
            path: PathBuf::from("/tmp/project/node_modules"),
            name: "node_modules".to_string(),
            rule_id: "node.node_modules".to_string(),
            category: crate::model::Category::Deps,
            safety: crate::model::Safety::Safe,
            reasons: Vec::new(),
            warnings: Vec::new(),
            restore_hint: "reinstall dependencies".to_string(),
        };
        local.add_draft(Path::new("/tmp/project"), draft);
    }));

    assert!(result.is_ok());
    assert!(scratch.is_poisoned());
    let err = match scratch.into_inner() {
        Ok(_) => panic!("poisoned scratch must error"),
        Err(err) => err,
    };
    assert!(
        err.to_string()
            .contains("walk scratch accumulator poisoned")
    );
}

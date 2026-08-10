use super::*;
use crate::model::{ActivityInfo, GitInfo, ProjectReport, Summary};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn fixture_report() -> ScanReport {
    ScanReport {
        schema_version: 1,
        tool_version: "test".to_string(),
        scanned_at: "2026-07-03T00:00:00Z".to_string(),
        roots: vec!["/tmp/root".to_string()],
        disk_attribution: None,
        warnings: Vec::new(),
        stale_after_days: 30,
        summary: Summary {
            projects_scanned: 1,
            projects_with_candidates: 1,
            candidates: 1,
            safe_candidates: 1,
            caution_candidates: 0,
            blocked_candidates: 0,
            report_only_candidates: 0,
            total_bytes: 42,
        },
        projects: vec![ProjectReport {
            path: "/tmp/root/app".to_string(),
            kind: "Node.js".to_string(),
            markers: vec!["package.json".to_string()],
            git: Some(GitInfo {
                repo_root: "/tmp/root/app".to_string(),
                dirty: true,
            }),
            activity: ActivityInfo {
                last_modified: "2026-07-01T00:00:00Z".to_string(),
                source: "test".to_string(),
            },
            candidates: vec![Candidate {
                path: "/tmp/root/app/node_modules".to_string(),
                name: "node_modules".to_string(),
                rule_id: "node.node_modules".to_string(),
                category: Category::Deps,
                bytes: 42,
                safety: Safety::Safe,
                requires_sudo: false,
                reasons: vec!["package.json present".to_string()],
                warnings: Vec::new(),
                restore_hint: "npm install".to_string(),
                risk_score: 0.2,
                staleness_days: Some(94),
            }],
            total_bytes: 42,
            project_bytes: 100,
            artifact_percent: 42.0,
        }],
    }
}

fn selector_report() -> ScanReport {
    let mut report = fixture_report();
    let candidates = &mut report.projects[0].candidates;
    candidates[0].bytes = 100;
    candidates[0].risk_score = 0.5;
    candidates[0].staleness_days = Some(10);
    candidates.extend([
        test_candidate("target", Category::Build, 300, 0.4, 20),
        test_candidate("pnpm-store", Category::Cache, 200, 0.1, 30),
        test_candidate("coverage", Category::Test, 50, 0.8, 40),
    ]);
    report
}

fn test_candidate(name: &str, category: Category, bytes: u64, risk: f32, stale: u64) -> Candidate {
    Candidate {
        path: format!("/tmp/root/app/{name}"),
        name: name.to_string(),
        rule_id: format!("test.{name}"),
        category,
        bytes,
        safety: Safety::Safe,
        requires_sudo: false,
        reasons: vec!["test fixture".to_string()],
        warnings: Vec::new(),
        restore_hint: "rebuild".to_string(),
        risk_score: risk,
        staleness_days: Some(stale),
    }
}

fn visible_labels(app: &SelectorApp) -> Vec<&str> {
    app.filtered
        .iter()
        .map(|index| app.rows[*index].label.as_str())
        .collect()
}

#[test]
fn question_mark_toggles_explain_pane() {
    let report = fixture_report();
    let mut app = SelectorApp::new(&report);
    assert!(!app.explain_open);

    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.explain_open);
    assert!(!app.cancelled);

    app.handle_key(key(KeyCode::Char('?')));
    assert!(!app.explain_open);
    assert!(!app.cancelled);
}

#[test]
fn esc_closes_explain_pane_before_cancelling() {
    let report = fixture_report();
    let mut app = SelectorApp::new(&report);

    app.handle_key(key(KeyCode::Char('?')));
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.explain_open);
    assert!(!app.cancelled, "first esc must only close the pane");

    app.handle_key(key(KeyCode::Esc));
    assert!(app.cancelled, "second esc cancels the selector");
}

#[test]
fn explain_detail_matches_explain_content_for_highlighted_candidate() {
    let report = fixture_report();
    let app = SelectorApp::new(&report);
    let detail = app.explain_detail();

    assert!(detail.contains("node.node_modules"));
    assert!(detail.contains("Safe"));
    assert!(detail.contains("package.json"));
    assert!(detail.contains("dirty worktree"));
    assert!(detail.contains("package.json present"));
    assert!(detail.contains("npm install"));
    assert!(detail.contains("last modified 2026-07-01T00:00:00Z"));
}

#[test]
fn preselects_matching_safe_candidate_by_path() {
    let report = fixture_report();
    let mut preselected = BTreeSet::new();
    preselected.insert(PathBuf::from("/tmp/root/app/node_modules"));

    let app =
        SelectorApp::new_with_preselected(&report, &preselected, SelectionNextStep::ConfirmCleanup);

    assert_eq!(app.selected.len(), 1);
    let selected = app.selected_candidates();
    assert_eq!(
        selected[0].path,
        PathBuf::from("/tmp/root/app/node_modules")
    );
}

#[test]
fn selector_sort_cycles_modes_and_orders_rows() {
    let report = selector_report();
    let mut app = SelectorApp::new(&report);
    assert_eq!(
        visible_labels(&app),
        ["target", "pnpm-store", "node_modules", "coverage"]
    );
    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(
        visible_labels(&app),
        ["coverage", "pnpm-store", "target", "node_modules"]
    );
    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(
        visible_labels(&app),
        ["pnpm-store", "target", "node_modules", "coverage"]
    );
}

#[test]
fn selector_filter_cycles_and_composes_with_search() {
    let report = selector_report();
    let mut app = SelectorApp::new(&report);
    app.handle_key(key(KeyCode::Char('c')));
    assert_eq!(visible_labels(&app), ["node_modules"]);
    app.category_filter = CategoryFilter::Cache;
    app.query = "pnpm".to_string();
    app.apply_filter();
    assert_eq!(visible_labels(&app), ["pnpm-store"]);
    app.query = "node".to_string();
    app.apply_filter();
    assert!(visible_labels(&app).is_empty());
}

#[test]
fn selector_header_shows_sort_and_filter() {
    let report = selector_report();
    let mut app = SelectorApp::new(&report);
    assert!(app.header().contains("Sort: size desc"));
    assert!(app.header().contains("Filter: all"));
    app.handle_key(key(KeyCode::Char('s')));
    app.handle_key(key(KeyCode::Char('c')));
    assert!(app.header().contains("Sort: stale desc"));
    assert!(app.header().contains("Filter: deps"));
}

#[test]
fn compact_summary_and_controls_keep_critical_actions_within_80_columns() {
    let report = selector_report();
    let app = SelectorApp::new(&report);
    let summary = app.header();
    let summary_lines = summary.lines().collect::<Vec<_>>();
    assert_eq!(summary_lines.len(), 2);
    assert!(summary_lines[1].contains("Reclaim"));
    assert!(summary_lines[1].contains("Selected"));
    assert!(display_width(summary_lines[1]) <= 78);

    let controls = app.controls();
    let control_lines = controls.lines().collect::<Vec<_>>();
    assert_eq!(control_lines.len(), 2);
    for label in [
        "[space] toggle",
        "[?] explain",
        "[enter] review",
        "[q] quit",
    ] {
        assert!(control_lines[0].contains(label));
    }
    assert!(control_lines.iter().all(|line| display_width(line) <= 78));
}

#[test]
fn standalone_tui_controls_describe_action_plan_without_cleanup() {
    let report = selector_report();
    let app = SelectorApp::new_with_preselected(
        &report,
        &BTreeSet::new(),
        SelectionNextStep::WriteActionPlan,
    );

    let controls = app.controls();
    assert!(controls.contains("[enter] plan"));
    assert!(controls.contains("save ActionPlan; no cleanup"));
    assert!(!controls.contains("confirm follows"));
    assert!(controls.lines().all(|line| display_width(line) <= 78));
}

#[test]
fn candidate_row_is_labeled_and_keeps_the_distinguishing_path_tail() {
    let report = fixture_report();
    let app = SelectorApp::new(&report);
    assert!(CANDIDATE_COLUMNS.contains("Safety"));
    assert!(CANDIDATE_COLUMNS.contains("Stale"));
    assert!(display_width(CANDIDATE_COLUMNS) <= 78);

    let row = app.list_item_text(0, 76);
    assert!(row.contains("safe"));
    assert!(row.contains("node_modules"));
    assert!(row.ends_with("node_modules"));
    assert!(display_width(&row) <= 76);
}

#[test]
fn candidate_row_truncates_wide_unicode_by_terminal_columns() {
    let mut report = fixture_report();
    report.projects[0].candidates[0].name = "缓存缓存缓存缓存缓存缓存缓存缓存缓存".to_string();
    report.projects[0].candidates[0].path = "/tmp/root/项目/缓存/🧰工具/node_modules".to_string();
    let app = SelectorApp::new(&report);

    let row = app.list_item_text(0, 76);

    assert!(row.ends_with("node_modules"));
    assert!(display_width(&row) <= 76);

    for width in 0..=56 {
        let row = app.list_item_text(0, width);
        assert!(
            display_width(&row) <= width,
            "row width {} exceeds terminal width {width}",
            display_width(&row)
        );
    }
    assert!(app.list_item_text(0, 16).ends_with("node_modules"));
}

#[test]
fn quit_and_confirmed_empty_selection_are_distinct_outcomes() {
    let report = fixture_report();
    let mut cancelled = SelectorApp::new(&report);
    cancelled.handle_key(key(KeyCode::Char('q')));
    assert!(matches!(
        cancelled.selection_outcome(),
        SelectionOutcome::Cancelled
    ));

    let mut confirmed = SelectorApp::new(&report);
    confirmed.handle_key(key(KeyCode::Enter));
    assert!(matches!(
        confirmed.selection_outcome(),
        SelectionOutcome::Confirmed(selected) if selected.is_empty()
    ));
}

#[test]
fn selector_selection_stability_across_sort_and_filter() {
    let report = selector_report();
    let mut app = SelectorApp::new(&report);
    app.toggle_current();
    assert_eq!(
        app.selected_candidates()[0].path,
        PathBuf::from("/tmp/root/app/target")
    );
    app.handle_key(key(KeyCode::Char('s')));
    app.handle_key(key(KeyCode::Char('c')));
    assert_eq!(app.selected.len(), 1);
    assert_eq!(
        app.selected_candidates()[0].path,
        PathBuf::from("/tmp/root/app/target")
    );
    app.handle_key(key(KeyCode::Char('c')));
    assert_eq!(visible_labels(&app), ["target"]);
}

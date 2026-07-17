use crate::model::{ActivityInfo, Candidate, Category, ProjectReport, Safety, ScanReport, Summary};

pub(crate) fn ranking_candidate(
    name: &str,
    bytes: u64,
    safety: Safety,
    staleness_days: Option<u64>,
) -> Candidate {
    Candidate {
        path: format!("/tmp/proj/{name}"),
        name: name.to_string(),
        rule_id: "rust.target".to_string(),
        category: Category::Build,
        bytes,
        safety,
        requires_sudo: false,
        reasons: vec!["test".to_string()],
        warnings: Vec::new(),
        restore_hint: "cargo build".to_string(),
        risk_score: 0.1,
        staleness_days,
    }
}

pub(crate) fn ranking_report(candidates: Vec<Candidate>) -> ScanReport {
    ScanReport {
        schema_version: 1,
        tool_version: "test".to_string(),
        scanned_at: "2026-07-03T00:00:00Z".to_string(),
        roots: vec!["/tmp".to_string()],
        disk_attribution: None,
        warnings: Vec::new(),
        stale_after_days: 30,
        summary: Summary {
            projects_scanned: 1,
            projects_with_candidates: 1,
            candidates: candidates.len(),
            safe_candidates: candidates.len(),
            caution_candidates: 0,
            blocked_candidates: 0,
            report_only_candidates: 0,
            total_bytes: candidates.iter().map(|c| c.bytes).sum(),
        },
        projects: vec![ProjectReport {
            path: "/tmp/proj".to_string(),
            kind: "Rust".to_string(),
            markers: vec!["Cargo.toml".to_string()],
            git: None,
            activity: ActivityInfo {
                last_modified: "2026-05-01T00:00:00Z".to_string(),
                source: "test".to_string(),
            },
            total_bytes: candidates.iter().map(|c| c.bytes).sum(),
            project_bytes: 100,
            artifact_percent: 50.0,
            candidates,
        }],
    }
}

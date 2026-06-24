use std::path::PathBuf;

use crate::model::{Candidate, Category, Safety};

#[derive(Debug, Clone, Copy)]
pub(super) struct SelectableCandidate<'a> {
    pub(super) project_path: &'a str,
    pub(super) candidate: &'a Candidate,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "graveyard"), allow(dead_code))]
pub struct SelectedCandidate {
    pub id: Option<String>,
    pub path: PathBuf,
    pub bytes: u64,
    pub rule_id: String,
    pub category: Category,
    pub safety: Safety,
    pub requires_sudo: bool,
    pub risk_score: f32,
}

#[derive(Debug, Default)]
pub struct CleanResult {
    pub cleaned: Vec<SelectedCandidate>,
    pub failed: Vec<(SelectedCandidate, String)>,
}

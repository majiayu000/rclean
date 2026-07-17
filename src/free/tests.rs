use super::*;
use crate::test_support::{ranking_candidate, ranking_report};

#[test]
fn prefers_stale_candidates_over_larger_fresh_ones() {
    let report = ranking_report(vec![
        ranking_candidate("fresh-large", 3_000, Safety::Safe, Some(0)),
        ranking_candidate("stale-small", 2_000, Safety::Safe, Some(90)),
    ]);
    let proposal = select_for_target(&report, 1_500);
    assert_eq!(proposal.candidates.len(), 1);
    assert_eq!(proposal.candidates[0].candidate.name, "stale-small");
}

#[test]
fn never_selects_non_safe_candidates_even_when_target_unmet() {
    let report = ranking_report(vec![
        ranking_candidate("safe-small", 1_000, Safety::Safe, Some(10)),
        ranking_candidate("caution-huge", 100_000, Safety::Caution, Some(90)),
        ranking_candidate("blocked-huge", 100_000, Safety::Blocked, Some(90)),
        ranking_candidate("report-only-huge", 100_000, Safety::ReportOnly, Some(90)),
    ]);
    let proposal = select_for_target(&report, 50_000);
    assert_eq!(proposal.candidates.len(), 1);
    assert_eq!(proposal.candidates[0].candidate.name, "safe-small");
    assert!(proposal.total_bytes < 50_000);
}

#[test]
fn prunes_picks_the_target_can_spare() {
    let report = ranking_report(vec![
        ranking_candidate("oldest", 1_000, Safety::Safe, Some(90)),
        ranking_candidate("older", 4_000, Safety::Safe, Some(60)),
        ranking_candidate("old", 2_000, Safety::Safe, Some(40)),
    ]);
    // Greedy picks oldest(1000) + older(4000) = 5000 >= 4500, then
    // the prune drops nothing (5000 - 1000 < 4500, 5000 - 4000 < 4500).
    let proposal = select_for_target(&report, 4_500);
    assert_eq!(proposal.total_bytes, 5_000);
    // A smaller target lets the prune drop the low-ranked pick.
    let proposal = select_for_target(&report, 900);
    assert_eq!(proposal.candidates.len(), 1);
    assert_eq!(proposal.candidates[0].candidate.name, "oldest");
    assert_eq!(proposal.total_bytes, 1_000);
}

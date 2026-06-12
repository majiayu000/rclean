mod audit;
mod deletion;
mod output;
mod roots;
mod selection;
mod types;
mod validation;

#[cfg(test)]
mod tests;

pub use audit::{DeleteAuditLogger, validate_audit_log_path};
pub use deletion::delete_selected;
#[cfg(feature = "graveyard")]
pub use deletion::delete_selected_into_graveyard;
pub use output::{confirm_if_needed, print_clean_result, print_plan};
pub use roots::check_broad_roots;
pub use selection::select_candidates;
#[cfg(feature = "tui")]
pub use selection::select_interactively_text;
pub use types::SelectedCandidate;

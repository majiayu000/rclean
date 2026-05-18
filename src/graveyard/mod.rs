//! Recoverable soft-delete layer ("graveyard") for cleaned candidates.
//!
//! Architecture lives in `docs/specs/v0.1.x-roadmap.md` §4.7. This
//! module is the storage half: it can move a candidate directory to
//! the graveyard, append a manifest record, list active graves, and
//! restore one. CLI wiring (`clean --graveyard`, `rclean restore`,
//! `rclean graveyard list/gc`) lands in follow-up PRs.
//!
//! Everything here is feature-gated behind `graveyard` so a
//! `cargo install rclean-cli --no-default-features` install strips
//! the module entirely.
//!
//! `dead_code` + `unused_imports` are intentionally allowed at the
//! module level for the same reason — the public API surface is
//! reviewed in isolation here, then exercised by the follow-up CLI
//! integration PR. Inner-module tests (`#[cfg(test)] mod tests`)
//! exercise every public item so it doesn't drift from the spec
//! while waiting for CLI wiring.

#![allow(dead_code, unused_imports)]

mod error;
mod id;
mod manifest;
mod store;

pub use error::GraveyardError;
pub use manifest::{GraveId, ManifestReader, ManifestRecord, RecordWriter};
pub use store::{Grave, GraveInput, Graveyard};

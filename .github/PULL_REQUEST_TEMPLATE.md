## Summary

- TODO

## Safety and Scope

- [ ] This change keeps `scan` read-only and does not weaken blocked-path handling.
- [ ] This change does not make `clean --all` select `caution`, `blocked`, or `report-only` candidates by default.
- [ ] This change does not silently swallow errors that would hide missing data or wrong output.
- [ ] Not applicable: this PR is docs-only, CI-only, or otherwise does not touch cleanup behavior.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo build --release`
- [ ] Not applicable: explain why full Rust verification was not relevant.

## Intake

Related issue:

Notes for reviewers:

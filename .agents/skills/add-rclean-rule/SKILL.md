---
name: add-rclean-rule
description: Use when adding or changing a built-in rclean cleanup rule, global cache rule, doctor applicability output, or supported-ecosystem documentation.
---

# Add rclean Rule

Use this skill for new or changed cleanup rules.

## Preflight

1. Search existing issues, PRs, `docs/specs/`, `specs/`, `src/rules/`,
   `tests/rules.rs`, and README tables for the rule or ecosystem.
2. Confirm the GitHub issue number.
3. For new behavior, read or create:
   - `specs/GH<number>/product.md`
   - `specs/GH<number>/tech.md`
   - `specs/GH<number>/tasks.md`
4. Read `SECURITY.md` if the rule touches broad roots, symlinks, protected
   paths, user records, destructive delete, or blocked/caution classification.

## Rule Contract

Every rule change must define:

- stable rule ID
- exact path or marker evidence
- safety classification: `safe`, `caution`, or `blocked`
- why the artifact is rebuildable or why it is report-only
- restore hint
- platform gates
- positive test
- negative test proving the rule does not fire without required markers

Generic names such as `build`, `dist`, `out`, `target`, and `vendor` must never
classify by name alone.

## Implementation Map

- Rule code: `src/rules/`
- Rule registration: `src/rules/mod.rs` and `src/rules/catalog.rs`
- Applicability output: `src/doctor.rs`
- Home scan roots: search for `home_toolchain_paths`
- Tests: `tests/rules.rs` and platform-specific tests when needed
- User docs: README supported ecosystem or global cache table

## Verification

Run focused tests first, then the full gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```

## Stop Conditions

Stop for maintainer review before code if the proposed rule:

- might delete user-authored data
- requires shelling out to a native cleanup tool
- needs credentials, sudo, or system permissions
- touches protected agent/session/memory paths
- changes ActionPlan replay or delete selection semantics

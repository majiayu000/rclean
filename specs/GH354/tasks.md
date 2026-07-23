# Per-Candidate Staleness - Tasks

GitHub issue: `#354`
Product spec: `specs/GH354/product.md`
Tech spec: `specs/GH354/tech.md`

## Tasks

- [x] T1: Add `newest_mtime: Option<SystemTime>` to `SizeOutcome`;
      update `merge` to fold it by max.
- [x] T2: Capture `metadata.modified()` at the single-file branch of
      `dir_size`.
- [x] T3: Capture it in `partition_parallel_roots` (serial descent).
- [x] T4: Capture it in `dir_size_walkdir` (per-root walkdir).
- [x] T5: Capture it in `dir_size_walk_parallel` via an `AtomicU64`
      Unix-seconds compare-exchange max, mirroring
      `saturating_atomic_add`.
- [x] T6: Add `candidate_activity: Vec<Option<SystemTime>>` to
      `SizeSummary`, parallel to `candidate_bytes`.
- [x] T7: In `build_project_report`, compute `staleness_days` from the
      per-candidate activity, falling back to the project `activity_time`
      when the candidate was not walked.
- [x] T8: Leave `risk_score` and `ProjectReport.activity` on the
      project `activity_time`.
- [x] T9: Unit tests — newest mtime across serial, parallel, single-file,
      and empty paths; serial-vs-parallel equivalence.
- [x] T10: Integration test — old candidate under a busy parent reports
      its true age, not 0d (fails pre-change).
- [x] T11: No existing staleness assertions needed updating. The ~5
      `staleness_days` sites in `tui`/`plan`/`watch` tests set the field
      directly on synthetic `Candidate` fixtures; none computes it from
      the sizer path, so the semantic change does not touch them.
- [x] T12: CHANGELOG entry.
- [x] T13: Verification gate — `cargo fmt -- --check`, `cargo clippy
      --all-targets --all-features -- -D warnings`, `cargo test`, plus a
      manual `scan --home` confirming Yarn/ms-playwright now read their
      real age.

## Acceptance Mapping

| Acceptance criterion | Tasks |
| --- | --- |
| 1. Candidate age despite fresh sibling | T2-T7, T10 |
| 2. Single-file candidate | T2, T9 |
| 3. Unwalked candidate falls back | T7, T9 |
| 4. risk_score unchanged | T8 |
| 5. No extra walk | T1-T6 |
| 6. Deterministic tests | T9, T10 |

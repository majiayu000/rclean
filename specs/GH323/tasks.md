# GH323 Tasks

## Linked Artifacts

- Issue: `#323`
- Product: `specs/GH323/product.md`
- Tech: `specs/GH323/tech.md`
- Route after approval: `implement`

## Status

`planned` — implementation waits for merged GH323 Spec PR.

## SpecRail Checklist

- [ ] `SP323-T1` | Owner: `tests` | Done when: exactly three non-timeout fake Docker report invocations use bounded 30s headroom | Verify: exact source/word diff + focused tests
- [ ] `SP323-T2` | Owner: `verification` | Done when: sequential/concurrent stress and full gates pass without production or assertion changes | Verify: stress/stable/MSRV/VibeGuard/CI/PR gates

## SP323-T1 — Add bounded non-timeout fixture headroom

- Dependencies: merged Spec; latest main; unchanged three command sites
- Covers: B-001, B-002, B-003, B-004, B-005
- Change: add `--timeout 30s` only to permission-denied, success/report-only and oversized-output fake report
  commands; keep the dedicated timeout test at 1s.
- Done when: one-file diff contains only the three argument-pair additions; production 5s constant, scripts and
  assertions are unchanged.
- Verify:
  - exact source counts from tech spec
  - `git diff --word-diff=porcelain origin/main...HEAD -- tests/docker_report_cli.rs`
  - `cargo test --test docker_report_cli -- --nocapture`

## SP323-T2 — Prove concurrent reliability and merge readiness

- Dependencies: SP323-T1
- Covers: B-001, B-002, B-003, B-004, B-005, B-006, B-007
- Done when: focused 10 rounds and three 12-way stress rounds all pass; full local and final remote gates are green.
- Verify:
  - focused/stress loops from tech spec
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
  - `rustup run 1.95.0 cargo build --all-targets --all-features`
  - `rustup run 1.95.0 cargo test`
  - Rust/universal VibeGuard guards
  - SpecRail, signatures, current-head CI and reviewThreads

## Invariant Coverage Audit

- Product: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Tasks: `{B-001, B-002, B-003, B-004, B-005, B-006, B-007}`
- Missing: `none`

## Handoff Notes

- Implementation file only: `tests/docker_report_cli.rs`.
- No mutex, retry, sleep, skip, assertion edit or production change.
- Preserve timeout test at 1s and production default at 5s.
- Refresh exact sites if main drifts.
- Fresh gates and standing authorization required; never force push.

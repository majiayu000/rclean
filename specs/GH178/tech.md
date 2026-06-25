# Adopt SpecRail Agent Workflow - Tech Spec

Product spec: `specs/GH178/product.md`
Tasks: `specs/GH178/tasks.md`
GitHub issue: `#178`

## Context

- `README.md` documents user-facing behavior and supported cleanup rules.
- `CONTRIBUTING.md` documents the trust model, code conventions, and local
  verification commands.
- `SECURITY.md` defines private disclosure and trust-model vulnerability scope.
- `docs/specs/` contains historical roadmap/spec material.
- `.github/workflows/ci.yml` runs fmt, clippy, test, release build, and MSRV
  build/test.

Current gaps:

- No top-level `AGENTS.md`.
- No repo-local agent skills.
- Existing issue templates need SpecRail route/spec/acceptance fields.
- Existing PR template needs linked work, trust-model, and current verification
  fields.
- No `specs/GH<number>/` packet for new SpecRail-governed work.
- `CONTRIBUTING.md` still mentions MSRV `1.88` even though `Cargo.toml` and CI
  use Rust `1.95`.

## Proposed Changes

1. Add `AGENTS.md` as a short repository router.
   - Keep existing README/CONTRIBUTING/SECURITY as source documents.
   - Add a SpecRail route table for docs-only, bugfix, new rule, risky
     behavior, and security/private changes.
   - Record the real verification commands.
2. Add `.agents/skills/add-rclean-rule/SKILL.md`.
   - Scope it to built-in rule additions or behavior changes.
   - Include search/spec preflight, rule contract, implementation map, full
     verification, and stop conditions.
3. Extend GitHub templates.
   - `scan-false-positive.yml` captures optional linked spec evidence for
     substantial rule-behavior fixes.
   - `cleanup-safety-concern.yml` captures SpecRail route and linked spec for
     trust-model changes.
   - `feature-request.yml` captures SpecRail route, spec paths, and acceptance
     criteria.
   - `config.yml` disables blank issues and routes security reports to private
     vulnerability reporting.
   - `PULL_REQUEST_TEMPLATE.md` captures linked work, trust-model impact,
     verification, and reviewer notes.
4. Add `specs/GH178/` with `product.md`, `tech.md`, and `tasks.md`.
5. Update `CONTRIBUTING.md`.
   - Replace stale MSRV `1.88` with `1.95`.
   - Match `cargo fmt -- --check` and MSRV commands to CI.

## Validation

Docs/workflow checks:

```sh
test -s AGENTS.md
test -s .agents/skills/add-rclean-rule/SKILL.md
test -s .github/ISSUE_TEMPLATE/scan-false-positive.yml
test -s .github/ISSUE_TEMPLATE/cleanup-safety-concern.yml
test -s .github/ISSUE_TEMPLATE/feature-request.yml
test -s .github/PULL_REQUEST_TEMPLATE.md
test -s specs/GH178/product.md
test -s specs/GH178/tech.md
test -s specs/GH178/tasks.md
rg -n '1\\.88|cargo fmt --check' CONTRIBUTING.md AGENTS.md .github .agents
```

Repository gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```

## Risks

- Too much process could slow small docs fixes.
  - Mitigation: `AGENTS.md` explicitly allows direct docs-only changes.
- Duplicating existing docs could create drift.
  - Mitigation: `AGENTS.md` routes to README/CONTRIBUTING/SECURITY instead of
    restating detailed behavior.
- GitHub templates could expose security details in public issues.
  - Mitigation: `config.yml` points vulnerabilities to private reporting and
    the router requires maintainer review for private/security work.

## Follow-Ups

- Add automated SpecRail checks only after this manual adoption flow proves
  useful in real `rclean` PRs.
- Decide later whether historical `docs/specs/` should be indexed or migrated;
  this PR intentionally leaves them unchanged.

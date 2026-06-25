# GH178 Tasks

Issue: `#178`
Product spec: `specs/GH178/product.md`
Tech spec: `specs/GH178/tech.md`

## Status

- [x] `SP178-T001` Owner: `workflow` | Done when: `AGENTS.md` exists and routes agents to docs, SpecRail packet paths, trust-model gates, and real verification commands | Verify: `test -s AGENTS.md`
- [x] `SP178-T002` Owner: `skill` | Done when: `.agents/skills/add-rclean-rule/SKILL.md` defines rule preflight, rule contract, implementation map, verification, and stop conditions | Verify: `test -s .agents/skills/add-rclean-rule/SKILL.md`
- [x] `SP178-T003` Owner: `github` | Done when: existing issue templates and PR template include linked-spec, trust-model, route, acceptance, and verification fields | Verify: `test -s .github/ISSUE_TEMPLATE/scan-false-positive.yml && test -s .github/ISSUE_TEMPLATE/cleanup-safety-concern.yml && test -s .github/ISSUE_TEMPLATE/feature-request.yml && test -s .github/PULL_REQUEST_TEMPLATE.md`
- [x] `SP178-T004` Owner: `spec` | Done when: `specs/GH178/product.md`, `tech.md`, and `tasks.md` exist and link issue `#178` | Verify: `test -s specs/GH178/product.md && test -s specs/GH178/tech.md && test -s specs/GH178/tasks.md`
- [x] `SP178-T005` Owner: `docs` | Done when: CONTRIBUTING MSRV and local verification commands match `Cargo.toml` and CI | Verify: `rg -n '1\\.88|cargo fmt --check' CONTRIBUTING.md AGENTS.md .github .agents`
- [x] `SP178-T006` Owner: `verification` | Done when: docs/workflow checks and Rust gates pass for the adoption PR | Verify: `cargo fmt -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test && cargo build --release && rustup run 1.95 cargo build --all-targets --all-features && rustup run 1.95 cargo test`

## Handoff Notes

- This adoption is docs/workflow only; it must not modify runtime cleanup
  behavior.
- Historical `docs/specs/` stay in place.
- The untracked local `drafts/` directory is unrelated and should remain
  untouched.

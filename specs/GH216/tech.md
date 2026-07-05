# Post-Release Install Smoke Workflow - Tech Spec

Product spec: `specs/GH216/product.md`
Tasks: `specs/GH216/tasks.md`
GitHub issue: `#216`

## Context

- Release packaging is defined in `.github/workflows/release.yml`.
- Homebrew packaging uses `.github/homebrew/rclean.rb.tmpl`.
- `Cargo.toml` contains cargo-binstall metadata.
- The workflow is expected to be red until v0.2.0 release assets and the
  crates.io package are available.

## Proposed Changes

1. Add a workflow such as `.github/workflows/install-smoke.yml`.
2. Trigger on `workflow_dispatch` and a weekly `schedule`.
3. Add independent jobs:
   - binstall on Ubuntu and macOS
   - Homebrew on macOS
   - cargo install on Ubuntu
4. Run `rclean --version` after each install.
5. Keep the workflow off `pull_request`.

## Safety And Compatibility

- CI-only. No runtime cleanup behavior changes.
- Network installs should be isolated to GitHub-hosted runners.
- Expected red state before publication must be documented in the PR body.

## Validation

Focused:

```sh
test -s .github/workflows/install-smoke.yml
rg -n 'workflow_dispatch|schedule|cargo binstall|brew install|cargo install rclean-cli|rclean --version' .github/workflows/install-smoke.yml
! rg -n 'pull_request' .github/workflows/install-smoke.yml
```

Repository gate:

```sh
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

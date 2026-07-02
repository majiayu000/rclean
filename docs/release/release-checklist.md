# rclean Release Checklist

Package name: `rclean-cli`
Binary name: `rclean`

## Preflight

- [ ] Confirm `Cargo.toml` version.
- [ ] Update `CHANGELOG.md`.
- [ ] Run `cargo fmt -- --check`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] Run `cargo build --release`.
- [ ] Run `target/release/rclean scan . --min-size 0`.
- [ ] Run `target/release/rclean clean . --all --dry-run --min-size 0`.
- [ ] Refresh benchmark report under `docs/reports/`.

## Package

- [ ] Verify crates.io package name `rclean-cli` is available.
- [ ] Verify binary installs as `rclean`.
- [ ] Run `cargo package --list`.
- [ ] Run `cargo publish --dry-run`.

## Release ordering (tag -> workflow -> crates.io)

Version bumps happen only at release time (see the repo semver
policy); feature and fix PRs never touch the version.

1. Bump `Cargo.toml` version in a release PR, merge it.
2. Tag the merge commit: `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. The tag triggers `.github/workflows/release.yml`, which builds all
   five target triples (macOS arm64/x64, Linux x64/arm64, Windows x64),
   packages archives with per-artifact `.sha256` files and a combined
   `SHA256SUMS`, and creates a **draft** GitHub Release.
4. Review the draft: artifact count (5 archives + checksums), spot-check
   one checksum, paste the changelog section as release notes, publish.
5. Only after the GitHub Release is published, run `cargo publish`.
   This ordering keeps `cargo binstall rclean-cli` working the moment
   the crates.io version appears, because binstall resolves the
   archives from the already-published GitHub Release.
6. Verify from a clean environment:
   - `cargo install rclean-cli` (builds from crates.io)
   - `cargo binstall rclean-cli` (downloads the release artifacts)
   - `rclean --version` matches the tag.

`cargo publish` is irreversible (a version can be yanked but never
replaced) — it stays a human-run step.

## GitHub Repository Metadata

- [ ] Confirm issue forms render from **Issues** -> **New issue**:
      scan false positive, cleanup safety concern, and feature request.
- [ ] Confirm the PR template renders when opening a pull request.
- [ ] Add repository topics in the GitHub UI: repository page ->
      **About** gear icon -> **Topics** -> add `rust`, `cli`,
      `developer-tools`, `disk-cleanup`, `cache-cleaner`,
      `filesystem`, and `safe-delete` -> **Save changes**.

## Homebrew

- [ ] Create or update formula for `rclean`.
- [ ] Formula should install the `rclean` binary from GitHub Release.
- [ ] Test with `brew install --build-from-source`.

## Announcement

- [ ] README has first-run screenshot or GIF.
- [ ] README has `scan`, `clean --dry-run`, ActionPlan, and safety examples.
- [ ] Post includes real benchmark numbers.
- [ ] Post states the trust model: scan first, explain every candidate, blocked symlinks.

Current README demo asset: `docs/assets/rclean-demo.svg`.

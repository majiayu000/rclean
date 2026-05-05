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
- [ ] Tag release: `vX.Y.Z`.

## GitHub Release

- [ ] Build macOS arm64 binary.
- [ ] Build Linux x86_64 binary in CI.
- [ ] Upload compressed binaries.
- [ ] Include SHA256 checksums.
- [ ] Include safety model in release notes.
- [ ] Include install commands.

## Homebrew

- [ ] Create or update formula for `rclean`.
- [ ] Formula should install the `rclean` binary from GitHub Release.
- [ ] Test with `brew install --build-from-source`.

## Announcement

- [ ] README has first-run screenshot or GIF.
- [ ] README has `scan`, `clean --dry-run`, ActionPlan, and safety examples.
- [ ] Post includes real benchmark numbers.
- [ ] Post states the trust model: scan first, explain every candidate, blocked symlinks.

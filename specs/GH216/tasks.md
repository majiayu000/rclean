# GH216 Tasks

Issue: `#216`
Product spec: `specs/GH216/product.md`
Tech spec: `specs/GH216/tech.md`

## Status

- [ ] `SP216-T001` Owner: `ci` | Done when: install smoke workflow has manual and weekly triggers | Verify: `rg -n 'workflow_dispatch|schedule' .github/workflows/install-smoke.yml`
- [ ] `SP216-T002` Owner: `ci` | Done when: binstall job runs on Ubuntu and macOS and checks `rclean --version` | Verify: `rg -n 'cargo binstall rclean-cli --no-confirm|rclean --version' .github/workflows/install-smoke.yml`
- [ ] `SP216-T003` Owner: `ci` | Done when: Homebrew job runs on macOS and checks `rclean --version` | Verify: `rg -n 'brew install majiayu000/rclean/rclean|rclean --version' .github/workflows/install-smoke.yml`
- [ ] `SP216-T004` Owner: `ci` | Done when: cargo install job runs on Ubuntu and checks `rclean --version` | Verify: `rg -n 'cargo install rclean-cli --locked|rclean --version' .github/workflows/install-smoke.yml`
- [ ] `SP216-T005` Owner: `ci` | Done when: workflow does not run on pull requests | Verify: `! rg -n 'pull_request' .github/workflows/install-smoke.yml`

## Handoff Notes

- Red runs before release publication are expected and should be called out in
  PR review notes.

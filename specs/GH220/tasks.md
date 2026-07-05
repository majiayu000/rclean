# GH220 Tasks

Issue: `#220`
Product spec: `specs/GH220/product.md`
Tech spec: `specs/GH220/tech.md`

## Status

- [x] `SP220-T001` Owner: `docs` | Done when: README Current Status lists the v0.2 CLI surface | Verify: `rg -n 'free <size>|completions|man|stale' README.md`
- [x] `SP220-T002` Owner: `docs` | Done when: Usage includes no-arg, TUI, `free`, completions, and man examples | Verify: `rg -n 'rclean$|rclean tui|rclean free|rclean completions|rclean man' README.md`
- [x] `SP220-T003` Owner: `docs` | Done when: install docs include cargo install, cargo-binstall, and Homebrew paths | Verify: `rg -n 'cargo install rclean-cli|cargo binstall rclean-cli|brew install majiayu000/rclean/rclean' README.md`
- [x] `SP220-T004` Owner: `docs` | Done when: stale future-publication phrasing is removed | Verify: `! rg -n 'After public release' README.md`
- [x] `SP220-T005` Owner: `verification` | Done when: documented command examples are checked against CLI help | Verify: implementation PR lists checked commands

## Handoff Notes

- This is docs-only unless command drift reveals a CLI bug.

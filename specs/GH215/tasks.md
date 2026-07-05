# GH215 Tasks

Issue: `#215`
Product spec: `specs/GH215/product.md`
Tech spec: `specs/GH215/tech.md`

## Status

- [ ] `SP215-T001` Owner: `cli` | Done when: `free` accepts `--interactive` and rejects non-TTY use before deletion | Verify: `cargo test --test cli free_interactive`
- [ ] `SP215-T002` Owner: `selector` | Done when: selector state can receive stable pre-selected candidate identities | Verify: `cargo test free_interactive`
- [ ] `SP215-T003` Owner: `safety` | Done when: pre-selection excludes caution without opt-in, blocked, report-only, and sudo candidates | Verify: `cargo test free_interactive`
- [ ] `SP215-T004` Owner: `cleanup` | Done when: adjusted interactive selection uses existing confirmation and recoverable-delete flow | Verify: `cargo test --test cli free_interactive`
- [ ] `SP215-T005` Owner: `regression` | Done when: existing `free` plan path tests still pass unchanged | Verify: `cargo test --test cli free_`

## Handoff Notes

- Implement before GH218 because both modify selector state.
- Do not change ActionPlan schema or cleanup validation.

# Free ActionPlan Default Location - Tasks

GitHub issue: `#349`
Product spec: `specs/GH349/product.md`
Tech spec: `specs/GH349/tech.md`

## Tasks

- [x] T1: Add `src/plan/location.rs` with `default_plans_dir()`
      implementing the documented env-var precedence and last-resort
      fallback.
- [x] T2: Register and re-export the module in `src/plan.rs`.
- [x] T3: Unit tests in `src/plan/location.rs` covering `XDG_STATE_HOME`
      set, `XDG_STATE_HOME` empty, `LOCALAPPDATA`, `USERPROFILE`, `HOME`
      fallback, and no-home fallback, run serially in one test function.
- [x] T4: Point `default_free_plan_path()` at `plan::default_plans_dir()`.
- [x] T5: Create the parent directory for the default path only, in
      `free::run`; propagate failure as an error, no silent cwd
      fallback. `--write-plan` behavior deliberately unchanged.
- [x] T6: Integration test — `free` leaves the working directory clean.
- [x] T7: Integration test — plan lands under `$XDG_STATE_HOME` and both
      printed lines show the resolved path.
- [x] T8: Integration test — `--write-plan` still wins and does not
      populate the state directory.
- [x] T9: CHANGELOG entry noting the default-location change.
- [x] T10: Verification gate — `cargo fmt -- --check`,
      `cargo clippy --all-targets --all-features -- -D warnings`,
      `cargo test`, plus a manual `free` run confirming a clean cwd.

## Deviations From The Tech Spec

- The resolver landed in `src/plan/location.rs`, not a new top-level
  `src/paths.rs`. `src/plan/io.rs` already owns ActionPlan file IO, so
  the plan module is the cohesive home, mirroring how `graveyard` owns
  its own `default_root()`.
- Directory creation landed in `free::run`, not in
  `plan::io::write_atomically`. The writer cannot tell a tool-chosen
  path from a user-typed one, and
  `tests/cli/free_output.rs:free_json_plan_write_failure_leaves_stdout_empty`
  pins the existing contract that a `--write-plan` path with a missing
  parent must fail. Creating directories there would have silently
  weakened that contract.
- Integration tests extended `tests/cli/free_output.rs` rather than
  adding `tests/free_cli.rs`, which did not exist.

## Acceptance Mapping

| Acceptance criterion | Tasks |
| --- | --- |
| 1. No file in cwd | T4, T6 |
| 2. Plan valid and replayable | T5, T7 |
| 3. `$XDG_STATE_HOME` honored | T1, T3, T7 |
| 4. `--write-plan` overrides | T8 |
| 5. Printed paths match | T7 |
| 6. No-home environment works | T1, T3 |

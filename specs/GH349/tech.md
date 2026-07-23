# Free ActionPlan Default Location - Tech Spec

GitHub issue: `#349`
Product spec: `specs/GH349/product.md`

## Current Behavior

`src/free.rs:375-377`:

```rust
fn default_free_plan_path() -> PathBuf {
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    PathBuf::from(format!("rclean-free-{stamp}.json"))
}
```

A bare relative filename resolves against the process working
directory. The value flows into `plan::write_selected_action_plan`
(`src/free.rs:75-80`) and is printed twice by `print_human_proposal`
(`src/free.rs:135-139`).

## Design

Add `src/paths.rs` owning user-directory resolution for rclean's own
files, and resolve the default plan path through it.

```rust
/// `<state>/rclean/plans/`, resolved per platform:
///   Linux/macOS: $XDG_STATE_HOME/rclean/plans
///                $HOME/.local/state/rclean/plans
///   Windows:     %LOCALAPPDATA%\rclean\plans
///                %USERPROFILE%\AppData\Local\rclean\plans
/// Last resort:   ./.rclean-plans/ when no home env var is set.
pub fn default_plans_dir() -> PathBuf
```

The env-var precedence and the last-resort fallback deliberately mirror
`graveyard::default_root()` (`src/graveyard/mod.rs:44`) so the two agree
on which environment wins. The fallback is a namespaced
`./.rclean-plans/` directory rather than a bare relative filename: with
no home environment the plan still has to go somewhere relative, but a
single ignorable directory is not the loose-timestamped-file litter
this issue is about, and it matches the graveyard's
`./.rclean-graveyard` posture. Plans use the *state* directory rather than
the *data* directory: a plan is a regenerable proposal, whereas a grave
holds the only copy of deleted bytes.

`default_free_plan_path()` becomes:

```rust
fn default_free_plan_path() -> PathBuf {
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    paths::plans_dir().join(format!("rclean-free-{stamp}.json"))
}
```

### Directory creation

The state directory does not exist on first use, so it must be created
before `plan::write_selected_action_plan` runs. Creation is scoped to
the **default** path only, inside `free::run`:

- Default path (rclean chose it): create the parent, and surface a
  creation failure as an error — never fall back to the working
  directory, which would reintroduce exactly the behavior this change
  removes (AGENTS.md: no silent degradation).
- `--write-plan <PATH>` (the user chose it): unchanged. A missing
  parent still fails. `tests/cli/free_output.rs:267`
  (`free_json_plan_write_failure_leaves_stdout_empty`) pins this
  contract, and it is the right one: an explicit destination with a
  typo should surface rather than silently materialize a directory
  tree.

This is why creation lives in `free.rs` rather than in
`plan::io::write_atomically` — the writer cannot distinguish a path the
tool picked from a path the user typed.

### Why not the `dirs` crate

The repository already hand-rolls this chain in `graveyard::default_root`
without a dependency. Adding `dirs` for one function would contradict the
"standard library first" rule with no behavioral gain.

## Files Touched

| File | Change |
| --- | --- |
| `src/plan/location.rs` | New. `default_plans_dir()` plus unit tests for env precedence. |
| `src/plan.rs` | Register and re-export the `location` module. |
| `src/free.rs` | `default_free_plan_path()` resolves through `plan::default_plans_dir()`; create the parent directory for the default path only. |
| `tests/cli/free_output.rs` | Integration coverage for cwd cleanliness and `XDG_STATE_HOME`. |

The resolver lives under `src/plan/` rather than in a new top-level
module because `plan/io.rs` already owns ActionPlan file IO, mirroring
how `graveyard` owns its own `default_root()`. `src/path_util.rs` was
considered and rejected: it holds pure filename string helpers, and
mixing environment-dependent directory resolution into it would make a
pure module env-dependent.

## Test Plan

Unit (`src/paths.rs`):

- `XDG_STATE_HOME` set and non-empty wins.
- Empty `XDG_STATE_HOME` is ignored, falls through to `HOME`.
- `HOME` produces `.local/state/rclean/plans`.
- No home env vars set produces the relative last-resort path.

Env-var mutation is process-global, so these run serially within one
test function rather than as separate parallel tests.

Integration (`tests/free_cli.rs`), each with a temp `XDG_STATE_HOME`:

- `free` writes no file into the working directory (regression test for
  #349; fails against the pre-change binary).
- The plan lands under `$XDG_STATE_HOME/rclean/plans/` and both printed
  lines carry that path.
- `--write-plan <PATH>` still writes exactly to `<PATH>` and nothing to
  the state directory.

## Risks

- **Trust model:** none. Selection, safety classification, ActionPlan
  schema and replay validation are untouched; only the default output
  location of a file that is already written today changes.
- **Behavior change:** scripts that assumed the plan appears in the cwd
  break. `free` shipped in 0.2 and prints the resolved path on every
  run, and `--write-plan` gives an explicit stable location, so the fix
  is preferred over preserving the litter. Called out in CHANGELOG.

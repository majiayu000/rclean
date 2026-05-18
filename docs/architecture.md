# Architecture

This is the 30-second tour of `rclean` for new contributors. It maps
the four CLI commands to the code that runs them, and explains the
trust boundaries that govern every decision in the pipeline.

For the strategic direction and milestone planning, read
[`docs/specs/v0.1.x-roadmap.md`](specs/v0.1.x-roadmap.md). This
document is the *current* implementation snapshot.

## Module map

```
src/
├── main.rs          CLI entry. Parses args, sets up tracing, dispatches.
├── cli.rs           clap argument definitions for every subcommand.
├── error.rs         thiserror types: ScanError, PlanError, CleanError,
│                    ParseError, RcleanError (top-level union).
├── model.rs         Domain types: Safety, Category, Candidate,
│                    CandidateDraft, ScanReport, ActionPlan, Summary.
├── parse.rs         CLI value parsers: size strings ("100mb"),
│                    duration strings ("6m", "30d").
├── scan.rs          The scan phase. scan() + explain_path() entry points.
│                    Owns GitCache, IgnoreMatcher, ScanContext.
├── scan_tests.rs    Co-located tests for scan.rs.
├── user_rules.rs    .rclean.toml loader: globset-based custom rules.
├── rules/           Built-in classification rules, one file per ecosystem.
│   ├── mod.rs       Public surface: classify_candidate(), rule_catalog().
│   ├── catalog.rs   Per-ecosystem rule registration (the dispatch list).
│   ├── markers.rs   Shared marker-detection helpers (parent must contain X).
│   ├── node.rs      node_modules, .next, .turbo, .vite, .parcel-cache, ...
│   ├── python.rs    .venv, __pycache__, .pytest_cache, .ruff_cache, .tox, ...
│   ├── rust.rs      target (with Cargo.toml marker)
│   ├── go.rs        vendor
│   ├── jvm.rs       target, build, .gradle (with build.gradle / pom.xml)
│   ├── flutter.rs   build, .dart_tool (with pubspec.yaml)
│   ├── dotnet.rs    bin, obj
│   ├── ruby.rs      .bundle, vendor/bundle
│   ├── ios.rs       Pods
│   └── generic.rs   build, dist, out (need project marker; never bare-name)
├── plan.rs          ActionPlan write/read/revalidate.
├── clean.rs         The clean phase. select_candidates(),
│                    delete_selected(), broad-root and symlink guards.
└── output.rs        Human table renderer, JSON renderer, explain printer,
                     rules printer.
```

## Pipelines per command

### `scan` and `clean` (without `--plan`)

```text
CLI args ──► scan::scan(paths, options) ──► ScanReport
                │
                ├── for each root:
                │     ├── canonicalize() ─────────► absolute path
                │     ├── UserRuleSet::load_from_root(root)   .rclean.toml
                │     ├── IgnoreMatcher::build(root, globs)   .rcleanignore + --ignore
                │     └── walk:
                │           ├── classify directory name ─► rules::classify_candidate()
                │           │       └─ falls back to UserRuleSet::classify()
                │           ├── apply_path_safety()    ─► symlinks → Blocked
                │           ├── git status caching     ─► GitCache (per-repo, shared)
                │           ├── dirty git              ─► Safety::Safe → Caution
                │           └── tally per-dir bytes    ─► DirSizes map
                │
                └── ProjectReport list
                        ├── filter by category / rule / min_size / older_than / blocked
                        ├── risk_score (dirty_git, recent_mtime, lockfiles, root_boundary)
                        └── sort by reclaimable bytes desc

ScanReport ──► output::print_table() or print_json()
            ──► (optional) plan::write_action_plan() ─► JSON on disk

           (clean mode)
           clean::select_candidates(report, args)
                │
                ├── --all          ─► every Safety::Safe (+ Caution if --include-caution)
                ├── interactive    ─► numbered selection
                └── --plan <path>  ─► plan::selected_from_action_plan() + revalidate_selected()
                │
                ▼
           clean::check_broad_roots()   ─► refuses /, $HOME, /etc, /usr unless --allow-broad-root
           clean::confirm_if_needed()    ─► interactive y/n unless --yes
           clean::delete_selected()      ─► trash::delete_all() or fs::remove_dir_all()
                                            depending on --permanent
                │
                ▼
           CleanResult { succeeded, failed }
```

### `clean --plan rclean-plan.json`

```text
plan.json ──► plan::read_action_plan()
           ──► plan::selected_from_action_plan()
           ──► plan::revalidate_selected()
                │  re-checks every path against the live filesystem:
                │   - path still exists
                │   - root signature matches the plan
                │   - no new symlink on the path
                │   - safety has not been promoted from Blocked
                ▼
           clean::check_broad_roots(plan.roots)
           clean::delete_selected(selected, permanent)
```

Plan-based cleanup is the trust boundary: even if `scan` ran a week
ago, `clean --plan` will refuse to delete anything that has changed
shape in the meantime.

### `explain <path>`

```text
path ──► scan::explain_path(path)
       ├── rules::classify_candidate(parent, name, path)
       │       └─ Some(draft) → continue; None → Safety::Unknown
       ├── apply_path_safety(".", &mut draft)
       └── GitCache::info_for(parent) + project_activity()
       
       Explanation ──► output::print_explanation()
       Exit codes:
         Blocked  → 4
         Unknown  → 3
         Safe/Caution → 0
```

### `rules`

```text
rules::rule_catalog() ──► Vec<RuleInfo>
                       ──► output::print_rules()  (table)
```

## Trust boundaries

| Boundary | Where | What it guarantees |
|---|---|---|
| **`scan` is read-only** | `src/scan.rs` | No fs writes anywhere in the call graph. |
| **`Blocked` is non-selectable** | `src/clean.rs::select_candidates` | `--all`, interactive, and `--plan` all skip `Safety::Blocked`. `--include-blocked` only affects *visibility* in the report. |
| **Symlink → Blocked** | `src/scan.rs::apply_path_safety` | Any classified candidate that is a symlink at the time of scan is marked Blocked. |
| **Generic names need markers** | `src/rules/generic.rs`, `src/rules/markers.rs` | `build`, `dist`, `out`, `target`, `vendor` only classify when an ecosystem marker exists in the parent. |
| **User rules can't produce Blocked** | `src/user_rules.rs::parse_safety` | `.rclean.toml` `safety = "blocked"` is rejected at load time. |
| **Broad roots gated** | `src/clean.rs::check_broad_roots` | `clean` on `/`, `$HOME`, `/etc`, `/usr` requires `--allow-broad-root`. |
| **Plan revalidation** | `src/plan.rs::revalidate_selected` | A stale plan cannot delete paths whose safety, root, or symlink shape has changed since `scan`. |
| **Dirty git → Caution** | `src/scan.rs` + `GitCache` | A candidate inside a dirty repo cannot be `Safe`. |
| **`clean --all` excludes Caution by default** | `src/clean.rs::select_candidates` | `--include-caution` is opt-in. |

A change that loosens any of these requires a SPEC note before code.

## Output schema

All `--json` output is versioned via `schema_version: 1` on
`ScanReport`. New fields land additively; renames and removals bump
the schema.

`ActionPlan` carries the same `schema_version` for the same reason:
a plan written by v0.1.x is replayable by any later v0.1.x.

## Performance shape

`scan()` is two phases internally:

1. **Walk phase** — visit every directory entry once. For each
   entry: accumulate byte size into a per-directory `DirSizes`
   map, and if the directory name is a candidate, classify and
   stash the draft.
2. **Sizing phase** — for each candidate kept after filtering, fold
   the `DirSizes` map under that candidate's subtree
   (`sum_subtree_bytes`) to compute its reclaimable bytes.

The single-pass walk replaced an earlier per-candidate second walk
that traversed every subtree twice. The v0.1.5 milestone in
[`docs/specs/v0.1.x-roadmap.md`](specs/v0.1.x-roadmap.md) targets
3× the v0.1.0 throughput by parallelizing both phases — performance
regressions there should be caught by the `benches/` suite that
ships with that milestone. Until then, include a wall-clock
before/after when a PR touches the walker, `DirSizes`
accumulation, or `sum_subtree_bytes`.

## Where to put a new feature

- **New ecosystem / artifact directory**: add a file in `src/rules/`,
  register it in `catalog.rs`, add positive + negative tests in
  `tests/rules.rs`, update the README table.
- **New filter flag**: extend `cli.rs`, thread through `ScanOptions`,
  apply in `should_include` (`src/scan.rs`). Document the flag in
  README under **Filtering**.
- **New output format**: add a renderer in `src/output.rs` and wire
  it from `main.rs`. Don't add a new public field to `ScanReport`
  without versioning the JSON schema.
- **New safety guarantee**: SPEC update under `docs/specs/` first,
  then implementation. The trust-boundary table above is the
  current contract.
- **Performance work**: target one of the two phases described
  above; run the benchmark and attach the numbers.

## Related documents

- [`docs/specs/v0.1.x-roadmap.md`](specs/v0.1.x-roadmap.md) — current
  minor-series roadmap, milestones, and ship gates.
- [`docs/specs/complete-spec.md`](specs/complete-spec.md) — long-form
  specification of the trust model and command semantics.
- [`SECURITY.md`](../SECURITY.md) — threat model and private
  disclosure workflow.
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — PR conventions,
  toolchain, local verification.

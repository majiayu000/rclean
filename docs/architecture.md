# Architecture

This is the 30-second tour of `rclean` for new contributors. It maps
the CLI command surface to the code that runs it, and explains the
trust boundaries that govern every decision in the pipeline.

For historical v0.1.x planning context, read
[`docs/specs/v0.1.x-roadmap.md`](specs/v0.1.x-roadmap.md). This
document is the *current* implementation snapshot.

The core command surface includes scanning, cleaning, space-target
planning, explanation, rule/doctor diagnostics, agent and Docker
inspection, stamping/watching, and generated shell/man output. The
`tui` command requires the `tui` feature to run. `restore` and
`graveyard` are present only in builds with the `graveyard` feature.

## Module map

```
src/
├── main.rs          CLI entry. Parses args, sets up tracing, dispatches.
├── cli.rs           clap argument definitions for every subcommand.
├── error.rs         thiserror types: ScanError, PlanError, CleanError,
│                    ParseError, RcleanError (top-level union).
├── model.rs         Domain types: Safety, Category, Candidate,
│                    CandidateDraft, ScanReport, Summary.
├── parse.rs         CLI value parsers: size strings ("100mb"),
│                    duration strings ("6m", "30d").
├── doctor.rs        Per-machine global-cache applicability report.
├── scan/            The scan phase. scan() + explain entry points.
│   ├── mod.rs       Public scan API, ScanOptions, IgnoreMatcher.
│   ├── walker.rs    Parallel ignore::Walk traversal and draft grouping.
│   ├── project.rs   ProjectReport materialization, activity, risk score.
│   ├── safety.rs    Symlink, runtime/system, root-boundary checks.
│   ├── sizer.rs     Candidate/source byte accounting.
│   ├── git_cache.rs Shared per-repo git status cache.
│   └── tests.rs     Co-located scan tests.
├── user_rules.rs    .rclean.toml loader: globset-based custom rules.
├── rules/           Built-in classification rules, one file per ecosystem.
│   ├── mod.rs       Public surface: classify_candidate(), rule_catalog().
│   ├── catalog.rs   Per-ecosystem rule registration (the dispatch list).
│   ├── markers.rs   Shared marker-detection helpers (parent must contain X).
│   ├── project.rs   Candidate-name and project-kind helpers.
│   ├── node.rs      node_modules, .next, .turbo, .vite, .parcel-cache, ...
│   ├── python.rs    .venv, __pycache__, .pytest_cache, .ruff_cache, .tox, ...
│   ├── rust.rs      target (with Cargo.toml marker)
│   ├── go.rs        vendor
│   ├── jvm.rs       target, build, .gradle (with build.gradle / pom.xml)
│   ├── flutter.rs   build, .dart_tool (with pubspec.yaml)
│   ├── dotnet.rs    bin, obj
│   ├── ruby.rs      .bundle, vendor/bundle
│   ├── ios.rs       Pods
│   ├── cargo_global.rs, node_global.rs, pip.rs, gradle.rs, maven.rs, xcode.rs
│   │                Global toolchain-cache rules used by scan --home.
│   └── generic.rs   build, dist, out (need project marker; never bare-name)
├── plan.rs          ActionPlan facade; re-exports the supported plan API.
├── plan/
│   ├── schema.rs    ActionPlan and PlanCandidate schema definitions.
│   ├── io.rs        Atomic plan write, read, and schema-version checks.
│   ├── selection.rs Candidate collection and selected-plan summaries.
│   ├── revalidate.rs Live classification, root, and link revalidation.
│   └── id.rs        Stable candidate ID generation.
├── clean.rs         Clean facade; re-exports the supported clean API.
├── clean/
│   ├── selection.rs Bulk/text/TUI selection and Blocked filtering.
│   ├── roots.rs     Broad-root guard.
│   ├── output.rs    Confirmation, plan, result, and recovery output.
│   ├── deletion.rs  Trash, permanent, and optional graveyard deletion.
│   ├── validation.rs Pre-delete filesystem and open-file validation.
│   ├── native_tool.rs Bounded native cleanup-tool execution.
│   ├── audit.rs     Delete audit records and audit-path validation.
│   └── types.rs     SelectedCandidate and CleanResult types.
├── graveyard/       Recoverable delete store, manifest, restore/gc helpers.
├── tui/             Optional ratatui selector and fuzzy search.
├── watch/           Lockfile watcher and timestamped plan writer.
└── output.rs        Human table renderer, JSON renderer, explain printer,
                     rules/doctor/graveyard printers.
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
path + activity_depth ──► scan::explain_path_with_activity_depth(path, activity_depth)
       ├── rules::classify_candidate(parent, name, path)
       │       └─ Some(draft) → continue; None → Safety::Unknown
       ├── apply_path_safety(".", &mut draft)
       └── GitCache::info_for(parent) + project_activity(activity_depth)
       
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
| **`scan` is read-only** | `src/scan/` | No fs writes anywhere in the call graph. |
| **`Blocked` is non-selectable** | `src/clean/selection.rs::selectable_candidates`, `src/plan/revalidate.rs::selected_from_action_plan` | `--all`, interactive, and `--plan` all skip `Safety::Blocked`. `--include-blocked` only affects *visibility* in the report. |
| **Symlink → Blocked** | `src/scan/safety.rs::apply_path_safety` | Any classified candidate that is a symlink at the time of scan is marked Blocked. |
| **Generic names need markers** | `src/rules/generic.rs`, `src/rules/markers.rs` | `build`, `dist`, `out`, `target`, `vendor` only classify when an ecosystem marker exists in the parent. |
| **User rules can't produce Blocked** | `src/user_rules.rs::parse_safety` | `.rclean.toml` `safety = "blocked"` is rejected at load time. |
| **Broad roots gated** | `src/clean/roots.rs::check_broad_roots` | `clean` on `/`, `$HOME`, `/etc`, `/usr` requires `--allow-broad-root`. |
| **Plan revalidation** | `src/plan/revalidate.rs::selected_from_action_plan`, `src/plan/revalidate.rs::revalidate_selected` | A stale plan cannot delete paths whose safety, root, or symlink shape has changed since `scan`. |
| **Dirty git → Caution** | `src/scan/project.rs` + `GitCache` | A candidate inside a dirty repo cannot be `Safe`. |
| **`clean --all` excludes Caution by default** | `src/clean/selection.rs::select_candidates_text` | `--include-caution` is opt-in. |

A change that loosens any of these requires a SPEC note before code.

## Output schema

Machine-readable command outputs carry their own camelCase `schemaVersion`.
`scan --json` emits `ScanReport` schema `1`. `free --json` emits free proposal
schema `1` with target/selected bytes, target status, the ActionPlan path (or
`null` when no plan was written), and candidates serialized through the same
`Candidate` shape as scan. New fields land additively; renames and removals
require the affected output schema to bump.

`ActionPlan` has its own `schemaVersion`. Current builds write and
read schema `2`; schema `1` plans are rejected with a rescan hint.

## Performance shape

`scan()` is two phases internally:

1. **Walk phase** — visit every directory entry once. For each
   entry: accumulate byte size into a per-directory `DirSizes`
   map, and if the directory name is a candidate, classify and
   stash the draft.
2. **Sizing phase** — size candidate artifact directories with
   `dir_size()` and compute project/source bytes from the `DirSizes`
   data collected during the walk.

The single-pass walk replaced an earlier source-size pass that
traversed every project tree twice. After the walk,
`SourceSizeIndex::from_dir_sizes()` builds one bottom-up subtree index
from `DirSizes`; project/source byte queries then use the indexed value
for the project directory instead of folding the full map per project.

Candidate directories are pruned from the walk phase and sized
separately with `dir_size()`, so large artifacts such as `target/` and
`node_modules/` remain the main sizing hotspot. The v0.1.x roadmap is
historical planning context, not a list of work still pending in the
current architecture. Use the `benches/` suite and include a wall-clock
before/after when a PR touches the walker, `DirSizes` accumulation,
`SourceSizeIndex`, or candidate `dir_size()`.

## Where to put a new feature

- **New ecosystem / artifact directory**: add a file in `src/rules/`,
  register it in `catalog.rs`, add positive + negative tests in
  `tests/rules.rs`, update the README table.
- **New filter flag**: extend `cli.rs`, thread through `ScanOptions`,
  apply in `should_include` (`src/scan/mod.rs`). Document the flag in
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

- [`docs/specs/v0.1.x-roadmap.md`](specs/v0.1.x-roadmap.md) — historical
  v0.1.x roadmap, milestones, and ship gates.
- [`docs/specs/complete-spec.md`](specs/complete-spec.md) — long-form
  specification of the trust model and command semantics.
- [`SECURITY.md`](../SECURITY.md) — threat model and private
  disclosure workflow.
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — PR conventions,
  toolchain, local verification.

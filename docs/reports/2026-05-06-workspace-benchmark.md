# Workspace Benchmark

Date: 2026-05-06
Machine path: `/Users/apple/Desktop/code/AI/tool`
Command:

```bash
/usr/bin/time -p target/debug/rclean scan /Users/apple/Desktop/code/AI/tool --min-size 100mb --json
```

## Result

```text
real 76.35
user 3.73
sys 33.11
```

Summary:

| Metric | Value |
| --- | ---: |
| Projects scanned | 50 |
| Projects with candidates | 50 |
| Candidates | 51 |
| Safe candidates | 38 |
| Caution candidates | 13 |
| Blocked candidates | 0 |
| Reclaimable bytes | 478,102,355,046 |
| Reclaimable human | 445 GB |

## Largest Candidates

| Project | Kind | Candidate | Rule | Safety | Size |
| --- | --- | --- | --- | --- | ---: |
| harness-workflow-runtime-phase2 | Rust | target | rust.target | safe | 101 GB |
| harness | Rust | target | rust.target | safe | 41 GB |
| harness-pr-github-intake | Rust | target | rust.target | safe | 33 GB |
| harness-workflow-runtime-no-compat | Rust | target | rust.target | safe | 22 GB |
| harness-workflow-runtime-pr-feedback-child | Rust | target | rust.target | safe | 19 GB |
| harness-workflow-runtime-prompt-submission | Rust | target | rust.target | safe | 19 GB |
| harness-workflow-runtime-next | Rust | target | rust.target | safe | 19 GB |
| harness-workflow-runtime-sprint-planning | Rust | target | rust.target | safe | 18 GB |
| harness-workflow-runtime-repo-backlog | Rust | target | rust.target | safe | 18 GB |
| harness-workflow-runtime-legacy-feedback-fallback | Rust | target | rust.target | safe | 18 GB |
| harness2 | Rust | target | rust.target | caution | 18 GB |
| tink | Rust | target | rust.target | caution | 17 GB |

## Notes

- The scan found a real high-impact cleanup opportunity: roughly 445 GB of
  rebuildable artifacts above the 100 MB threshold.
- Runtime is dominated by directory sizing for large Rust `target` trees.
- No false positives were manually confirmed in this benchmark pass; the largest
  findings are project-local `target` directories with `Cargo.toml` markers.
- Caution entries are mostly dirty git worktrees. They should remain excluded
  from `clean --all` unless the user passes `--include-caution`.

## Follow-Up Performance Work

- Parallelize or batch directory size calculation.
- Add `--fast` mode that estimates from filesystem metadata where supported.
- Cache scan results by `(path, mtime)` for repeated scans.
- Add benchmark tests with generated fixture directories.

# Issue 111 Scan Perf Note

Date: 2026-05-31

## Fixture

Generated locally under `target/issue-111-perf-fixture-clean`:

- 100 small Node projects, each with one `node_modules/blob`
- 1 Rust project with `target/debug/deps/{64 dirs x 32 files}`
- a nested clean git repo whose `.gitignore` ignores the generated
  fixture payload, so before/after safety tiers are comparable

`hyperfine` was not installed, so timing used `/usr/bin/time -p` over
three runs. The command was:

```bash
target/release/rclean scan target/issue-111-perf-fixture-clean --json --min-size 0
```

## Result

| Revision | real runs | mean real | median real |
| --- | --- | ---: | ---: |
| `origin/main` baseline | 6.82s, 5.40s, 5.28s | 5.83s | 5.40s |
| after issue-111 patch | 4.87s, 4.46s, 4.09s | 4.47s | 4.46s |

Mean wall time improved by roughly 23% on this mixed fixture. A JSON
summary diff of run 3 matched after excluding volatile scan metadata,
so candidate bytes and safety tiers were preserved. This should still
be treated as a modest first PR rather than a complete solution for
the 2026-05-06 workspace benchmark.

## Notes

- Candidate sizing now splits a wide artifact tree across Rayon only
  after finding a multi-subdirectory fanout. Single-branch or tiny
  candidates stay close to the old `walkdir` path.
- Walker accumulation now uses per-worker maps and merges them at
  worker teardown, avoiding a global mutex lock per file.
- Output ordering remains deterministic by sorting candidate drafts by
  path after walker merge.

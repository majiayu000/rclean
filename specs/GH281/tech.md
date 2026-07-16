# Graceful Closed-Pipe Output - Tech Spec

## Linked Artifacts

- GitHub issue: `#281`
- Product spec: `specs/GH281/product.md`
- Tasks: `specs/GH281/tasks.md`
- Route: `write_spec`

## Evidence And Root Cause

| Area | Evidence | Decision |
| --- | --- | --- |
| Rust contract | stable `println!` panics when stdout write fails | Replace on non-interactive stdout paths; never catch its panic. |
| reproduction | `rules` and `doctor` against a short-lived reader panic and exit 101 | Add deterministic closed-reader E2E for both commands. |
| error model | `RcleanError::OutputIo(std::io::Error)` already exists | Reuse it; do not add an alias or stringly error. |
| MSRV | Rust 1.95 supports `std::io::pipe()` and `PipeWriter -> Stdio` | Use std-only cross-platform tests; no dev dependency. |
| command status | several commands intentionally return 3/4; clean can return 1 after work | Normalize BrokenPipe at command completion with the already computed status. |
| safety order | clean prints scan/plan before delete and summary after delete | Pre-delete pipe close stops; post-delete pipe close preserves result status. |
| unstable alternative | stable Rust has no stable `on_broken_pipe` hook for this policy | Do not use nightly flags, libc signal mutation or panic hooks. |

## Proposed Design

Add `src/stdio.rs` as the single fallible stdout primitive:

```rust
pub fn write_line(args: std::fmt::Arguments<'_>) -> std::io::Result<()>;
pub fn write_bytes(bytes: &[u8]) -> std::io::Result<()>;
pub fn is_broken_pipe(error: &RcleanError) -> bool;
```

Each writer locks stdout, uses `Write::write_fmt`/`write_all`, and returns the concrete I/O error.
It must not call `process::exit`, panic, log, or downgrade non-BrokenPipe errors.

Add a private command-boundary helper in `src/main.rs` conceptually equivalent to:

```rust
fn finish_output(
    status: ExitCode,
    result: Result<(), RcleanError>,
) -> Result<ExitCode, RcleanError> {
    match result {
        Ok(()) => Ok(status),
        Err(err) if stdio::is_broken_pipe(&err) => Ok(status),
        Err(err) => Err(err),
    }
}
```

Serialization happens before the write. Output modules may return `std::io::Result<()>` when they
only render, or `Result<(), RcleanError>` when serialization is also involved. In either case the
I/O source must reach `RcleanError::OutputIo` without string conversion.

### Command sequencing

- report/JSON commands compute their normal status, render, then call `finish_output`.
- generated completions render into `Vec<u8>` before one fallible stdout write; man/help propagate
  their native I/O error.
- free and stamp retain their plan/stamp-first order, compute 0/3, and finish output with that status.
- watch propagates fallible initial/diff output; initial BrokenPipe returns before watcher setup or
  loop entry, while later BrokenPipe unwinds the loop to success.
- clean treats pre-delete BrokenPipe as a safe early stop and returns without invoking delete.
  After delete, it computes 0/1 before rendering result/restore output and preserves that status if
  the pipe closes.
- interactive selection/TUI terminal drawing and stderr diagnostics remain outside this refactor.

## Product-to-Test Mapping

| Invariant | Evidence |
| --- | --- |
| B-001 | closed `std::io::pipe()` E2E for `rules` and `doctor`: no panic stderr, semantic status |
| B-002 | source audit: all declared non-interactive `println!` sites routed through `stdio` |
| B-003 | unit test BrokenPipe is quiet while a different `OutputIo` remains an error |
| B-004 | existing JSON parsing suites plus source order review |
| B-005 | status helper tests for success and nonzero semantic status |
| B-006 | pre-delete closed-pipe clean test proves target still exists |
| B-007 | output completion helper test preserves a supplied exit 1; existing failed-clean E2E |
| B-008 | existing free/stamp plan and result tests plus sequence review |
| B-009 | watch source/control-flow test; no watcher starts after initial output failure |
| B-010 | full existing CLI suites and exact output assertions |
| B-011 | tests use stable std anonymous pipe; exact MSRV and three-OS CI |
| B-012 | exact manifest, full Rust/VibeGuard/current-head gates |

## Planned Changes Manifest

| Path | Change |
| --- | --- |
| `src/stdio.rs` | Add the shared fallible stdout primitive and exact BrokenPipe classifier. |
| `src/main.rs` | Register stdio, preserve command statuses, route direct/generator output, and enforce clean sequencing. |
| `src/error.rs` | Expose structured inspection needed to recognize only direct output BrokenPipe. |
| `src/output.rs` | Convert non-interactive report/JSON renderers to fallible stdout. |
| `src/free.rs` | Make human/JSON proposal output fallible and preserve 0/3 status. |
| `src/docker.rs` | Make Docker report rendering fallible. |
| `src/watch/mod.rs` | Propagate initial and diff output failure out of the watch loop. |
| `src/stamp/mod.rs` | Make stamp report output fallible while preserving mutation-before-report order. |
| `src/clean/output.rs` | Make plan/result/recovery stdout fallible; leave confirmation I/O semantics unchanged. |
| `tests/cli.rs` | Register the new CLI regression module. |
| `tests/cli/pipe_output.rs` | Add deterministic closed-pipe and pre-delete safety E2E coverage. |
| `README.md` | Document early-reader-close behavior and retained semantic statuses. |
| `docs/architecture.md` | Record the fallible stdout boundary and safety ordering. |

No other implementation path is permitted. `Cargo.toml` and `Cargo.lock` must remain unchanged
because the MSRV standard library supplies the test pipe.

## Verification Plan

```sh
cargo test --test cli pipe_output -- --nocapture
cargo test --test cli -- --nocapture
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95.0 cargo build --all-targets --all-features
rustup run 1.95.0 cargo test
git diff --check
git diff --name-only origin/main...HEAD
```

Run every installed VibeGuard Rust guard. Before merge, re-read current head/base/files/checks,
GraphQL review threads and merge state. Require the exact approved path set, successful current-head
CI, zero unresolved current threads and CLEAN/MERGEABLE.

## Risks And Mitigations

- **masked command failure:** pass the already computed semantic status into output completion;
  never hard-code BrokenPipe to success after destructive work.
- **delete after output failure:** pre-delete output uses early return before any delete call.
- **partial JSON:** serialize to memory before the first stdout write.
- **partial human output:** downstream intentionally closed the stream; stop immediately and do not
  attempt fallback output.
- **error overmatching:** compare structured `ErrorKind::BrokenPipe`, never message substrings.
- **platform drift:** use stable `std::io::pipe()` rather than shell pipelines or Unix-only signals.
- **scope explosion:** terminal-only TUI/selection and stderr remain unchanged; exact manifest gate.

## Rollback

Revert the implementation commit. No persisted schema or cleanup state migration is required.

## Human Gates

- Spec and implementation remain separate PRs.
- The user has standing merge authorization; each merge still requires current-head CI, thread,
  merge-state and exact-scope evidence. Never force push.
- This is output reliability only and does not authorize trust-model changes.

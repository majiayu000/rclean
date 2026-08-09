# GH374 Deterministic Parallel-Walk Warnings - Tech Spec

## Linked Artifacts

- GitHub issue: `#374`
- Product spec: `specs/GH374/product.md`
- Tasks: `specs/GH374/tasks.md`
- Route: `plan_first`

```specrail-planned-changes
{
  "issue": 374,
  "complete": true,
  "paths": [
    "CHANGELOG.md",
    "specs/README.md",
    "specs/GH374/product.md",
    "specs/GH374/tech.md",
    "specs/GH374/tasks.md",
    "src/scan/sizer.rs",
    "src/scan/walker.rs",
    "src/scan/walker/tests.rs"
  ],
  "spec_refs": [
    "specs/GH374/product.md",
    "specs/GH374/tech.md",
    "specs/GH374/tasks.md"
  ]
}
```

## Root Cause

Each `WalkLocal` appends its warning vector under a mutex during `Drop`. Worker
completion order determines append order. `WalkScratch::into_inner` sorts
candidate drafts but returns the warning vector unchanged.

## Design

Expose the existing scan-internal `sizer::sort_warnings` helper to sibling
modules and call it in `WalkScratch::into_inner` after the warning mutex is
successfully unwrapped. The walker already depends on `sizer::DirSizes`, so
this introduces no new layering edge.

Canonical order remains:

1. ignore-file load warnings;
2. metadata warnings;
3. walk warnings;
4. path, then message within a variant.

## Product-to-Test Mapping

| Invariant | Implementation | Verification |
| --- | --- | --- |
| B-001/B-002 | sort at walker merge boundary | reverse-order unit fixture |
| B-003 | in-place stable data-preserving sort | exact vector equality/count |
| B-004 | `pub(super)` sizing helper | no duplicate comparator in walker |
| B-005 | scoped diff | existing scan and full suites |

## Verification

```sh
cargo test scan::walker::tests
cargo test scan::sizer::tests
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
rustup run 1.95 cargo build --all-targets --all-features
rustup run 1.95 cargo test
```


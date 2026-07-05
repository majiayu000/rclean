# GH221 Tasks

Issue: `#221`
Product spec: `specs/GH221/product.md`
Tech spec: `specs/GH221/tech.md`

## Status

- [x] `SP221-T001` Owner: `spec` | Done when: GH221 product, tech, and tasks files exist and link issue `#221` | Verify: `test -s specs/GH221/product.md && test -s specs/GH221/tech.md && test -s specs/GH221/tasks.md`
- [x] `SP221-T002` Owner: `ci` | Done when: benchmark workflow runs on `main` push and manual dispatch, not pull requests | Verify: `rg -n 'push:|workflow_dispatch' .github/workflows/benchmarks.yml && ! rg -n 'pull_request' .github/workflows/benchmarks.yml`
- [x] `SP221-T003` Owner: `ci` | Done when: workflow runs `scan_throughput` and uploads `target/criterion` as an artifact | Verify: `rg -n 'cargo bench --bench scan_throughput|target/criterion|upload-artifact' .github/workflows/benchmarks.yml`

## Handoff Notes

- This tranche closes issue `#221` after the workflow lands and the first main
  run publishes the artifact.
- It intentionally adds no PR CI trigger or benchmark threshold.

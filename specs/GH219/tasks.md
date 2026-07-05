# GH219 Tasks

Issue: `#219`
Product spec: `specs/GH219/product.md`
Tech spec: `specs/GH219/tech.md`

## Status

- [x] `SP219-T001` Owner: `spec` | Done when: GH219 product, tech, and tasks files exist and link issue `#219` | Verify: `test -s specs/GH219/product.md && test -s specs/GH219/tech.md && test -s specs/GH219/tasks.md`
- [x] `SP219-T002` Owner: `output` | Done when: empty human-readable scan output prints the home/tmp guidance hint | Verify: `cargo test --test cli empty_scan_human_output_suggests_home_or_tmp`
- [x] `SP219-T003` Owner: `tests` | Done when: JSON empty scan output remains hint-free and exits `3` | Verify: `cargo test --test cli empty_scan_json_omits_home_tmp_hint`

## Handoff Notes

- Runtime scan, cleanup, and ActionPlan behavior must remain unchanged.
- This tranche closes issue `#219` when its verification passes.

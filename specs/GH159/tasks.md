# GH159 tasks: Docker cleanup safety gates

- [x] Write product spec with Docker safety taxonomy.
- [x] Write technical spec documenting daemon/API and ActionPlan constraints.
- [x] Maintainer review: approve object model and first runtime slice
  (`docker report`, no ActionPlan/delete support).
- [x] Implement report-only Docker discovery after review.
- [x] Add mocked tests for daemon unavailable, permission denied, timeout,
  stale object, and report-only categories.
- [x] Add doctor/applicability output for Docker status behind explicit
  `doctor --docker`.
- [x] Update README and `docs/native-tool-cleanup-policy.md` after runtime
  behavior is implemented.
- [ ] Consider a later deletion PR only after report-only discovery is stable.

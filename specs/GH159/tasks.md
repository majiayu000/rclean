# GH159 tasks: Docker cleanup safety gates

- [x] Write product spec with Docker safety taxonomy.
- [x] Write technical spec documenting daemon/API and ActionPlan constraints.
- [ ] Maintainer review: approve object model and first runtime slice.
- [ ] Implement report-only Docker discovery after review.
- [ ] Add mocked tests for daemon unavailable, permission denied, timeout,
  stale object, and report-only categories.
- [ ] Add doctor/applicability output for Docker status.
- [ ] Update README and `docs/native-tool-cleanup-policy.md` after runtime
  behavior is implemented.
- [ ] Consider a later deletion PR only after report-only discovery is stable.

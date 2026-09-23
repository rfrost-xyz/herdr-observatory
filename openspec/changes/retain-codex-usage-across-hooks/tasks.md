# Tasks

## 1. Preserve the observation

- [ ] 1.1 Retain a validated Codex usage sample across an empty or older hook in the same native session, and verify the focused telemetry tests pass.
- [ ] 1.2 Cover source age, newer replacement and session change in a regression test, and verify it fails without the reporter fix.
- [ ] 1.3 Update the operator explanation and verify it matches the delta spec.

## 2. Verify and deliver

- [ ] 2.1 Run the repository gates, strict OpenSpec validation and independent review; record their results and traceability.
- [ ] 2.2 Archive the verified change, publish and merge the reviewed PR with a two-parent merge, and verify main matches origin/main.
- [ ] 2.3 Deploy one release image to both hosts, verify live health and retained usage, and preserve a working rollback.
- [ ] 2.4 Remove only obsolete Observatory images and the merged worktree, then verify the remaining images and Git state.

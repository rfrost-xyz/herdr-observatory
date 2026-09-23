## Traceability

| Requirement or scenario | Implementation | Verification |
| --- | --- | --- |
| Intermittent Codex usage read | `observatory/telemetry.py` reuses only validated Codex usage for the matching Herdr session and retains its original `usage_seq` | `tests/test_telemetry.py::TelemetryTests.test_codex_hook_without_usage_retains_last_bound_sample_and_age` covers empty, older and equal-timestamp hooks |
| Newer sample and session replacement | The same reporter accepts a later source timestamp and rejects previous metadata after the native binding changes | The regression covers both paths; the existing telemetry suite covers malformed and mismatched reports |
| Truthful last-known display | The reporter preserves the source timestamp; the existing browser labels usage older than two minutes | Existing `tests/test_ui.cjs` coverage and the full Node suite |

## Verification

- The new regression failed against `origin/main` with `AssertionError: None != 42`, then passed with the fix.
- Python: 122 tests passed, including the Unix socket roundtrip with the required local socket access.
- Node: all seven test files passed. `npm ci --ignore-scripts`, `npm run build:web`, `git diff --exit-code -- web/react-view.mjs` and `node --check web/app.js` passed.
- `openspec validate --all --strict` passed for four specs and this change; `git diff --check` passed.
- Independent review of corrected source head `6e66d17` found no actionable findings. PR #4 is mergeable and both hosted `checks/test` jobs passed for that head.

## Deployment and cleanup boundary

Both hosts currently run healthy `herdr-observatory:bdbd5af` at image digest `sha256:3fb9ddb9c73f11ab13ad93a81dda03a575fe18c1e1048094066eb3a8aaf07336`. Its source tree equals current `origin/main` despite its pre-rebase commit ID. Both `.env.previous` files select `3a546d7`. Compose configuration validates on both hosts and there is enough disk space for a release image.

Build the reviewed release on iapetus, transfer that same image to ws-255, then change each deployment's image selection while preserving `bdbd5af` as rollback. After live acceptance, remove only the unreferenced `3a546d7` Observatory image on each host. Shared Docker cache, other images, containers and volumes are outside this change.

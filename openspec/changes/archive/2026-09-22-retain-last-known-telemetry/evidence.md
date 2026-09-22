## Traceability

| Requirement | Implementation | Verification |
| --- | --- | --- |
| Retained session-bound detail | `observatory/telemetry.py`, `observatory/probe.py`, `web/app.js` | `tests/test_telemetry.py`, `tests/test_ui.cjs` |
| Cache percentage and exact token scope | `web/app.js` | `tests/test_ui.cjs` balance, Pi write and unknown tests |
| Idle muted, Done green | `web/app.js`, `web/style.css` | `tests/test_ui.cjs` theme/state tests |

## Gates
107 Python tests and the full JavaScript suite pass. Strict OpenSpec validation and whitespace checks pass. Independent review found and verified fixes for Codex's old-record cutoff, Pi's old-record cutoff and Pi's transient branch lookup path. The final review has no actionable findings. Browser layout remains unverified because automatic browser approval blocked the local file URL; the user previously authorised deployment without that check.

The versioned `ab1fbb4` image passed all 107 Python tests inside the image without network access. Both hosts run the same healthy image `sha256:4bf00984d67f466c0d1fa27b7f3eb179d6ae554b6a3ea2490f440db23ab417b0`. Served JavaScript and CSS hashes match on both hosts, as do installed Codex and Pi adapter payloads. The Work endpoint returns only Work agents. Both hosts preserve the previous `4e1e4ad` environment and config as rollback copies; its image remains available. Existing Pi sessions need reload for the updated extension.

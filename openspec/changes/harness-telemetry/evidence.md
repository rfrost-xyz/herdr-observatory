# Evidence

## Reviewed implementation

- Supplementary reports: `observatory/telemetry.py`; real Unix socket fixture plus session replacement, wrong authority/harness, ordering, expiry and missing fields in `tests/test_telemetry.py`.
- Live native API contract: temporary expiring token was written through `pane.report_metadata`, observed in `session.snapshot`, and removed. `pane.get` returns the expected pane envelope. No lifecycle or input mutation.
- Privacy/metrics: `observatory/probe.py`, `core.technical`, existing `feed.validate_feed`; raw-token and Work-feed path/tool filtering, unknown metrics, typed counters and Personal exclusion tests.
- Adapter lifecycle: image `hooks/` payload, `install.py`; preservation, repeated install/uninstall, conflicts and container validation tests. Pi adapter test verifies selected event payloads, sequence order and absence of content.
- Presentation: `web/app.js`; sampled change deduplication, expiry, authoritative state preservation, compact observation and last-response usage through output-to-idle tests.
- Gates: 53 Python tests; 77 Node UI/WASM tests, all 37 effects complete; JavaScript syntax checks; strict OpenSpec validation (no errors or warnings).
- Independent adversarial review initially found path disclosure and usage visibility issues. Both corrected with regression tests; renewed review approved implementation and README without actionable findings.

## Deployment

Pending deployment and installed-adapter verification. Existing sessions require a new session or reload; installation will not interrupt agents.

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

- Committed runtime `94def4f` built and deployed to both authorised hosts; both report healthy with `unless-stopped`. The same 53 Python contracts pass inside both production images.
- Installed image payloads match on both hosts. Repeated installation leaves exactly nine owned Codex entries; native Codex entries are unchanged. Pi extension is separate from the native integration. The office host lacked the native Pi integration; installed its official payload without editing managed files.
- Primary-host installed Codex adapter -> image -> native Herdr -> filtered probe succeeds (0.336 seconds). Native session remains unchanged; temporary telemetry is cleared.
- Office-host image -> native metadata -> snapshot round trip succeeds. Its already-running Codex thread has no native session reference: installed adapter correctly refuses an unmatched report, with no output or state mutation. New Codex sessions and Pi reload/new sessions are required for activation; existing agents were not restarted. No claim of a real post-install model/tool turn on either host.
- Both HTTP profiles have two online sources. Work response contains only Work agents. Payload identity/path stripping and both transport boundaries are covered by automated tests.
- Retained `756fe78` image and matching `.env.previous` on both hosts; removed superseded `cad165f`. Removed temporary installers, verified pre-install backups, verification metadata, task scripts/downloads and generated caches. No additional service, listener, checkout or host-side telemetry spool remains.

## Lifecycle

Runtime reviewed at `94def4f`; final deployment evidence review approved without findings. Both hosted implementation checks passed (push and PR workflows). The ready PR remains open and unmerged. Archive/spec synchronisation follows this acceptance checkpoint; documentation commits do not change the deployed runtime image.

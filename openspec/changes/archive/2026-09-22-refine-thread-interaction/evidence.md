# Verification

## Requirement trace

- Herdr state roles and inspection: `web/app.js`, `web/style.css`, `web/index.html`; UI regressions cover theme roles, click/keyboard inspection, source removal, clearing private details, frozen paging and focus after card reordering. Herdr upstream `src/app/state.rs` defines yellow for Working, red for Blocked and green for Done/Idle.
- Track presentation: `web/music-title.mjs`, both canvas instances in `web/app.js`; music tests cover shared title/artist identity, bounded completion, pending identity, reduced motion, stale sources and failure.
- Structured usage and truthful telemetry: `hooks/codex_usage.py`, Pi adapter, reporter and probe; Python/Pi tests cover owner/session/path bounds, malformed records, numeric allowlisting, source age, actual zeroes, reload seeding and delayed persistence.

## Checks

- 75 Python tests passed, including socket fixtures.
- All 163 JavaScript tests passed across six files, including 57 UI tests after the focus correction.
- JavaScript syntax, strict OpenSpec validation and whitespace checks passed.
- Synthetic eight-thread browser fixture: exact document/viewport dimensions at 1280x720 and 1920x1080; all eight cards have matching client/scroll heights. Inspector opens, updates and Escape restores focus with dynamic details cleared.
- Independent backend and frontend reviews resolved stale numeric timestamp handling, Pi persistence races and inspector privacy/focus findings. Final reviews clean.
- Codex structured schema checked against installed 0.155.1. Pi active-branch API and millisecond timestamps checked against installed extension/session documentation. No provider-independent missing counters are invented.

## Deployment

Release `d3cf8ec`, image `sha256:1796e9c38e630d4f4fe56a2e8bb277d075b100c70e55624d57750ca734b09142`, is healthy on both hosts. The packaged image passed all 75 Python tests. Served app, stylesheet and music module hashes match across hosts. All three installed hook/helper payload hashes match across hosts.

Live iapetus snapshots contain fresh Codex numeric enrichment. Both currently permitted Work threads were Idle with no recent hook usage, correctly retaining unknown metrics. All Work agents remain classified Work, both hosts are online, and Tokyo Night/theme plus music feeds are available. No Pi session was active during release validation; reload or start Pi to load its updated extension. Existing Codex shell entrypoints already invoke the updated helper.

Previous `23eee37` image and `.env.previous`/`config.json.previous` retained for rollback. Synthetic browser server and tab removed after geometry/interaction checks. No Windows desktop browser visual verification was performed; ws-255 service and assets were checked over SSH.

The refreshed live browser has the inspector and artist canvas, omits the sampling label, fits its viewport and renders Working in Tokyo Night amber. PR #1 is published and ready for review, unmerged. Hosted checks passed on the release evidence commit. Current and immediate rollback image tags are the only retained Observatory tags on either engine.

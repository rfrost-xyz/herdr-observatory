# Verification

## Requirement trace

- Herdr state roles and inspection: `web/app.js`, `web/style.css`, `web/index.html`; UI regressions cover theme roles, click/keyboard inspection, source removal, clearing private details, frozen paging and focus after card reordering. Herdr upstream `src/app/state.rs` defines yellow for Working, red for Blocked and green for Done/Idle.
- Track presentation: `web/music-title.mjs`, both canvas instances in `web/app.js`; music tests cover shared title/artist identity, bounded completion, pending identity, reduced motion, stale sources and failure.
- Structured usage and truthful telemetry: `hooks/codex_usage.py`, Pi adapter, reporter and probe; Python/Pi tests cover owner/session/path bounds, malformed records, numeric allowlisting, source age, actual zeroes, reload seeding and delayed persistence.

## Checks

- 75 Python tests passed, including socket fixtures.
- All six JavaScript suites passed; focused final UI rerun passed 57 tests after the focus correction.
- JavaScript syntax, strict OpenSpec validation and whitespace checks passed.
- Synthetic eight-thread browser fixture: exact document/viewport dimensions at 1280x720 and 1920x1080; all eight cards have matching client/scroll heights. Inspector opens, updates and Escape restores focus with dynamic details cleared.
- Independent backend and frontend reviews resolved stale numeric timestamp handling, Pi persistence races and inspector privacy/focus findings. Final reviews clean.
- Codex structured schema checked against installed 0.155.1. Pi active-branch API and millisecond timestamps checked against installed extension/session documentation. No provider-independent missing counters are invented.

## Deployment

Pending immutable image build and both-host verification. Pi live response validation depends on an active reloaded Pi session; adapter API fixtures verify that path without controlling a user's session.

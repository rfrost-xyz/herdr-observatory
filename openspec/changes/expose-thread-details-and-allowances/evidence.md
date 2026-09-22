# Verification evidence

## Requirement trace

| Requirement | Implementation | Evidence |
| --- | --- | --- |
| Persistent thread details and decorative activation | web/app.js, style.css | UI tests and 720p/1080p browser checks, eight cards fit; no hover overlay; click retains state |
| Taller muted fleet panels | web/style.css | Both viewport checks, measured-history regression |
| Manual music effects | web/music-title.mjs, app.js | Busy/stale/reduced-motion tests; clicking track starts both visible effect canvases |
| Split footer and honest relative allowance values | web/allowances.mjs, index.html | Seven allowance tests, missing/zero/stale/reset passage, both panel bounds verified |
| Account-bound collection and publication | allowances_probe.py, allowances.py, hooks, core.py, feed.py | Python allowance tests cover read-only RPC, byte/time bounds, mapping, deduplication, expiry and export revocation |
| Existing host binary lookup | allowances_probe.py | PATH priority and executable user-local fallback tests; live ws-255 minimal PATH diagnosis |
| Cross-account disclosure | Private configuration | User explicitly authorised both account summaries on both displays; no account identifiers in public evidence |

## Checks before release

107 Python tests pass. Browser geometry: 1280x720 document exactly1280x720 with eight171px cards and two97px allowance panels; 1920x1080 document exactly1920x1080 with eight307px cards and two105px allowance panels. No card or panel content overflows its height. Independent review found and resolved event-canvas overflow into the allowance column and stale-click motion. Long-line regression clamps a1200px text effect to595px visible column space. Native live reads returned distinct account identities with supported plan, weekly and reset-pass count fields; only private configuration retains identity hashes.

Final release and publication verification will be recorded after deployment.

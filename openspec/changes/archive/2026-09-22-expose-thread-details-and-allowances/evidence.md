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

107 Python tests pass. Browser geometry: 1280x720 document exactly 1280x720 with eight 171px cards and two 97px allowance panels; 1920x1080 document exactly 1920x1080 with eight 307px cards and two 105px allowance panels. No card or panel content overflows its height. Independent review found and resolved event-canvas overflow into the allowance column and stale-click motion. Long-line regression clamps a 1200px text effect to 595px visible column space. Native live reads returned distinct account identities with supported plan, weekly and reset-pass count fields; only private configuration retains identity hashes.

## Release verification

All 181 JavaScript tests and all 107 Python tests passed. The packaged Python image also passed all 107 tests. Final independent reviews were clean after clipping event effects to their observations column, suppressing stale click motion, validating reset-pass expiry and resolving the existing ws-255 user-local Codex binary.

Both machines run `herdr-observatory:1a845c7`, image `sha256:dc4626858400b04202ac5df5279ab75db5042b5eb82844f9a366ad1259e8af62`, healthy with `unless-stopped`. Exact served app, stylesheet and allowance-module hashes match the reviewed checkout. The image-supplied adapters were installed on both hosts. Both HTTP services report the two fresh, distinct, explicitly mapped accounts; neither exports account identifiers. The Work service contains zero Personal agents. Theme palettes match and the existing music feed is available.

The actual iapetus browser was refreshed: both account instruments render, no hover overlay remains, and all live cards/fleet/footer panels have matching client and content heights. Synthetic 720p and 1080p checks cover eight cards, long metrics and partial telemetry. Windows desktop UI was not inspected; ws-255 verification is through its service endpoints.

Previous image `2a787b1` and matching `.env.previous`/configuration copies remain on each host. Account setup data stays private and is removed after mapping installation. No new host service, credential mount or Docker socket mount was introduced. The only new installed helper is the image-owned allowance probe beside the existing hooks, plus its private throttle timestamp.

Canonical delta requirements and scenarios were checked for exact parity before archive. Publication remains the existing PR #1, ready for review and unmerged.

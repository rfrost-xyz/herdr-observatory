# Acceptance evidence

## Traceability

| Requirement / scenarios | Implementation | Verification |
| --- | --- | --- |
| Passive fleet observation / working agent, failure and recovery | observatory/probe.py, core.py | Snapshot allowlist, SSH stdin, failure/recovery and identity tests; live local 0.9.1 and SSH 0.9.0 samples succeeded |
| Display disclosure / Work and Personal | core.py normalise and classification, server.py | Private titles, paths and native IDs absent; personal inclusion; path boundaries; HTTP query cannot unlock Personal |
| Honest telemetry / missing GPU and stale browser | probe.py metrics, core.py rates, web/app.js | Counter reset, first sample, malformed values; UI stale and old-host tests; live kernel/container scope labels |
| Omarchy display / theme changes | probe.py palette, core.py theme, web/style.css and app.js | Validated palette/fallback and change tests; live Tokyo Night detected; responsive CSS, labelled controls and reduced-motion rules inspected |
| Local access boundary / unexpected origin | server.py | Real loopback HTTP tests reject Host/Origin, POST and unlisted paths; static assets and security headers verified |

## Checks

Python unit/integration suite, Node UI logic tests, JavaScript syntax, strict OpenSpec validation and git diff checks pass. Live polling succeeded for two source hosts under both profiles. Work mode excluded two local personal/unclassified agents while retaining two approved work agents. The SSH source reported one approved work agent.

Preview HTTP returned 200 at loopback. Browser opening was unavailable in this tool environment, so visual browser QA and the Windows physical display are not claimed. Windows launch is documented; no remote persistent service or desktop changes were made. Sampled history is not a complete tool-call stream. GPU values are unavailable when the collector cannot access NVIDIA tools.

## Independent review

Pending final review of the implementation commit. No programme register exists in this new repository.

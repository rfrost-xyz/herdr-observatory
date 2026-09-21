# Acceptance evidence

Runtime implementation and image tag: 45671e2. Both hosts run identical image ID sha256:e408e0f9187678747df2cbfda0676b11bab1c97c9900858c59758bd9b03725f5.

| Requirement / scenario | Implementation | Verification |
| --- | --- | --- |
| Differentiated technical activity / multiple working entities | web/app.js identityHash and heartbeatPhase; web/style.css | Eight Node tests cover stable identity, reordering, monotonic progression, stale-state removal and explicit missing fields. Chromium measured nine distinct durations for three machines and six threads: 5.09, 4.064, 4.121, 5.092, 5.38, 4.633, 5.272, 3.69 and 3.782 seconds |
| Differentiated technical activity / missing or private telemetry | observatory/probe.py, core.py technical/counter, feed.py | Typed counters/booleans, missing and malformed values, unsafe numbers and unknown-field stripping tested. Work excludes personal agents and their technical metadata before transmission |
| Fixed geometry and truthful visual treatment | web/app.js, style.css, README.md | Chromium synthetic fixture at actual 1280x720 and 1920x1080 content viewports reports no document or machine/agent/resource-card vertical overflow. Screenshots visually inspected. Reduced-motion override retained; ECG and moving highlights documented as decorative reported-status indicators |
| Complete operating guide | README.md, deploy/README.md | Root README contains architecture, Docker setup, private config, Tailscale, daily commands, startup, upgrades/rollback, Windows launch and field definitions; deployment guide links to the canonical sections |

37 Python tests pass on the host and inside the release image; eight Node tests, JavaScript syntax, strict OpenSpec and diff checks pass. Independent adversarial review approved 45671e2 with no actionable findings.

Live deployment: both Compose containers healthy and both sources online; protocol 22 and integer pane revisions verified in Personal and Work APIs. Typed metadata from the laptop survives the filtered Work feed. Personal-project exclusion, publisher health and active theme continuity passed. Previous image and .env.previous retained for rollback. No private configuration or live activity is committed. Optional readiness flags are absent on the installed versions and correctly remain unavailable. Model names, token/cost/context metrics, custom token maps, terminal contents and full paths are not added.

Visual verification uses synthetic data; the physical Windows monitor remains unverified from the container connection. No programme register applies.

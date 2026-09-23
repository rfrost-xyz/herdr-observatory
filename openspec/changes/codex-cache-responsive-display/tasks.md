# Tasks

## 1. Codex cache measurement

- [ ] 1.1 Add bounded opaque turn/child association validation to the source adapter without publishing identities; verify malformed and child-stop cases in focused tests.
- [ ] 1.2 Derive a bounded recent cache series from distinct cumulative Codex samples and model/compaction markers; verify repeats, resets, session changes, missing counters and timestamps.
- [ ] 1.3 Publish the recent cache view within existing metadata limits and Work filtering; verify source migration and privacy tests.

## 2. Account token activity

- [ ] 2.1 Read account activity through the existing app-server probe with independent response validation; verify supported, partial, unsupported and bounded responses.
- [ ] 2.2 Carry account activity through the mapped account cache, optional sharing and browser view; verify stale and unmapped-account cases.

## 3. React and responsive display

- [ ] 3.1 Add pinned React 19 and loading-dev build dependencies and local production asset output; verify clean build and no external runtime requests.
- [ ] 3.2 Move data-driven browser presentation to React while retaining stable canvas effects and controls; verify existing event, state, accessibility and motion behaviours.
- [ ] 3.3 Reflow the interface for narrow and short tiled windows; verify 720p, 1080p and tiled widths in Chromium with no horizontal clipping.
- [ ] 3.4 Measure bundle and initial render timing against the existing static page; record observed performance rather than an assumed advantage.

## 4. Acceptance and delivery

- [ ] 4.1 Run full Python and JavaScript gates, strict OpenSpec validation, build and diff checks; record requirement-to-evidence coverage.
- [ ] 4.2 Build and deploy a versioned image and owned adapters to both hosts, verify cache/account telemetry, responsive presentation and Work disclosure, retaining rollback assets.

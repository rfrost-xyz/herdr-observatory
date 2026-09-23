# Tasks

## 1. Codex cache measurement

- [x] 1.1 Add bounded opaque turn/child association validation to the source adapter without publishing identities; verify malformed and child-stop cases in focused tests.
- [x] 1.2 Derive a bounded recent cache series in the browser from distinct cumulative Codex samples and model/compaction markers; verify repeats, resets, pane changes, missing counters and timestamps.
- [x] 1.3 Render the recent cache view beside session totals without adding identity to metadata or the Work feed; verify privacy and source migration tests.

## 2. Account token activity

- [x] 2.1 Read account activity through the existing app-server probe with independent response validation; verify supported, partial, unsupported and bounded responses.
- [x] 2.2 Carry account activity through the mapped account cache, optional sharing and browser view; verify stale and unmapped-account cases.

## 3. React and responsive display

- [x] 3.1 Add pinned React 19 and loading-dev build dependencies and local production asset output; verify clean build and no external runtime requests.
- [x] 3.2 Move data-driven browser presentation to React while retaining stable canvas effects and controls; verify existing event, state, accessibility and motion behaviours.
- [x] 3.3 Reflow the interface for narrow and short tiled windows; verify 720p, 1080p and tiled widths in Chromium with no horizontal clipping.
- [x] 3.4 Measure bundle and initial render timing against the existing static page; record observed performance rather than an assumed advantage.

## 4. Acceptance and delivery

- [x] 4.1 Run full Python and JavaScript gates, strict OpenSpec validation, build and diff checks; record requirement-to-evidence coverage.
- [x] 4.2 Build a versioned image, smoke-test it with synthetic telemetry and account activity, and document the host rollout and rollback procedure. Live deployment follows integration with the separate graphics repair.

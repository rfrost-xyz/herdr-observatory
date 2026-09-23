# Acceptance evidence

## Requirement trace

| Requirement and scenarios | Implementation | Verification |
| --- | --- | --- |
| Recent Codex cache activity: distinct samples, repeats/resets, model/compaction markers | `observatory/core.py` session generation; `web/app.js` bounded deltas, gap and event markers; `web/react-view.jsx` display | `tests/test_core.py`, `tests/test_feed.py`, `tests/test_ui.cjs`; synthetic Chromium cache card |
| Opaque Codex turn association: child hook, invalid association | `hooks/codex_usage.py` source-bound opaque keys; `observatory/telemetry.py` owned metadata allowlist | `tests/test_codex_usage.py`, `tests/test_telemetry.py`; Work-feed validation |
| Account token activity: supported, partial/unsupported, office sharing | `observatory/allowances_probe.py`, `allowances.py`, `web/allowances.mjs`, `web/react-view.jsx` | `tests/test_allowances.py`, `tests/test_allowances.mjs`, `tests/test_feed.py`; live read-only capability probe |
| Tiled browser presentation: narrow tile, full display, connection/accessibility | `web/style.css`, `web/react-view.jsx`, local `web/react-view.mjs`, `observatory/server.py` | Chromium geometry and screenshots; `tests/test_ui.cjs`, `tests/test_server.py`, JavaScript build check |

## Source and disclosure

The installed Codex 0.155.1 CLI continues to supply the hook's bounded rollout usage. The hook validates opaque turn and child associations; the reporter's metadata allowlist does not transmit them. The collector generates a per-process numeric session generation when the validated Herdr session binding changes. Work-feed receipt revalidates that number and rejects extra native session fields. Browser cache history resets on generation change, missing counters, falling counters, source loss, harness change and pane departure. A resumed interval after a gap is marked `?`. `tests/test_codex_usage.py`, `tests/test_core.py`, `tests/test_feed.py` and `tests/test_ui.cjs` cover these boundaries. `SubagentStop` remains a parent observation without a child usage increment.

The short-lived account app-server now reads `account/usage/read` after `account/rateLimits/read`. A live read-only probe returned `available: true`, `activity_available: true` and 30 bounded buckets. No account numbers or identity from that read were recorded here. `tests/test_allowances.py` and `tests/test_allowances.mjs` cover supported, missing and malformed activity, mapping, expiry and browser display. Existing explicit publication and receipt validation apply to activity fields.

## Browser and build

`npm ci --ignore-scripts` and `npm run build:web` produced the committed, local `web/react-view.mjs` asset. The Python server serves it from its explicit asset allowlist. Chromium showed the React-rendered fleet, eight cards, account activity and accessible current-state text using synthetic data. Responsive checks returned document scroll width equal to viewport width at 520x700, 800x600, 1280x720 and 1920x1080. The thread area had its own scroll range below 1080p; both account cards remained in the document. Screenshots were inspected at those sizes. The site uses no runtime package manager or external script URL.

Static JavaScript payload comparison against `origin/main`: previous `app.js` 43,205 bytes raw / 13,631 gzip; current `app.js` plus React bundle 284,569 bytes raw / 88,392 gzip. Local Chromium navigation measurements under reload varied substantially between samples and are not a reliable production latency estimate. The bundle is materially larger, and no browser performance gain is claimed. React improves component maintenance and future UI changes; CSS provides the responsive reflow.

## Gates and rollout

- `python -m unittest discover -s tests -q`: 112 passed after the collector, feed and malformed account-usage changes.
- `node --test tests/test_ui.cjs tests/test_wasm.mjs tests/test_background.mjs tests/test_title.mjs tests/test_music_title.mjs tests/test_pi_hooks.mjs tests/test_allowances.mjs`: passed.
- `node --check web/app.js`, `git diff --check`, `openspec validate --all --strict`: passed.
- `docker build --pull=false --build-arg REVISION=codex-cache-responsive-display -t herdr-observatory:codex-cache-responsive-display .`: passed locally.
- The versioned image served the synthetic `/api/state`, `/react-view.mjs` (237,055 bytes) and `/` with HTTP 200 on loopback. The temporary container was stopped after the smoke check.

The graphics fix remains a separate open PR. Deploying this branch by itself would replace that repair, so live host rollout is held until both changes are integrated. The current versioned host images and installed hook payloads remain the rollback assets. After integration, rebuild one release image, install its adapters on iapetus and ws-255, deploy the image to both Compose services, verify local cache/account values and Work disclosure, and retain the previous image tag and adapter payload until live acceptance passes.

# Evidence

Reactive rendered terminal: `web/app.js`, `web/index.html`, `web/style.css` replace cards with a persistent canvas. Transition model deduplicates baseline, unchanged samples, state changes, expiry and reconnect. Canvas text and textContent preserve the disclosure boundary.

Verification: 37 Python tests, 10 Node tests, JS syntax and strict OpenSpec pass. Node scenarios cover baseline/metadata silence, work/input/done transitions, fresh removal, source expiry/recovery, transport recovery, reduced motion, bounded text/buffers, 720p/1080p geometry, theme accent and paging. Chromium synthetic fixture rendered the full scene and INPUT overlay. Independent review findings on theme, paging, telemetry and filters resolved; pane identifier heading corrected.

Deployment verification pending below.

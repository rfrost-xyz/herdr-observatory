# Evidence

Reactive rendered terminal: `web/app.js`, `web/index.html`, `web/style.css` replace cards with a persistent canvas. Transition model deduplicates baseline, unchanged samples, state changes, expiry and reconnect. Canvas text and textContent preserve the disclosure boundary.

Verification: 37 Python tests, 10 Node tests, JS syntax and strict OpenSpec pass. Node scenarios cover baseline/metadata silence, work/input/done transitions, fresh removal, source expiry/recovery, transport recovery, reduced motion, bounded text/buffers, 720p/1080p geometry, theme accent and paging. Chromium synthetic fixture rendered the full scene and INPUT overlay. Independent review findings on theme, paging, telemetry and filters resolved; pane identifier heading corrected.

Implementation commit `047335a` independently approved. Both Compose services run identical image `herdr-observatory:047335a`, healthy. Live Work response contains only Work agents, excludes the configured private project, includes both online sources and matches the Personal theme. Both serve the new terminal asset. Packaged image: 37 Python tests pass. Previous image retained in each deployment configuration. Chromium reduced-motion fixture retains readable observations without event effects; physical Windows monitor was not inspected.

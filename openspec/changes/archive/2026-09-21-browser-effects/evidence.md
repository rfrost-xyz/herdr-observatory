# Evidence

- Full catalogue and browser-local generation: `web/effects.mjs`, pinned `web/vendor/manifest.json`, and `tests/test_wasm.mjs`. All 37 effects reached natural completion on a 120×36 synthetic terminal with coloured cells. Three shuffled rotations contain each effect exactly once and avoid adjacent repeats.
- Playback and controls: `web/app.js`, `tests/test_ui.cjs`; fresh samples do not replace playback, final frame precedes hold, 10-second default, one-second adjustments, fractional timing, pause/reduced motion, source-loss cancellation and failure cleanup pass.
- Disclosure and serving: `observatory/server.py`, `tests/test_server.py`; filtered terminal text, explicit same-origin assets, WASM MIME/CSP and retired frame endpoint pass.
- Local gates: 42 Python tests, 60 Node tests, JavaScript syntax and strict OpenSpec pass. Packaged image repeats all 42 Python tests and verifies exact vendor hashes/licences.
- Chromium synthetic browser checks: full catalogue 37, active frame 100, Left/Right controls pass at 1280×720 and 1920×1080. Whole-terminal amber-themed screenshot inspected. No physical office monitor inspection claimed.
- Independent review: initial Docker allowlist omission and trailing blank line corrected; final review approved with no actionable findings.
- Deployment: both profiles healthy on image `herdr-observatory:c69ef8c`. Both serve WASM SHA-256 `d43fc55e05699b38eb3f372f73055e55a789b46e9149e49cd0f24813653d2d1a`, matching the manifest. Live asset MIME/CSP, removed frame endpoint and two online sources verified on each. Work snapshot contains only Work agents and excludes personal project data. Previous image `966ac56` retained for rollback.
- Publication: implementation pushed to existing PR #1; both implementation CI jobs passed. Canonical office-display requirement synchronised including all seven scenarios. Archive metadata published as the final documentation commit.

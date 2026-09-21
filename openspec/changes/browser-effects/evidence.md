# Evidence

- Full catalogue and browser-local generation: `web/effects.mjs`, pinned `web/vendor/manifest.json`, and `tests/test_wasm.mjs`. All 37 effects reached natural completion on a 120×36 synthetic terminal with coloured cells. Three shuffled rotations contain each effect exactly once and avoid adjacent repeats.
- Playback and controls: `web/app.js`, `tests/test_ui.cjs`; fresh samples do not replace playback, final frame precedes hold, 10-second default, one-second adjustments, fractional timing, pause/reduced motion, source-loss cancellation and failure cleanup pass.
- Disclosure and serving: `observatory/server.py`, `tests/test_server.py`; filtered terminal text, explicit same-origin assets, WASM MIME/CSP and retired frame endpoint pass.
- Local gates: 42 Python tests, 60 Node tests, JavaScript syntax and strict OpenSpec pass. Packaged image repeats all 42 Python tests and verifies exact vendor hashes/licences.
- Chromium synthetic browser checks: full catalogue 37, active frame 100, Left/Right controls pass at 1280×720 and 1920×1080. Whole-terminal amber-themed screenshot inspected. No physical office monitor inspection claimed.
- Independent review: initial Docker allowlist omission and trailing blank line corrected; final review approved with no actionable findings.
- Deployment and publication: pending.

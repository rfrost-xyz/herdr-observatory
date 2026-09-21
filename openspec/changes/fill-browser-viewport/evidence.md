# Evidence

- `web/app.js` derives 120×36 cell dimensions from viewport size with eight-pixel edges, shared by live text and WASM effects. `web/style.css` fixes the canvas to the viewport.
- Geometry regression covers 720p, 1080p, 1536×864 and 1000×800; resize regression checks canvas backing size at DPR 2 and changing dimensions.
- 42 Python tests, 23 UI tests and 39 WASM tests passed; syntax, strict OpenSpec and diff checks passed. Chromium reports 1280×720 and 1920×1080 with active effects and working keyboard controls. Updated full-width screenshot inspected.
- Independent review approved runtime geometry and clipping with no actionable findings.
- Deployment pending.

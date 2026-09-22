# Evidence

- Runtime `fc94d85`: browser state labels and accessible transcript use Herdr words. `threadMotion` independently phases working indicators/highlights and suppresses movement for paused, reduced, stale and disconnected views. UI tests verify duplicate suppression and bounded arrival highlights.
- `effectText`, clipped `draw` and the WASM wrapper agree on a 140x24 CLI capture. Tests verify live thread text and controls during playback, exact clipping coordinates and deferred artwork activation. All 37 effects complete at CLI dimensions without consecutive repeats.
- Gates: 54 Python tests, 82 Node UI/WASM tests, JavaScript syntax and strict OpenSpec validation pass. Python suite also passed inside the production image. Independent review approved final implementation and documentation without findings.
- Synthetic isolated Chromium renders inspected at 1280x720 and 1920x1080, including a real WASM CLI effect. Thread labels, host information, controls and one-screen geometry remain intact above/below the effect rectangle. No physical monitor interaction claimed.
- Both authorised deployments are healthy at the runtime image. HTTP app.js and effects.mjs match the reviewed source byte-for-byte on both hosts; Personal and Work source availability and Work disclosure checked.
- Retained a9a0948 rollback and matching previous environment on both machines, removed superseded 94def4f images. Temporary synthetic server, browser profiles and screenshots removed. Existing harness adapters unchanged.

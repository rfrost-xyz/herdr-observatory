# Evidence

Runtime 6ad74b4 implements one-line events, native pane labels, exact-width one-row effects and removal of travelling bands/artwork. Tests cover all 37 final frames, Unicode, full playback, cooldown, duplicate suppression, source expiry and genuinely in-flight load cancellation across recovery. Independent review approved after the async cancellation finding was fixed.

54 Python and 85 Node tests passed, plus syntax, strict OpenSpec and diff checks. Python tests also passed in the production image. Synthetic Chromium composition inspected at 720p/1080p, including a deterministic real WASM frame with unaffected neighbouring lines and threads.

Both hosts deployed 6ad74b4 and became healthy. Served app.js, effects.mjs and index.html match committed source on both. Both source connections available; Work disclosure verified. Previous fc94d85 image and environment retained for rollback.

The user subsequently requested a card-focused Omarchy-inspired redesign. This delivery records the completed intermediate deployment; its replacement is tracked separately.

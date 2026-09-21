# Design

Link ttfx 0.3.3 at commit 54d21f046f22512b113056a1964077d7b7bf04cc via Cargo.lock. A compiled Rust adapter reads <=2048 bytes of printable snapshot-derived text and uses the library Effect/EngineCtx API. Allow only decrypt, vhstape and crumble; bound to 360 simulation frames, emitting every third frame (<=120 frames, 100x6 cells). Render without ANSI colours so the browser applies the active palette. Build in a pinned Rust Docker stage and copy only the executable and licence notices to the runtime image.

The server selects only current, already-profile-filtered observations. Serialize generation with a lock, cache each source-capture signature, enforce a subprocess deadline and validate bounded frame dimensions/control characters. No client-supplied text or effect arguments. Existing Host/Origin restrictions cover the GET endpoint. Errors return plain observations; offline sources return no animated frames.

The browser uses a static character-cell TUI with states first, machine/harness and project/thread title. No circles, projected grids, wave geometry, glyph fragments or line displacement. Only the separate six-row CLI area plays ttfx frames at 30fps, then settles on original text. Pause/reduced motion display original text immediately. Thread state continues updating independently. Plain fallback remains usable when the library is unavailable.

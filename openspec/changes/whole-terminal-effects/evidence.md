# Evidence

`renderer/src/main.rs` emits coloured full 120x36 frames through natural completion with an END marker. `observatory/effects.py` validates completeness and bounds, selects a different effect and shares the terminal builder with the state endpoint. Browser scheduler preserves every frame and fractional timing, starts the hold after completion, keeps routine updates separate, applies one-second arrow adjustments and removes all renderer display branding. Page Up/Down pages threads.

45 Python tests and 19 Node tests pass. Actual full-scene effects generated hundreds of coloured frames through completion within budget. Tests cover completion-marker rejection, nonrepeating selection, palette input, source expiry, privacy, pause, reduced motion, last-frame/hold ordering and fractional ticks. Syntax, diff checks and strict OpenSpec pass. Chromium full-screen synthetic amber-theme fixture inspected at 720p: all regions animate, colour preserved, no branding. Independent review timing and capture-alignment findings resolved and approved.

Deployment verification pending.

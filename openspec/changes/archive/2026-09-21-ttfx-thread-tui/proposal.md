# ttfx thread TUI

## Why
The user rejects decorative circles and requests a clear thread/state TUI with a CLI feed rendered by a text-effects library such as ttfx.

## What Changes
Remove all geometric background effects and hand-written text animation. Render a fixed character-cell thread TUI and a separate CLI area using actual frames from the pinned ttfx Rust library. A bounded adapter and same-origin read-only endpoint generate frames from the server-filtered snapshot. Preserve Docker, themes, privacy and reduced motion.

## Capabilities
### Modified Capabilities
- office-display: library-rendered text feed and stable thread TUI.

## Impact
Rust build-time library adapter, Python read-only frame endpoint, browser renderer, image build, tests and documentation. No Herdr control or raw terminal capture.

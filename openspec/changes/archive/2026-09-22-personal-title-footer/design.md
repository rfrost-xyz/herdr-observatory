# Design

## Context
The current display has an agent activity widget, full-viewport pixel canvas and four event rows. Delta Corps Priest 1 glyph provenance already exists in the repository history. Event effects intentionally accept only one bounded line.

## Decisions
Restore only the title glyph artwork and attribution. A separate title component owns its canvas, WASM session and shuffled effect selection. Add a bounded title-specific multiline factory while retaining event-line validation. Scale the static eight-row glyph artwork to a small header button. Start an automatic effect after sixty visible seconds and support click activation without interrupting an effect or queuing duplicate clicks. Hidden/reduced-motion and failure paths restore static artwork and release resources; do not replay missed intervals. Title animation is presentation only, independent of Herdr connectivity.

Size the background canvas to the actual footer top, update it after layout/resize and keep the footer fully opaque and full-width. Derive all field inks from the active accent mixed towards the background, including music and click highlights. Keep native upstream geometry and interactions.

Render a stable semantic icon beside each one-line event and select the colour for every event kind. Keep the text effect aligned to the text span, leaving its icon visible. Remove the old activity widget and its bookkeeping, retain per-card impulses, and omit the OS theme suffix.

## Risks / Trade-offs
- Multiline effects may behave differently: exercise all bundled effects against the exact title dimensions.
- Compact header and footer can reduce space: verify 720p and 1080p without scroll or clipped cards.
- Quiet or reduced-motion views: keep a readable Rich fallback and never imply agent events from decorative title effects.

## Migration
Build one reviewed versioned image and replace both existing Compose deployments. Preserve one pre-change image/configuration for rollback. No host adapter or daemon changes.

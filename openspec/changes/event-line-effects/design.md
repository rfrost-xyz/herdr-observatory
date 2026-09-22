## Decisions
Use a single-line effect canvas sized to its text, positioned at the event's existing left edge and row. This removes block centring. Render all other records normally; new arrivals scroll the affected line with its neighbours. Only fresh status transitions are eligible, with at most one active effect and a 120-second default minimum cooldown after completion. Events during cooldown are displayed immediately without deferred replay. Drop pending starts if their source, record or view is no longer valid. Pause freezes playback; reduced motion and failure retain readable text. Remove the decorative atlas and route entirely.

Thread spinners remain local to their state glyph. Whole-row bounded impulses show observed changes without travelling bands. CLI records fit one row with bounded columns for time, project, state and thread number followed by the update. Thread identifiers use available pane metadata, never an invented count.

## Verification
Test event fields and deduplication, single-row geometry, no idle replay, cooldown and playback safety, source loss, removal of artwork, bounded impulse and all bundled effects at line dimensions. Inspect synthetic rendering and verify deployed asset hashes on both hosts.

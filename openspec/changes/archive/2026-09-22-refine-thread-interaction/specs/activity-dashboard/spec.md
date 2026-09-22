## ADDED Requirements

### Requirement: Herdr state roles and thread inspection
The display SHALL use the active theme's semantic Herdr state roles, with Working amber/yellow, Blocked red, and Done/Idle green; Idle retains its own label and glyph. Clicking a card or activating it by keyboard SHALL open a read-only in-page inspector showing exact available metrics, current state, identity, freshness and bounded recent observations for that thread. It SHALL preserve focus, close with Escape and close when its permitted source disappears. It SHALL not send input to Herdr or add scrolling to the document. The footer SHALL omit the sampling disclaimer and music SHALL omit the source playback label.

#### Scenario: Inspect a thread
- **WHEN** a user activates a permitted card and observations arrive
- **THEN** its inspector updates only from that thread's available metadata and recent observations, while the fleet remains passive.

#### Scenario: Source loss
- **WHEN** the inspected thread becomes unavailable or leaves the selected disclosure view
- **THEN** the inspector closes without retaining stale private details.

## MODIFIED Requirements

### Requirement: Track change presentation
The music tile SHALL emphasise track title and artist and play bounded, theme-aware in-place text effects on both lines on an observed track change. Duplicate samples, initial connection, pause/resume and stale-source recovery SHALL NOT trigger track-change effects. Effects SHALL remain outside thread cards, have no consecutive repeats, finish normally before another starts, and release resources on hidden/reduced-motion/stale/error paths with readable text retained.

#### Scenario: Track transition
- **WHEN** a fresh source changes from one track identity to another
- **THEN** title and artist receive effects while both native strings remain accessible.

#### Scenario: Unavailable or rapid changes
- **WHEN** the source expires or another track arrives during playback
- **THEN** stale effects are removed or allowed to finish without replacing the current readable metadata; at most the latest pending identity is retained, with no replay backlog.

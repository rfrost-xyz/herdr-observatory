## MODIFIED Requirements

### Requirement: Herdr state roles and thread inspection
The display SHALL use the active theme's semantic Herdr state roles, with Working amber/yellow, Blocked red, and Done/Idle green. Cards SHALL persistently show bounded current technical detail with readable labels and visual instruments. Hover and focus SHALL provide visual emphasis only; no information SHALL depend on hover or open a separate click inspector. Clicking or keyboard-activating a tile SHALL play a brief local decorative response respecting reduced motion, using a bounded decorative glitch without synthetic activity or Herdr input. Detail SHALL clear when its permitted source disappears. The footer SHALL omit the sampling disclaimer and music SHALL omit the source playback label.

#### Scenario: Inspect a thread
- **WHEN** a permitted card is visible and observations arrive
- **THEN** its in-card details update from that thread's available metadata without changing its state or sending input.

#### Scenario: Source loss
- **WHEN** the inspected thread becomes unavailable or leaves the selected disclosure view
- **THEN** its visible detail clears without retaining stale private details.

#### Scenario: Decorative activation
- **WHEN** a card is clicked or activated by keyboard
- **THEN** a bounded local effect responds without opening a dialogue or generating an activity observation.

### Requirement: Compact measured metric panels
Fleet machines SHALL show CPU, memory, disk and network in subtly coloured theme-aware bordered panels with instruments using their available height inspired by the supplied terminal monitor. Histories SHALL contain distinct measured samples, stay bounded and stop showing live traces for stale sources. Thread panels SHALL prioritise available session totals, context percentage and cache/compaction data with readable scope and visible precise values. The display SHALL retain eight-card paging, no document scrolling at 720p and 1080p, and reduced-motion support.

#### Scenario: Measured fleet history
- **WHEN** successive fresh host samples arrive
- **THEN** compact traces update from those measurements; repeated polls add no duplicate measurement and missing values are not zeroes.

#### Scenario: Thread usage
- **WHEN** a fresh harness reports cumulative input/output, context and cache counters
- **THEN** the card displays their distinct scopes without presenting last-response counters as totals or cached tokens as request counts.

### Requirement: Track change presentation
The music tile SHALL emphasise track title and artist and play bounded, theme-aware in-place text effects on both lines on an observed track change or explicit title/artist activation. Duplicate samples, initial connection, pause/resume and stale-source recovery SHALL NOT trigger automatic track-change effects. Effects SHALL remain outside thread cards, have no consecutive repeats, finish normally before another starts, and release resources on hidden/reduced-motion/stale/error paths with readable text retained.

#### Scenario: Track transition
- **WHEN** a fresh source changes from one track identity to another
- **THEN** title and artist receive effects while both native strings remain accessible.

#### Scenario: Unavailable or rapid changes
- **WHEN** the source expires or another track arrives during playback
- **THEN** stale effects are removed or allowed to finish without replacing the current readable metadata; at most the latest pending identity is retained, with no replay backlog.

#### Scenario: Manual music effects
- **WHEN** a user clicks or keyboard-activates either fresh music line
- **THEN** both title and artist effects can play, without interrupting active playback, queuing repeats or changing the music player.

## MODIFIED Requirements

### Requirement: Herdr state roles and thread inspection
The display SHALL use the active theme's semantic Herdr state roles, with Working amber/yellow, Blocked red, Done green and Idle muted like its header total. Cards SHALL persistently show bounded current technical detail with readable labels and visual instruments. Hover and focus SHALL provide visual emphasis only; no information SHALL depend on hover or open a separate click inspector. Clicking or keyboard-activating a tile SHALL play a brief local decorative response respecting reduced motion, using a bounded decorative glitch without synthetic activity or Herdr input. Detail SHALL clear when its permitted source disappears. The footer SHALL omit the sampling disclaimer and music SHALL omit the source playback label.

#### Scenario: Inspect a thread
- **WHEN** a permitted card is visible and observations arrive
- **THEN** its in-card details update from that thread's available metadata without changing its state or sending input.

#### Scenario: Source loss
- **WHEN** the inspected thread becomes unavailable or leaves the selected disclosure view
- **THEN** its visible detail clears without retaining stale private details.

#### Scenario: Last known session-bound detail
- **WHEN** an identified thread remains present without a new hook report for more than two minutes
- **THEN** its last reported values remain visible with their original source age and an explicit last-known label, without changing Herdr state or implying live activity.
- **WHEN** the pane closes, its session binding changes or a newer report replaces the metadata
- **THEN** the previous values no longer appear on that thread.

#### Scenario: Decorative activation
- **WHEN** a card is clicked or activated by keyboard
- **THEN** a bounded local effect responds without opening a dialogue or generating an activity observation.

### Requirement: Compact measured metric panels
Fleet machines SHALL show CPU, memory, disk and network in subtly coloured theme-aware bordered panels with instruments using their available height inspired by the supplied terminal monitor. Histories SHALL contain distinct measured samples, stay bounded and stop showing live traces for stale sources. Thread panels SHALL prioritise available session totals, context percentage and cache/compaction data with readable scope, compact visible counts and exact accessible values. A complete cache-read balance SHALL include a percentage of total input tokens, without implying request hit/miss counts. The display SHALL retain eight-card paging, no document scrolling at 720p and 1080p, and reduced-motion support.

#### Scenario: Measured fleet history
- **WHEN** successive fresh host samples arrive
- **THEN** compact traces update from those measurements; repeated polls add no duplicate measurement and missing values are not zeroes.

#### Scenario: Thread usage
- **WHEN** a fresh harness reports cumulative input/output, context and cache counters
- **THEN** the card displays their distinct scopes and a source-grounded cache-read token percentage without presenting last-response counters as totals or cached tokens as request counts.

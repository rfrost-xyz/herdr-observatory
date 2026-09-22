## MODIFIED Requirements

### Requirement: Glanceable activity presentation
The display SHALL give thread cards persistent subtle semantic state colour, prominent native state labels, project and checkout identity instead of terminal titles, and compact visual telemetry. Missing hook fields SHALL be explained as one coverage note rather than repeated empty rows. Unknown subagent counts SHALL never appear as zero. Recent observations SHALL use the associated native state colour when available. The header SHALL omit host-role and theme-name labels and retain Idle, Working, Blocked and Done totals in subtle persistent boxes. Cards SHALL avoid repeating the native state as their hook activity and omit meaningless bare-repository checkout labels.

#### Scenario: Partial hook coverage
- **WHEN** a fresh hook sample has a tool and phase but no usage counters
- **THEN** the card shows its available activity with a concise coverage explanation and no fabricated counters or repeated Unavailable rows.

#### Scenario: Completed thread
- **WHEN** a thread is Done, including a tool observation associated with that state
- **THEN** its card and observation use the theme's green and the Done total counts it.

#### Scenario: Checkout disclosure
- **WHEN** a permitted thread has a reported checkout directory
- **THEN** only its sanitised leaf label reaches the browser and Work feed; full paths and excluded projects remain undisclosed.

### Requirement: Herdr state roles and thread inspection
The display SHALL use the active theme's semantic Herdr state roles, with Working amber/yellow, Blocked red, and Done/Idle green. Hovering or focusing a card SHALL reveal bounded current technical detail with keyboard parity; it SHALL not open a separate click inspector. Clicking or keyboard-activating a tile SHALL play a brief local decorative response respecting reduced motion, without text effects, synthetic activity or Herdr input. Detail SHALL clear when its permitted source disappears. The footer SHALL omit the sampling disclaimer and music SHALL omit the source playback label.

#### Scenario: Inspect a thread
- **WHEN** a user hovers or focuses a permitted card and observations arrive
- **THEN** its in-card details update from that thread's available metadata without changing its state or sending input.

#### Scenario: Source loss
- **WHEN** the inspected thread becomes unavailable or leaves the selected disclosure view
- **THEN** its hover/focus detail clears without retaining stale private details.

#### Scenario: Decorative activation
- **WHEN** a card is clicked or activated by keyboard
- **THEN** a bounded local effect responds without opening a dialogue or generating an activity observation.

## ADDED Requirements

### Requirement: Compact measured metric panels
Fleet machines SHALL show CPU, memory, disk and network in theme-aware bordered panels inspired by the supplied terminal monitor. Histories SHALL contain distinct measured samples, stay bounded and stop showing live traces for stale sources. Thread panels SHALL prioritise available session totals, context percentage and cache/compaction data with readable scope and exact hover/focus values. The display SHALL retain eight-card paging, no document scrolling at 720p and 1080p, and reduced-motion support.

#### Scenario: Measured fleet history
- **WHEN** successive fresh host samples arrive
- **THEN** compact traces update from those measurements; repeated polls add no duplicate measurement and missing values are not zeroes.

#### Scenario: Thread usage
- **WHEN** a fresh harness reports cumulative input/output, context and cache counters
- **THEN** the card displays their distinct scopes without presenting last-response counters as totals or cached tokens as request counts.

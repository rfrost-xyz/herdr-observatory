# Activity dashboard

## Purpose
Provide truthful, passive visibility of Herdr agents and machine activity across a private fleet with display-specific disclosure.

## Requirements

### Requirement: Passive fleet observation
The dashboard SHALL collect Herdr agent metadata from configured local and SSH hosts and explicitly configured Work feed files without controlling panes. Feed persistence SHALL be limited to the configured private display state path.

#### Scenario: Working agent
- **WHEN** a valid snapshot contains a working agent
- **THEN** the dashboard shows its machine, project, harness, safe checkout label when reported and prominent working state.

#### Scenario: Collector failure and recovery
- **WHEN** a host times out or returns invalid data
- **THEN** that host is unavailable and its previous agents are not counted as working; other hosts continue and a later valid sample restores it.

### Requirement: Display disclosure
The server SHALL enforce its selected Work or Personal profile, classify unknown paths as Personal and omit raw terminal output, full paths and native session identifiers from HTTP responses.

#### Scenario: Work display
- **WHEN** a personal or unclassified agent is collected in Work mode
- **THEN** its metadata and history are absent from browser responses, including requests containing a personal profile parameter.

#### Scenario: Personal display
- **WHEN** Personal mode is selected at server startup
- **THEN** personal and work agents can be viewed and filtered.

### Requirement: Honest telemetry
The dashboard SHALL show sampled status transitions and available resource metrics with timestamps and collection scope; unavailable metrics SHALL remain unavailable.

#### Scenario: Missing GPU and stale browser
- **WHEN** GPU telemetry is unavailable or the browser loses its connection
- **THEN** GPU values are marked unavailable and stale browser activity stops appearing live.

### Requirement: Omarchy display
The dashboard SHALL adopt valid active Omarchy palette changes without modifying desktop configuration and provide a usable fallback, responsive layout, keyboard controls and reduced-motion support.

#### Scenario: Theme changes
- **WHEN** the selected theme source publishes a different valid palette
- **THEN** the next successful refresh updates dashboard colours; missing or malformed colours use safe defaults.

### Requirement: Local access boundary
The service SHALL bind to loopback and reject browser requests with untrusted Host or Origin headers and expose no agent mutation endpoints.

#### Scenario: Unexpected origin
- **WHEN** a request comes with an external Host or Origin
- **THEN** the service denies it without returning activity data.

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

### Requirement: Track change presentation
The music tile SHALL emphasise track title and artist and play bounded, theme-aware in-place text effects on both lines on an observed track change. Duplicate samples, initial connection, pause/resume and stale-source recovery SHALL NOT trigger track-change effects. Effects SHALL remain outside thread cards, have no consecutive repeats, finish normally before another starts, and release resources on hidden/reduced-motion/stale/error paths with readable text retained.

#### Scenario: Track transition
- **WHEN** a fresh source changes from one track identity to another
- **THEN** title and artist receive effects while both native strings remain accessible.

#### Scenario: Unavailable or rapid changes
- **WHEN** the source expires or another track arrives during playback
- **THEN** stale effects are removed or allowed to finish without replacing the current readable metadata; at most the latest pending identity is retained, with no replay backlog.

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

### Requirement: Compact measured metric panels
Fleet machines SHALL show CPU, memory, disk and network in theme-aware bordered panels inspired by the supplied terminal monitor. Histories SHALL contain distinct measured samples, stay bounded and stop showing live traces for stale sources. Thread panels SHALL prioritise available session totals, context percentage and cache/compaction data with readable scope and exact hover/focus values. The display SHALL retain eight-card paging, no document scrolling at 720p and 1080p, and reduced-motion support.

#### Scenario: Measured fleet history
- **WHEN** successive fresh host samples arrive
- **THEN** compact traces update from those measurements; repeated polls add no duplicate measurement and missing values are not zeroes.

#### Scenario: Thread usage
- **WHEN** a fresh harness reports cumulative input/output, context and cache counters
- **THEN** the card displays their distinct scopes without presenting last-response counters as totals or cached tokens as request counts.

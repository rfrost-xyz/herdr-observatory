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
The display SHALL give thread cards persistent subtle semantic state colour, prominent native state labels, project and checkout identity instead of terminal titles, and compact visual telemetry. Missing hook fields SHALL be explained as one coverage note rather than repeated empty rows. Unknown subagent counts SHALL never appear as zero. Recent observations SHALL use the associated native state colour when available. The header SHALL identify the local observation host or client and display prominent Working, Blocked and Done totals.

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
The display SHALL use the active theme's semantic Herdr state roles, with Working amber/yellow, Blocked red, and Done/Idle green; Idle retains its own label and glyph. Clicking a card or activating it by keyboard SHALL open a read-only in-page inspector showing exact available metrics, current state, identity, freshness and bounded recent observations for that thread. It SHALL preserve focus, close with Escape and close when its permitted source disappears. It SHALL not send input to Herdr or add scrolling to the document. The footer SHALL omit the sampling disclaimer and music SHALL omit the source playback label.

#### Scenario: Inspect a thread
- **WHEN** a user activates a permitted card and observations arrive
- **THEN** its inspector updates only from that thread's available metadata and recent observations, while the fleet remains passive.

#### Scenario: Source loss
- **WHEN** the inspected thread becomes unavailable or leaves the selected disclosure view
- **THEN** the inspector closes without retaining stale private details.

## MODIFIED Requirements

### Requirement: Passive fleet observation
The dashboard SHALL collect Herdr agent metadata from configured local and SSH hosts and explicitly configured Work feed files without controlling panes. Feed persistence SHALL be limited to the configured private display state path.

#### Scenario: Working agent
- **WHEN** a valid snapshot contains a working agent
- **THEN** the dashboard shows its machine, project, harness, safe checkout label when reported and prominent working state.

#### Scenario: Collector failure and recovery
- **WHEN** a host times out or returns invalid data
- **THEN** that host is unavailable and its previous agents are not counted as working; other hosts continue and a later valid sample restores it.

## ADDED Requirements

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
The music tile SHALL emphasise track title and artist and play a bounded, theme-aware in-place text effect on an observed track change. Duplicate samples, initial connection, pause/resume and stale-source recovery SHALL NOT trigger track-change effects. Effects SHALL remain outside thread cards, have no consecutive repeats, finish normally before another starts, and release resources on hidden/reduced-motion/stale/error paths with readable text retained.

#### Scenario: Track transition
- **WHEN** a fresh source changes from one track identity to another
- **THEN** the music text receives one effect while artist and native text remain accessible.

#### Scenario: Unavailable or rapid changes
- **WHEN** the source expires or another track arrives during playback
- **THEN** stale effects are removed or allowed to finish without replacing the current readable metadata; at most the latest pending identity is retained, with no replay backlog.

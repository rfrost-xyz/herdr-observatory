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
The dashboard SHALL show sampled status transitions and available resource metrics with timestamps and collection scope; unavailable metrics SHALL remain unavailable. Graphics utilisation SHALL identify whether it measures visible NVIDIA devices or the busiest Intel Xe device engine.

#### Scenario: Missing GPU and stale browser
- **WHEN** GPU telemetry is unavailable or the browser loses its connection
- **THEN** GPU values are marked unavailable and stale browser activity stops appearing live.

#### Scenario: WSL NVIDIA graphics
- **WHEN** the NVIDIA GPU device and read-only driver tools are explicitly exposed to the collector
- **THEN** a valid `nvidia-smi` reading produces utilisation for visible NVIDIA devices, with the existing VRAM totals.

#### Scenario: Remote fleet view of WSL graphics
- **WHEN** an SSH collector cannot access the WSL GPU device but is configured to read the remote dashboard's loopback state
- **THEN** it uses only the matching host's fresh, valid NVIDIA aggregate, without forwarding the other dashboard's agent data or invoking Docker.

#### Scenario: Intel Xe device graphics
- **WHEN** an isolated monitor publishes a fresh aggregate of Intel Xe device counters
- **THEN** the dashboard shows measured utilisation for the busiest available engine, labelled as device scope, without inventing dedicated VRAM or exposing process data.

#### Scenario: Invalid or incomplete graphics counters
- **WHEN** a graphics source is missing, unreadable, malformed, resets, or has no matching interval
- **THEN** its percentage remains unavailable; other host metrics and agents remain available.

### Requirement: Omarchy display
The dashboard SHALL adopt valid active Omarchy palette changes without modifying desktop configuration and provide a usable fallback, responsive layout, keyboard controls and reduced-motion support. Thread identity, activity, metric and provenance modules SHALL remain reachable when their content exceeds a display row.

#### Scenario: Theme changes
- **WHEN** the selected theme source publishes a different valid palette
- **THEN** the next successful refresh updates dashboard colours; missing or malformed colours use safe defaults.

#### Scenario: Short display viewport
- **WHEN** the viewport cannot fit two full rows of thread details
- **THEN** thread card rows grow to contain their modules and the thread area scrolls to reach them without clipping the footer.

#### Scenario: Narrow tiled viewport
- **WHEN** a browser tile cannot fit the desktop column count
- **THEN** the layout reduces columns, sizes rows from their modules and keeps the cards reachable without horizontal document clipping.

#### Scenario: Initial connection
- **WHEN** the browser is waiting for its first state sample
- **THEN** the connection label has a reduced-motion-aware block loading indicator that disappears when that wait ends; measured percentages remain determinate gauges.

### Requirement: Local access boundary
The service SHALL bind to loopback and reject browser requests with untrusted Host or Origin headers and expose no agent mutation endpoints.

#### Scenario: Unexpected origin
- **WHEN** a request comes with an external Host or Origin
- **THEN** the service denies it without returning activity data.

### Requirement: Glanceable activity presentation
The display SHALL give thread cards persistent semantic state colour and a two-column top bar, with project and safe branch in the larger left region and a large icon-only native state in the right region. Current activity and reported tool SHALL sit beneath the state icon while remaining distinct from the native state. The branch SHALL sit in a subbox under the project name and inherit the state colour. Compact harness, host, pane and model metadata SHALL sit at the bottom left opposite compaction and source ages. Cards SHALL show validated context as the only dial, and cumulative session input, cumulative session output and cumulative cache-read share as three equally prominent non-dial readings beside context. Last-response counters MAY replace a missing cumulative instrument only with explicit response scope; they SHALL NOT repeat available session totals. Exact counts, source ages and coverage SHALL remain accessible. Missing usage SHALL have one visible cue and SHALL NOT appear as zero. Unknown subagent counts SHALL NOT become zero. Recent observations SHALL use native state colour when available. The header SHALL retain Idle, Working, Blocked and Done totals. Cards SHALL omit meaningless bare-repository checkout labels.

#### Scenario: Status header and footer
- **WHEN** a permitted thread has project, branch, state, activity and model metadata
- **THEN** the top bar gives the project two thirds with a state-coloured branch subbox, and gives the icon-only state one third with activity and tool beneath; harness, host, pane and model sit in the bottom left opposite age detail.

#### Scenario: Valid telemetry
- **WHEN** context, cumulative session tokens and cache composition are valid
- **THEN** the card shows one context dial and separate, equally prominent input, output and cached-input readings without clipping or overlap.

#### Scenario: Partial hook coverage
- **WHEN** activity exists but usage counters are absent
- **THEN** the state and current activity/tool remain readable, and one pending-usage cue appears.

#### Scenario: Response-only counters
- **WHEN** cumulative counters are unavailable but last-response counters are valid
- **THEN** the input and output readings visibly name their response scope, cache scope is visible when valid, and missing context remains unknown.

#### Scenario: Stale usage
- **WHEN** retained usage is older than the latest hook
- **THEN** its instrument remains visibly dated regardless of native state.

#### Scenario: Completed thread
- **WHEN** a thread is Done, including a tool observation associated with that state
- **THEN** its card and observation use the theme's green and the Done total counts it.

#### Scenario: Checkout disclosure
- **WHEN** a permitted thread has a reported checkout directory
- **THEN** only its sanitised leaf label reaches the browser and Work feed; full paths and excluded projects remain undisclosed.

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

### Requirement: Herdr state roles and thread inspection
The display SHALL use the active theme's semantic Herdr state roles, with Working amber/yellow, Blocked red, Done green and Idle muted like its header total. Cards SHALL persistently show bounded technical detail with readable labels and visual instruments. Hook activity and numeric usage SHALL retain independent source ages; a fresh hook SHALL NOT make older usage appear fresh or dim freshly measured values. Older hook activity SHALL be worded as a dated observation without changing Herdr state. Hover and focus SHALL provide visual emphasis only; no information SHALL depend on hover or open a separate click inspector. Clicking or keyboard-activating a tile SHALL play a brief local decorative response respecting reduced motion, using a bounded decorative glitch without synthetic activity or Herdr input. Detail SHALL clear when its permitted source disappears. The footer SHALL omit the sampling disclaimer and music SHALL omit the source playback label.

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

#### Scenario: Independent hook and usage ages
- **WHEN** a fresh hook carries older Codex usage or Pi last-response usage
- **THEN** hook activity and usage have separate visible ages, only values from the older source receive last-known styling, and fresh Pi context and cumulative totals retain their current-hook presentation.

#### Scenario: Historical tool observation
- **WHEN** the latest hook activity is older than two minutes while the pane remains present
- **THEN** the activity is labelled as a past observation with its age, while the state badge continues to show Herdr's current state.

#### Scenario: Decorative activation
- **WHEN** a card is clicked or activated by keyboard
- **THEN** a bounded local effect responds without opening a dialogue or generating an activity observation.

### Requirement: Compact measured metric panels
Fleet machines SHALL show CPU, memory, disk and network in subtly coloured theme-aware bordered panels with instruments using their available height inspired by the supplied terminal monitor. Histories SHALL contain distinct measured samples, stay bounded and stop showing live traces for stale sources. Thread panels SHALL prioritise available session totals, context percentage and cache/compaction data with readable scope, compact visible counts and exact accessible values. A complete cache-read balance SHALL explicitly name total input as the percentage denominator and show separate read, uncached and cache-write parts when writes are reported, without implying request hit/miss counts. The display SHALL retain eight-card paging, readable labels without document scrolling at 720p and 1080p, and reduced-motion support.

#### Scenario: Measured fleet history
- **WHEN** successive fresh host samples arrive
- **THEN** compact traces update from those measurements; repeated polls add no duplicate measurement and missing values are not zeroes.

#### Scenario: Thread usage
- **WHEN** a fresh harness reports cumulative input/output, context and cache counters
- **THEN** the card displays their distinct scopes and a source-grounded cache-read token percentage without presenting last-response counters as totals or cached tokens as request counts.

#### Scenario: Cache writes
- **WHEN** Pi reports a complete input balance containing cache writes
- **THEN** read, uncached and write token counts remain separately labelled and the three gauge segments add to the reported input total.

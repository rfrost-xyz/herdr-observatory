# Office display

## Purpose
Provide a continuously visible, private office activity display that operates independently of the remote operator's laptop.

## Requirements

### Requirement: Single-screen activity
The display SHALL fit 1280x720 and 1920x1080 16:9 viewports without document scrolling, with bounded process pages and a count of all permitted live panes. Thread cards SHALL occupy the majority of the viewport, with a compact activity header, labelled fleet strip and small recent-activity area, including after resize.

#### Scenario: Many agents
- **WHEN** more agents exist than fit on one page
- **THEN** the total includes all permitted live agents and labelled pages rotate with keyboard manual navigation that holds the selected page until rotation is resumed.

#### Scenario: Viewport resize
- **WHEN** the browser viewport changes size
- **THEN** the card composition and bounded activity effects resize to the viewport without document scrolling.

### Requirement: Work-only publication
The publisher SHALL send only explicitly selected hosts' Work agents, sanitised telemetry and palette through authenticated SSH; it SHALL omit personal agent titles, paths, session identifiers and history before transmission.

#### Scenario: Mixed activity
- **WHEN** the laptop manages personal and work agents
- **THEN** the office feed contains only permitted work agents even though the laptop Personal display includes both.

### Requirement: Feed expiry and theme continuity
The workstation SHALL expire stale laptop activity within 30 seconds and preserve its last valid received theme across display-service restart.

#### Scenario: Laptop offline
- **WHEN** the laptop stops publishing
- **THEN** its agents and telemetry become unavailable while workstation agents continue and the theme remains the last received palette.

### Requirement: Independent Work service
The workstation SHALL run its own loopback Work dashboard independently of the laptop and interactive SSH session, with a documented Windows full-screen launcher.

#### Scenario: Service restart
- **WHEN** the dashboard service restarts
- **THEN** it resumes the Work profile, local collection and the latest valid feed without starting or controlling Herdr panes.

### Requirement: Reproducible supervised deployment
Both profiles SHALL run from versioned application images under Compose with health checks, bounded logs and restart policies, independently of development checkouts. Runtime mounts SHALL be limited to explicit configuration, Herdr integration, palette and feed paths; the application SHALL NOT mount the Docker socket or an entire home directory.

#### Scenario: Restart and update
- **WHEN** the dashboard container or Docker engine restarts
- **THEN** the configured profile resumes without an interactive development session, and a documented previous image can be used for rollback.

### Requirement: Bounded socket observation
Local container collection SHALL issue only the read-only session snapshot request to a configured Herdr socket, bound response size and time, and preserve existing disclosure filtering.

#### Scenario: Invalid socket response
- **WHEN** Herdr is absent or returns malformed, oversized or mismatched data
- **THEN** the source is unavailable while independent sources continue and later valid samples restore it.

### Requirement: Differentiated technical activity
The display SHALL show readable fleet resource metrics and available typed hook activity/usage metadata without implying measured model throughput, while retaining one-screen geometry and reduced-motion support.

#### Scenario: Multiple working entities
- **WHEN** multiple machines or threads are working
- **THEN** their current state and available counters remain readable, with reactions reserved for observed transitions.

#### Scenario: Missing or private telemetry
- **WHEN** metadata is unavailable, malformed or belongs to an excluded Personal agent
- **THEN** unavailable fields remain marked unavailable and excluded metadata never reaches the Work feed or display.

### Requirement: Reactive rendered terminal
The display SHALL show an unbranded live thread-card display alongside text effects confined to individual incoming status-event lines in the recent-activity strip. Effects SHALL use current theme colours, complete naturally without replacement by routine updates, and never repeat consecutively. A 120-second default cooldown SHALL begin after completion and be adjustable with Left/Right in one-second increments. Renderer failure SHALL fall back to the live display. All 37 bundled effects SHALL be available in a shuffled rotation, generated locally in the browser from disclosed event lines without animation-frame downloads. Profile filtering, accessibility and reduced-motion support SHALL remain.

#### Scenario: Agent milestone
- **WHEN** an agent appears, changes status or disappears from a fresh source
- **THEN** the display updates the current state and records the observed change once, without claiming unsampled tool execution.

#### Scenario: Source unavailable
- **WHEN** a source or browser connection is lost
- **THEN** the display records loss once, removes stale agents from the live listing and does not report them as completed.

#### Scenario: Accessible one-screen rendering
- **WHEN** the scene runs at 720p or 1080p, or reduced motion is requested
- **THEN** the display remains bounded to one screen with an accessible text equivalent and reduced motion suppresses dynamic effects.

#### Scenario: Complete playback and hold
- **WHEN** fresh data arrives during an effect
- **THEN** playback continues through the final frame, followed by the configured hold before a later eligible incoming status line may start a different effect.

#### Scenario: Theme and controls
- **WHEN** the user presses Left or Right
- **THEN** the hold changes by one second and the unbranded display retains theme-derived coloured effects only on the affected event line while thread cards and fleet strip remain live and readable.

#### Scenario: Complete catalogue
- **WHEN** a full rotation completes
- **THEN** each of the 37 effects has played once on eligible arriving lines, including across rotation boundaries without consecutive repeats, and each finishes naturally.

#### Scenario: Browser renderer unavailable
- **WHEN** WebAssembly cannot initialise or an effect fails
- **THEN** current thread state remains readable and collection continues.

#### Scenario: Quiet or busy feed
- **WHEN** no new status event arrives, or one arrives during playback/cooldown
- **THEN** no old record is replayed; incoming records display immediately and do not interrupt playback.

#### Scenario: In-place playback
- **WHEN** an incoming status event receives an effect
- **THEN** only that line animates at its existing left edge and scrolling row, without horizontal or vertical block centring or covering adjacent records; if the line scrolls out of view, its playback is discarded without later replay.

### Requirement: Scrolling observation console
The activity strip SHALL scroll timestamped, bounded observations only as notable agent lifecycle/status and source-availability changes are observed. Duplicate samples SHALL NOT create duplicate transitions. Status stamps SHALL distinguish observed state changes from raw events or tool execution. The current thread cards SHALL remain separate and readable. Readiness, launch flags and revision/sequence changes SHALL use only sanitised available metadata in the thread cards. The activity strip SHALL use up to four recent events, each on one bounded line containing observation time, project, Herdr state, available thread number and pertinent status update without inventing activity. Unavailable fields SHALL be marked unavailable.

#### Scenario: Fresh and duplicate observations
- **WHEN** an agent changes between idle, working, blocked or done and a duplicate snapshot follows
- **THEN** one timestamped transition is appended and the thread state updates; routine samples and metadata counters add no log rows, while notable changes scroll in once.

#### Scenario: Sparse milestone artwork
- **WHEN** a fresh done or blocked transition is observed
- **THEN** the event stays on one line without decorative project-name artwork, and may receive an in-place text effect when eligible.

#### Scenario: Animation accessibility and disclosure
- **WHEN** motion is paused, reduced motion is requested, a source expires, or an activity-line effect is playing
- **THEN** no fabricated activity is added, effects stay outside thread cards and retain their completion semantics, obsolete pending effect starts are discarded, and current disclosed state remains accessible.

#### Scenario: Denser padded terminal
- **WHEN** the browser renders at 720p or 1080p
- **THEN** thread cards and the activity strip retain modest side padding, fit without document scrolling and retain keyboard controls after resizing.

### Requirement: Modern glyph-capable terminal
The display SHALL provide a modern Omarchy-inspired card composition with locally served Nerd Font glyphs, Unicode separators, clear thread-state labels and consistent typography in live and animated views. Icons SHALL supplement readable words. Font availability SHALL NOT prevent current state from rendering.

#### Scenario: Locally rendered glyphs
- **WHEN** a browser loads the display without installed Nerd Fonts
- **THEN** the bundled font renders state and section glyphs without external font requests, and activity-line effects use the same face.

#### Scenario: Font failure and accessibility
- **WHEN** the font cannot load or reduced motion is enabled
- **THEN** readable state labels, keyboard controls and the accessible current-state transcript remain available.

#### Scenario: Modern one-screen composition
- **WHEN** threads and events arrive at 720p or 1080p
- **THEN** prominent cards, a compact activity header and labelled fleet strip retain clear thread state above the recent-activity strip, within the existing padded viewport.

### Requirement: State-driven thread motion
Thread cards SHALL use Herdr's Working, Blocked, Done, Idle and Unknown state names. Working state indicators SHALL animate independently, and newly observed state or telemetry changes SHALL briefly highlight the affected card. Unchanged samples SHALL NOT retrigger arrival highlights. Card impulses and a compact header activity indicator SHALL respond only to observed state or supported hook changes and settle when quiet. Text effects SHALL NOT run inside cards. Motion SHALL stop for stale/disconnected, paused and reduced-motion views, without obscuring state text or using travelling horizontal highlights.

#### Scenario: Independent working rows
- **WHEN** multiple permitted working threads are current
- **THEN** their state glyphs move at independently phased timings while state labels remain readable.

#### Scenario: Settled or unavailable rows
- **WHEN** a thread is idle, done or unknown, the source becomes stale, or motion is paused/reduced
- **THEN** no ongoing working animation is shown for that card.

#### Scenario: Observed activity
- **WHEN** a fresh state or telemetry sequence changes
- **THEN** the affected card receives one bounded highlight, without duplicate samples restarting it.

### Requirement: Readable fleet and theme integration
Fleet information SHALL use clear labels for processor, memory, graphics, storage, network traffic and sample age, retaining host identity and Online/Offline state. The synchronised OS theme SHALL colour the entire display, including surfaces, borders and status accents, with a visible theme name and readable light and dark presentations.

#### Scenario: Missing fleet metrics
- **WHEN** a source or metric is unavailable
- **THEN** its label remains understandable and the value is explicitly unavailable, never inferred as zero.

#### Scenario: Theme update
- **WHEN** a new valid OS palette arrives
- **THEN** cards, header, fleet and activity effects adopt it without modifying OS configuration or adding a service.

#### Scenario: Hook-driven card detail
- **WHEN** fresh supported hook telemetry arrives
- **THEN** the card shows the available phase, tool, model and usage/context information, briefly reacts once, and preserves Herdr's lifecycle state as authority.

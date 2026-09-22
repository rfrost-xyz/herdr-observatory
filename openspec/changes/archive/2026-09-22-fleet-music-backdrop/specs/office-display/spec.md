## MODIFIED Requirements

### Requirement: Single-screen activity
The display SHALL fit 1280x720 and 1920x1080 16:9 viewports without document scrolling, with bounded process pages and a count of all permitted live panes. Thread cards SHALL occupy the majority of the viewport, with a compact activity header, labelled fleet strip and small recent-activity area, including after resize.

#### Scenario: Many agents
- **WHEN** more agents exist than fit on one page
- **THEN** the total includes all permitted live agents and labelled eight-thread pages rotate every 15 seconds by default when more than eight permitted threads exist, with keyboard manual navigation that holds the selected page until rotation is resumed.

#### Scenario: Viewport resize
- **WHEN** the browser viewport changes size
- **THEN** the card composition and bounded activity effects resize to the viewport without document scrolling.

### Requirement: Reproducible supervised deployment
Both profiles SHALL run from versioned application images under Compose with health checks, bounded logs and restart policies, independently of development checkouts. Runtime mounts SHALL be limited to explicit configuration, Herdr integration, optional music socket, palette and feed paths; the application SHALL NOT mount the Docker socket or an entire home directory.

#### Scenario: Restart and update
- **WHEN** the dashboard container or Docker engine restarts
- **THEN** the configured profile resumes without an interactive development session, and a documented previous image can be used for rollback.

### Requirement: Reactive rendered terminal
The display SHALL show an unbranded live thread-card display alongside text effects confined to individual incoming status-event lines in the recent-activity strip. Effects SHALL use current theme colours, complete naturally without replacement by routine updates, and never repeat consecutively. A fixed internal 120-second cooldown SHALL begin after completion. The display SHALL omit effect-timer and pause controls. Renderer failure SHALL fall back to the live display. All 37 bundled effects SHALL be available in a shuffled rotation, generated locally in the browser from disclosed event lines without animation-frame downloads. Profile filtering, accessibility and reduced-motion support SHALL remain.

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
- **WHEN** the user views the display
- **THEN** there is no timer or pause control, the internal cooldown remains 120 seconds, and theme-derived effects remain confined to the affected event line.

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
- **WHEN** reduced motion is requested, a source expires, or an activity-line effect is playing
- **THEN** no fabricated activity is added, effects stay outside thread cards and retain their completion semantics, obsolete pending effect starts are discarded, and current disclosed state remains accessible.

#### Scenario: Denser padded terminal
- **WHEN** the browser renders at 720p or 1080p
- **THEN** thread cards and the activity strip retain modest side padding, fit without document scrolling and retain keyboard controls after resizing.

### Requirement: State-driven thread motion
Thread cards SHALL use Herdr's Working, Blocked, Done, Idle and Unknown state names. Working state indicators SHALL animate independently, and newly observed state or telemetry changes SHALL briefly highlight the affected card. Unchanged samples SHALL NOT retrigger arrival highlights. Card impulses and a compact header activity indicator SHALL respond only to observed state or supported hook changes and settle when quiet. Text effects SHALL NOT run inside cards. Motion SHALL stop for stale/disconnected and reduced-motion views, without obscuring state text or using travelling horizontal highlights.

#### Scenario: Independent working rows
- **WHEN** multiple permitted working threads are current
- **THEN** their state glyphs move at independently phased timings while state labels remain readable.

#### Scenario: Settled or unavailable rows
- **WHEN** a thread is idle, done or unknown, the source becomes stale, or reduced motion is requested
- **THEN** no ongoing working animation is shown for that card.

#### Scenario: Observed activity
- **WHEN** a fresh state or telemetry sequence changes
- **THEN** the affected card receives one bounded highlight, without duplicate samples restarting it.

### Requirement: Readable fleet and theme integration
Fleet information SHALL use clear labels for processor, memory, graphics, storage, network traffic and sample age, retaining host identity and Online/Offline state. Processor, memory, graphics and storage SHALL have bounded percentage gauges; unknown values SHALL be visually distinct from zero. Thread identity SHALL label its harness, native pane ID and host. The synchronised OS theme SHALL colour the entire display, including surfaces, borders and status accents, with a visible theme name and readable light and dark presentations.

#### Scenario: Missing fleet metrics
- **WHEN** a source or metric is unavailable
- **THEN** its label remains understandable and the value is explicitly unavailable, never inferred as zero.

#### Scenario: Theme update
- **WHEN** a new valid OS palette arrives
- **THEN** cards, header, fleet and activity effects adopt it without modifying OS configuration or adding a service.

#### Scenario: Hook-driven card detail
- **WHEN** fresh supported hook telemetry arrives
- **THEN** the card shows the available phase, tool, model and usage/context information, briefly reacts once, and preserves Herdr's lifecycle state as authority.

## ADDED Requirements

### Requirement: Optional shared music background
An explicitly configured music source SHALL provide only bounded playback state, title, artist, real spectrum bands and capture time. Authorised sharing SHALL carry these fields independently of Work agent filtering and SHALL NOT transmit paths, artwork URLs, provider metadata or audio. Disabled, missing, invalid or older-than-three-second music SHALL be unavailable without affecting agent collection. The service SHALL use existing containers and authenticated transport without an additional installed daemon.

#### Scenario: Shared playback
- **WHEN** iapetus plays through the configured cliamp socket and music publication is enabled
- **THEN** both displays show the permitted track title and the background reacts to current spectrum data, independently of thread activity.

#### Scenario: Playback unavailable
- **WHEN** playback is paused, stopped, invalid or stale, or the music source disconnects
- **THEN** audio-driven motion stops; stale titles disappear and agent collection remains operational.

#### Scenario: Music disclosure
- **WHEN** playback metadata contains extra fields or music sharing is not configured
- **THEN** extra fields never reach a client and unconfigured music is not collected or published.

### Requirement: Theme-aware interactive pixel field
The display SHALL render a bounded Omarchy-inspired pixel field behind its content, using the synchronised theme. Background pointer and click interactions SHALL produce local bounded reactions without triggering through cards or controls. Real music spectrum SHALL influence the field only while fresh and playing. Motion SHALL be suppressed in hidden and reduced-motion views, preserving readable foreground state and one-screen geometry.

#### Scenario: Background interaction
- **WHEN** the user clicks an exposed background area
- **THEN** a bounded local pixel impulse appears behind the content without animating thread text.

#### Scenario: Motion accessibility
- **WHEN** reduced motion is requested or the tab is hidden
- **THEN** background animation and music-driven motion stop without interrupting collection or changing playback.

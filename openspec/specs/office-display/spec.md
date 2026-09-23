# Office display

## Purpose
Provide a continuously visible, private office activity display that operates independently of the remote operator's laptop.

## Requirements

### Requirement: Single-screen activity
The display SHALL fit 1280×720 and 1920×1080 16:9 viewports, and 1280×800 and 1920×1200 16:10 viewports, without document or thread-grid scrolling for up to eight permitted threads. Thread cards SHALL remain fully contained in a four-column, two-row page and occupy the majority of the viewport with a compact title, fleet strip and recent-activity area. Narrow tiled browsers MAY scroll their own content. The count SHALL include all permitted live panes.

#### Scenario: Eight visible threads
- **WHEN** a page contains eight permitted threads with maximum supported card content
- **THEN** all eight cards and their current state, activity and visual telemetry fit on screen without clipping or scrolling.

#### Scenario: Many agents
- **WHEN** more agents exist than fit on one page
- **THEN** the total includes all permitted live agents and labelled eight-thread pages rotate every 15 seconds by default, with keyboard manual navigation that holds the selected page until rotation resumes.

#### Scenario: Viewport resize
- **WHEN** the browser resizes between supported full-screen proportions
- **THEN** card composition and bounded activity effects resize without document or thread-grid scrolling.

#### Scenario: Narrow browser tile
- **WHEN** browser width is below full-screen display width
- **THEN** cards reduce columns and stay reachable without horizontal document clipping.

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
Both profiles SHALL run from versioned application images under Compose with health checks, bounded logs and restart policies, independently of development checkouts. Runtime mounts SHALL be limited to explicit configuration, Herdr integration, optional music socket, palette, feed and opt-in aggregate graphics sources; the application SHALL NOT mount the Docker socket, host process filesystem or an entire home directory.

#### Scenario: Restart and update
- **WHEN** the dashboard container or Docker engine restarts
- **THEN** the configured profile resumes without an interactive development session, and a documented previous image can be used for rollback.

#### Scenario: Optional graphics integration
- **WHEN** an operator enables a host-specific graphics source
- **THEN** the dashboard retains its user, capabilities and network exposure; any privileged GPU monitor has no host process or home mount and publishes only an aggregate.

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
The display SHALL show an unbranded live thread-card display alongside event text effects confined to individual incoming status-event lines in the recent-activity footer. Effects SHALL use current theme colours, complete naturally without replacement by routine updates, and never repeat consecutively. A fixed internal 120-second cooldown SHALL begin after completion. The display SHALL omit effect-timer and pause controls. Renderer failure SHALL fall back to the live display. All 37 bundled effects SHALL be available in a shuffled rotation, generated locally in the browser from disclosed event lines without animation-frame downloads. Profile filtering, accessibility and reduced-motion support SHALL remain.

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
The activity strip SHALL scroll timestamped, bounded observations only as notable agent lifecycle/status and source-availability changes are observed. Duplicate samples SHALL NOT create duplicate transitions. Status stamps SHALL distinguish observed state changes from raw events or tool execution. The current thread cards SHALL remain separate and readable. Readiness, launch flags and revision/sequence changes SHALL use only sanitised available metadata in the thread cards. The activity strip SHALL sit inside an opaque full-width footer split horizontally between observations and account allowances and excluded from the music visualiser and use up to four recent events with semantic icons and theme-derived colours, each on one bounded line containing observation time, project, Herdr state, available thread number and pertinent status update without inventing activity. Unavailable fields SHALL be marked unavailable.

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
Thread cards SHALL use Herdr's Working, Blocked, Done, Idle and Unknown state names. Working state indicators SHALL animate independently, and newly observed state or telemetry changes SHALL briefly highlight the affected card. Unchanged samples SHALL NOT retrigger arrival highlights. Card impulses SHALL respond only to observed state or supported hook changes and settle when quiet. Automatic text effects SHALL NOT run inside cards; explicit click or keyboard activation SHALL permit a brief local decorative glitch, independent of sampled activity. Motion SHALL stop for stale/disconnected and reduced-motion views, without obscuring state text or using travelling horizontal highlights.

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
Fleet information SHALL use clear labels for processor, memory, graphics, storage, network traffic and sample age, retaining host identity and Online/Offline state. Processor, memory, graphics and storage SHALL have bounded percentage gauges; unknown values SHALL be visually distinct from zero. A measured graphics percentage SHALL explain its source and scope. Thread identity SHALL label its harness, native pane ID and host. The synchronised OS theme SHALL colour the entire display, including surfaces, borders and status accents, with readable light and dark presentations and no visible theme-name label.

#### Scenario: Missing fleet metrics
- **WHEN** a source or metric is unavailable
- **THEN** its label remains understandable and the value is explicitly unavailable, never inferred as zero.

#### Scenario: Theme update
- **WHEN** a new valid OS palette arrives
- **THEN** cards, header, fleet and activity effects adopt it without modifying OS configuration or adding a service.

#### Scenario: Hook-driven card detail
- **WHEN** fresh supported hook telemetry arrives
- **THEN** the card shows the available phase, tool, model and usage/context information, briefly reacts once, and preserves Herdr's lifecycle state as authority.

#### Scenario: Measured graphics scope
- **WHEN** the graphics gauge has a valid reading
- **THEN** its accessible description distinguishes visible NVIDIA device utilisation from the busiest Intel Xe device engine.

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
The display SHALL render a bounded Omarchy-inspired pixel field behind its content, using muted inks derived from the synchronised theme accent, mixed towards the theme background even during music or click peaks. Its canvas SHALL end above the observations footer. Background pointer and click interactions SHALL produce local bounded reactions without triggering through cards or controls. Real music spectrum SHALL influence the field only while fresh and playing. Motion SHALL be suppressed in hidden and reduced-motion views, preserving readable foreground state and one-screen geometry.

#### Scenario: Background interaction
- **WHEN** the user clicks an exposed background area
- **THEN** a bounded local pixel impulse appears behind the content without animating thread text.

#### Scenario: Motion accessibility
- **WHEN** reduced motion is requested or the tab is hidden
- **THEN** background animation and music-driven motion stop without interrupting collection or changing playback.

### Requirement: Personal title artwork
The header SHALL display a compact Rich mark in scaled Delta Corps Priest 1 artwork, replacing the generic title and observed-activity widget. The mark SHALL play a local text effect when clicked and once per minute while visible. Title effects SHALL remain isolated from thread cards, event effects and music, complete without click interruption and avoid consecutive repeats. Static artwork and an accessible Rich label SHALL remain available on renderer failure or reduced motion; hidden views SHALL suspend or cancel animation without queuing missed automatic effects.

#### Scenario: Title activation
- **WHEN** the title is clicked or a visible minute elapses
- **THEN** an effect plays within the small title area; clicks during playback do not interrupt it or queue another effect.

#### Scenario: Unavailable animation
- **WHEN** reduced motion is requested, the tab is hidden or the renderer fails
- **THEN** the title retains a static accessible identity, releases obsolete effect resources and does not replay missed intervals on return.

#### Scenario: Independent presentation
- **WHEN** title effects run while agents are quiet or disconnected
- **THEN** only the title animates and no agent event, state or observation is fabricated.

### Requirement: Tiled browser presentation
The browser display SHALL remain usable in narrower and shorter Omarchy tiling windows as well as the established 1280x720 and 1920x1080 Windows and Omarchy layouts. It SHALL preserve readable fleet, thread and account information through reflow or bounded internal scrolling, keep the current-state text accessible, and stop decorative motion for hidden or reduced-motion views. Its connection-loading indicator SHALL use the selected Blocks component and SHALL not animate when the connection is established or reduced motion is requested. Browser assets SHALL load locally without a runtime package manager or external CDN.

#### Scenario: Narrow tile
- **WHEN** the browser is resized to a narrow tiled viewport
- **THEN** controls, fleet values, thread cards and account activity remain reachable without horizontal document clipping.

#### Scenario: Full display
- **WHEN** the browser uses a 720p or 1080p full display
- **THEN** the existing one-screen composition, card motion and bounded event effects remain functional.

#### Scenario: Connection and accessibility
- **WHEN** the connection is pending, established or lost, or reduced motion is requested
- **THEN** the loading indicator follows that state, text names the connection state and reduced motion suppresses the animation.

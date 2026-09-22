## MODIFIED Requirements

### Requirement: Reactive rendered terminal
The display SHALL show an unbranded live thread TUI alongside text effects confined to the events CLI. Effects SHALL use current theme colours, complete naturally without replacement by routine updates, and never repeat consecutively. A 120-second default hold SHALL begin after completion and be adjustable with Left/Right in one-second increments. Renderer failure SHALL fall back to the live TUI. All 37 bundled effects SHALL be available in a shuffled rotation, generated locally in the browser from disclosed snapshots without animation-frame downloads. Profile filtering, accessibility and reduced-motion support SHALL remain.

#### Scenario: Agent milestone
- **WHEN** an agent appears, changes status or disappears from a fresh source
- **THEN** the TUI updates the current state and records the observed change once, without claiming unsampled tool execution.

#### Scenario: Source unavailable
- **WHEN** a source or browser connection is lost
- **THEN** the terminal records loss once, removes stale agents from the live listing and does not report them as completed.

#### Scenario: Accessible one-screen rendering
- **WHEN** the scene runs at 720p or 1080p, or reduced motion is requested
- **THEN** the terminal remains bounded to one screen with an accessible text equivalent and reduced motion suppresses dynamic effects.

#### Scenario: Complete playback and hold
- **WHEN** fresh data arrives during an effect
- **THEN** playback continues through the final frame, followed by the configured hold before a different effect begins.

#### Scenario: Theme and controls
- **WHEN** the user presses Left or Right
- **THEN** the hold changes by one second and the unbranded terminal retains theme-derived coloured effects only inside the events CLI while thread and machine rows remain live and readable.

#### Scenario: Complete catalogue
- **WHEN** a full rotation completes
- **THEN** each of the 37 effects has played once, including across rotation boundaries without consecutive repeats, and each finishes naturally.

#### Scenario: Browser renderer unavailable
- **WHEN** WebAssembly cannot initialise or an effect fails
- **THEN** current thread state remains readable and collection continues.

### Requirement: Scrolling observation console
The live terminal SHALL scroll timestamped, bounded observations only as notable agent lifecycle/status and source-availability changes are observed. Duplicate samples SHALL NOT create duplicate transitions. Status stamps SHALL distinguish observed state changes from raw events or tool execution. The current thread list SHALL remain separate and readable. Readiness, launch flags and revision/sequence changes SHALL use only sanitised available metadata in the live table. The CLI SHALL use concise event text and local arrival effects without inventing activity.

#### Scenario: Fresh and duplicate observations
- **WHEN** an agent changes between idle, working, blocked or done and a duplicate snapshot follows
- **THEN** one timestamped transition is appended and the thread state updates; routine samples and metadata counters add no log rows, while notable changes scroll in once.

#### Scenario: Sparse milestone artwork
- **WHEN** a fresh done or blocked transition is observed
- **THEN** a brief themed Delta Corps Priest 1 project-name stamp may appear inside the expanded CLI, with event context and a cooldown preventing repeated large banners, without covering thread rows; ordinary observations use normal terminal text.

#### Scenario: Animation accessibility and disclosure
- **WHEN** motion is paused, reduced motion is requested, a source expires, or a whole-terminal effect is playing
- **THEN** no fabricated activity is added, effects retain their completion semantics, obsolete pending stamps are discarded, and current disclosed state remains accessible.

#### Scenario: Denser padded terminal
- **WHEN** the browser renders at 720p or 1080p
- **THEN** live text and effects share a denser grid with modest left/right padding, fit without document scrolling and retain keyboard controls after resizing.

### Requirement: Modern glyph-capable terminal
The display SHALL provide a modern themed TUI with locally served Nerd Font glyphs, Unicode separators, clear thread-state labels and consistent typography in live and animated views. Icons SHALL supplement readable words. Font availability SHALL NOT prevent current state from rendering.

#### Scenario: Locally rendered glyphs
- **WHEN** a browser loads the display without installed Nerd Fonts
- **THEN** the bundled font renders state and section glyphs without external font requests, and CLI effects use the same face.

#### Scenario: Font failure and accessibility
- **WHEN** the font cannot load or reduced motion is enabled
- **THEN** readable state labels, keyboard controls and the accessible current-state transcript remain available.

#### Scenario: Modern one-screen composition
- **WHEN** threads and events arrive at 720p or 1080p
- **THEN** themed glyph-led rows and Unicode separators retain clear thread state above the scrolling feed, within the existing padded viewport.

## ADDED Requirements

### Requirement: State-driven thread motion
Thread rows SHALL use Herdr's Working, Blocked, Done, Idle and Unknown state names. Working rows SHALL animate independently, and newly observed state or telemetry changes SHALL briefly highlight the affected row. Unchanged samples SHALL NOT retrigger arrival highlights. Motion SHALL stop for stale/disconnected, paused and reduced-motion views, without obscuring state text.

#### Scenario: Independent working rows
- **WHEN** multiple permitted working threads are current
- **THEN** their row indicators and highlights move at independently phased timings while state labels remain readable.

#### Scenario: Settled or unavailable rows
- **WHEN** a thread is idle, done or unknown, the source becomes stale, or motion is paused/reduced
- **THEN** no ongoing working animation is shown for that row.

#### Scenario: Observed activity
- **WHEN** a fresh state or telemetry sequence changes
- **THEN** the affected row receives one bounded highlight, without duplicate samples restarting it.

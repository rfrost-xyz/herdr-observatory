## MODIFIED Requirements

### Requirement: Menubar and popover only
The plugin SHALL provide a manifest-backed Omarchy bar widget with an anchored popover. It SHALL NOT install or launch a companion window. Display data SHALL come from the bundled native collector over a bounded private process pipe, preserving the existing Observatory disclosure rules. No local HTTP listener, Docker service or independent autostart daemon SHALL be required. The collector SHALL stop when the plugin owner pipe closes. Configured remote publication MAY continue only while the plugin is enabled. The QML presentation SHALL NOT read raw terminal output, credentials or agent session files, send input to agents or change their lifecycle. The plugin-owned collector MAY read only validated session-bound Codex records on configured sources for allowlisted numeric and lifecycle enrichment, with bounded execution and no transcript publication. An explicit thread activation MAY focus the exact live agent through Herdr and raise or open its terminal view.

#### Scenario: Open a thread
- **WHEN** the operator clicks a thread or selects it with arrows and presses Enter
- **THEN** the plugin resolves that thread's host and pane, focuses it through Herdr, and raises the most recently focused matching Herdr terminal or opens the corresponding view.
- **AND** an unknown or ambiguous host fails visibly without falling back to another machine.

### Requirement: Aggregate icon state
The bar SHALL use the native omarchy.agents glyph in the normal monochrome bar foreground, without a number. A static mark against the glyph's upper-right edge with slight overlap SHALL show aggregate state in the theme status colour: a red exclamation for blocked, a six-unit solid amber dot for working, a green tick for an unacknowledged done episode, and no mark for idle. Opening that exact thread successfully SHALL acknowledge its current done episode locally; a later working-to-done episode SHALL be eligible again. The Agents glyph itself SHALL stay monochrome. Priority SHALL be blocked, working, done, idle. The bar SHALL NOT pulse or use an underline. Unknown or unavailable sources SHALL NOT imply completion. Tooltips SHALL retain aggregate state and incomplete-source reporting.

#### Scenario: Mixed thread states
- **WHEN** blocked and done threads coexist
- **THEN** the monochrome glyph carries an exclamation, changing to a working dot if working threads remain, otherwise a tick while an unacknowledged done episode exists.

### Requirement: Truthful current state
A failed or expired host sample SHALL be unavailable rather than contributing zero threads. Codex allowance SHALL show only mapped accounts and valid source-dated weekly values. The continuous remaining-allowance bar SHALL distinguish the burn difference from its muted surrounding fill. Allowance bar, difference and numeric emphasis SHALL use one pace meaning: neutral when equal, green reserve and red deficit, with intensity increasing with the absolute gap and no hidden tolerance. Valid session counters SHALL remain visible while their live thread is reported, with older values softened and their original age shown, rather than blanked after two minutes. Missing and disconnected data SHALL stay explicit. All mapped accounts SHALL show days/hours until reset and valid reset-pass counts. The concise hover SHALL compare remaining and expected balance directly; visible pace SHALL avoid an unbounded relative ratio near reset.

#### Scenario: One host stops reporting
- **WHEN** a previously reporting host becomes unavailable
- **THEN** its threads disappear, its active-thread count becomes unavailable and reporting remains labelled as partial.

### Requirement: Restrained visual feedback
Fine inset separators SHALL distinguish adjacent threads. Working titles MAY show a visible periodic glyph sheen; idle, done and blocked titles SHALL NOT shimmer continuously. A newly observed state MAY briefly highlight its row and icon. Effects SHALL match stable host/thread identities across reordering, and SHALL NOT treat initial snapshots or source disappearance as state changes. Hovering allowance SHALL show dense sparks for both surplus and deficit confined to the measured difference interval, preserving the bar geometry. Highlighted allowance numbers SHALL brighten with a soft halo. Motion SHALL stop when the popover closes and support a reduced-motion override.

#### Scenario: Allowance hover
- **WHEN** an available allowance row is hovered
- **THEN** its positive or negative difference sparkles within that interval, stopping on mouse exit, source expiry or popover closure.

#### Scenario: A genuinely new thread arrives
- **WHEN** a previously unseen thread arrives while its reporting host and group are already visible
- **THEN** it enters with a restrained approximately 300 ms transition.
- **AND** initial hydration, source reconnect, popover reopening, filtering, reordering and expand/collapse SHALL NOT replay that entrance.

#### Scenario: Stable interaction and compaction
- **WHEN** state changes while the operator is interacting with the popover
- **THEN** existing rows retain their order, and a trustworthy increased compaction count may briefly highlight its context dial without inventing a count from incomplete history.

### Requirement: Native subagent completion
A separate completion dial SHALL use typed Codex completed states and parent activity records, never a ratio of start/stop events. Only bounded total/done counts, optional coherent running/interrupted/failed/unknown outcome partitions and original source timestamps SHALL enter the existing sanitised telemetry path. Older total/done-only sources SHALL remain usable without assigning unknown remaining children a running state. Completion SHALL NOT be presented as overall parent-task progress. Private subagent replay cursors SHALL contain only hashed associations, allowlisted states and bounded file cursor metadata, bounded to 32 sessions, 128 children per session, 256 KiB and 24 hours. Missing baseline, ambiguous identities and invalid data SHALL remain unknown. The session roster SHALL survive user prompts. The adapter SHALL incrementally read native `item_completed` / `SubAgentActivity` records with parent binding and original timestamps; incomplete replay SHALL remain unknown. Resuming work SHALL invalidate old completion. The existing adapter uninstaller SHALL remove its private cache.

#### Scenario: Completed child resumes
- **WHEN** native evidence confirms a child completed, and the parent subsequently sends more work to it
- **THEN** the dial counts the completion once and removes it when work resumes, until a fresh typed completion is observed.

#### Scenario: Duplicate completion and privacy
- **WHEN** repeated native wait/list results report the same completed child
- **THEN** it contributes once, and no child identifier, name, prompt or result appears in the public feed.


#### Scenario: Wrapped native lifecycle and prompt changes
- **WHEN** a saved parent `item_completed` record contains a typed `SubAgentActivity` completion and the user submits another prompt
- **THEN** the unique completed child remains in the session roster until a typed interaction or interruption changes its state, without requiring a list-agents tool call.


### Requirement: Collapsible groups and icon filters
Threads and Allowances section headers SHALL toggle their entire contents, with persistent preferences independent of nested groups. Machine headers SHALL toggle expansion with persistent preferences. Allowance SHALL contain one matching collapsible subsection per configured provider with its own persistent preference. Existing Codex accounts SHALL remain compatible. Expansion and collapse SHALL be instant. Collapsing an allowance provider SHALL NOT stop account collection. Four equal-width status icons at the top right SHALL independently toggle idle, blocked, working and done threads. Unknown states SHALL remain visible. Filtering and collapse SHALL NOT alter the menubar aggregate or allowance readings. Keyboard navigation SHALL skip hidden rows and empty groups, including all threads when the Threads section is collapsed. Unavailable machines SHALL remain distinguishable from filtered or collapsed machines.

#### Scenario: Hide idle work and collapse a machine
- **WHEN** idle is deselected and a machine is collapsed
- **THEN** idle threads and that machine's rows are omitted from display and keyboard navigation, while the machine header and unfiltered menubar state remain visible.
- **AND** the choices survive a shell restart.

#### Scenario: Collector owner exits
- **WHEN** the widget is disabled or the shell closes its collector pipe
- **THEN** collection and publication stop, without leaving a local listening service.
### Requirement: Private account identity display
Allowance rows SHALL display the verified account email instead of category labels. Clicking anywhere on any allowance row SHALL toggle all account emails together, using distinct randomly chosen Silicon Valley character aliases persisted locally. Names SHALL NOT be separate controls. Concealed email SHALL NOT appear in tooltip or accessibility text. Native read-only identity RPCs SHALL be matched against existing account mappings, with emails kept out of the shared API and source checkout. Uninstall SHALL remove the private identity file and preferences.

#### Scenario: Conceal account identity
- **WHEN** the operator clicks any allowance row
- **THEN** character aliases replace all emails and remain after a shell restart, until any row is clicked again.

## ADDED Requirements

### Requirement: Lean owner-bound native collection
The native plugin SHALL collect current thread, usage, lifecycle, account and connection state without unnecessary dashboard host metrics or observation histories. Explicitly configured publication and music SHALL retain their existing downstream contracts. Native Codex records on local and configured SSH sources SHALL update while the parent waits, independently of the next parent hook. Remote enrichment SHALL use the existing ephemeral read-only transport without remote installed files or services. Duplicate snapshots MAY be suppressed only while a bounded heartbeat preserves transport health independently of measurement freshness. The collector SHALL terminate with its owner and recover supported state after restart without another daemon or listener.

#### Scenario: Parent waits while a child completes
- **WHEN** a validated parent record on a local or configured SSH source reports a child completion without another parent hook
- **THEN** the next bounded native collection publishes the updated outcome with its original source timestamp.

#### Scenario: Idle transport and restart
- **WHEN** no measured value changes or the plugin restarts
- **THEN** transport health remains observable, previous usage timestamps are not renewed, and valid session bindings recover without publishing raw session material.

### Requirement: Provider and account collections
The native projection SHALL use stable provider/account identities for configured allowance rows and collapse preferences. Existing label-only mappings and private identity data SHALL remain compatible. Account expansion SHALL NOT broaden Work project disclosure or publish account email. Unsupported providers SHALL remain unavailable until an explicit adapter exists.

#### Scenario: More than two mapped accounts
- **WHEN** native configuration contains three valid mapped accounts
- **THEN** all appear under their provider with independent readings, shared concealment and no two-row limit.

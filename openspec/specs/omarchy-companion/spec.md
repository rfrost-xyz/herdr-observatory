# Omarchy popover Specification

## Purpose
Provide an Omarchy menubar status icon and compact popover for current Herdr threads, Codex allowance and fleet readings. Navigate to a thread only on explicit activation. Keep unavailable data explicit and installation reversible.

## Requirements

### Requirement: Menubar and popover only
The plugin SHALL provide a manifest-backed Omarchy bar widget with an anchored popover. It SHALL NOT install or launch a companion window. Display data SHALL come from the bundled native collector over a bounded private process pipe, preserving local privacy and numeric allowlists. No local HTTP listener, Docker service or independent autostart daemon SHALL be required. The collector SHALL stop when the plugin owner pipe closes. The plugin SHALL NOT publish Work or music feeds or require a web service. The QML presentation SHALL NOT read raw terminal output, credentials or agent session files, send input to agents or change their lifecycle. The plugin-owned collector MAY read only validated session-bound Codex records on configured sources for allowlisted numeric and lifecycle enrichment, with bounded execution and no transcript publication. An explicit thread activation MAY focus the exact live agent through Herdr and raise or open its terminal view. Remote activation SHALL resolve one enabled saved machine profile and its exact target/session, and SHALL work without requiring remote machine API forwarding. The bounded remote operation SHALL resolve the installed executable in the non-interactive environment. New terminal launches SHALL preserve the selected remote target and session without allowing a launcher alias to replace them with a default local command. Activation SHALL make one focus attempt, retain most-recently-selected matching window behaviour, and report a concise relevant failure without retrying another host or falling back to Local.

#### Scenario: Open a thread
- **WHEN** the operator clicks a thread or selects it with arrows and presses Enter
- **THEN** the plugin resolves that thread's host and pane, focuses it through Herdr, and raises the most recently focused matching Herdr terminal or opens the corresponding view.
- **AND** an unknown or ambiguous host fails visibly without falling back to another machine.

#### Scenario: Reachable remote host lacks machine API forwarding
- **WHEN** a clicked remote thread belongs to one enabled saved profile but the remote Herdr version lacks machine API forwarding
- **THEN** navigation focuses the exact pane on that profile's target and session over the supported authenticated transport, then raises the most recently focused matching terminal or opens that exact view.

#### Scenario: Non-interactive remote executable and navigation failure
- **WHEN** remote Herdr is installed at its established user-local location but absent from the non-interactive PATH
- **THEN** navigation resolves that executable for the same exact target and session.
- **AND** if the executable or connection is unavailable, it reports a concise relevant error after one attempt without opening another machine or sending agent input.

#### Scenario: Launch a missing remote view
- **WHEN** no existing terminal matches the selected remote target and session
- **THEN** the terminal launcher receives the explicit Herdr command with those selectors preserved, rather than resolving a named default-view alias.

### Requirement: Aggregate icon state
The bar SHALL use the native omarchy.agents glyph in the normal monochrome bar foreground, without a number. A static mark against the glyph's upper-right edge with slight overlap SHALL show aggregate state in the theme status colour: a red exclamation for blocked, a six-unit solid amber dot for working, a green tick for an unacknowledged done episode, and no mark for idle. Opening that exact thread successfully SHALL acknowledge its current done episode locally; a later working-to-done episode SHALL be eligible again. The Agents glyph itself SHALL stay monochrome. Priority SHALL be blocked, working, done, idle. The bar SHALL NOT pulse or use an underline. Unknown or unavailable sources SHALL NOT imply completion. Tooltips SHALL retain aggregate state and incomplete-source reporting.

#### Scenario: Mixed thread states
- **WHEN** blocked and done threads coexist
- **THEN** the monochrome glyph carries an exclamation, changing to a working dot if working threads remain, otherwise a tick while an unacknowledged done episode exists.

### Requirement: Truthful current state
A failed or expired host sample SHALL be unavailable rather than contributing zero threads. Allowances SHALL show only mapped accounts and valid source-dated values of each account's designated pacing window. Remaining allowance SHALL be 100 minus the pacing window's used percentage, and expected remaining allowance SHALL be the time until that window resets as a percentage of its source-supplied duration; the popover SHALL NOT assume a window duration or a provider. Remaining allowance SHALL use a solid theme-accent bar of six logical pixels and a normal foreground percentage. A positive surplus SHALL use a separate two-logical-pixel strip below the balance spanning expected to actual remaining allowance. A deficit SHALL use a static diagonal hatch in the main bar accent spanning actual to expected remaining allowance at the main track height, visually distinct from solid remaining balance. A neutral tick SHALL mark expected remaining allowance. A smaller caption-sized signed percentage SHALL show remaining minus expected allowance to one decimal place. The concise single-line tooltip SHALL compare left and expected values using %, without repeating the signed value or spelling out percentage points. Its sign and threshold colour SHALL use the same rounded value, suppressing negative zero. Pace SHALL use the percentage-point difference on the allowance scale: green at or above expected, amber for deficits up to and including five points, stronger amber above five and below ten points, and red at ten points or more. Missing pacing data SHALL NOT produce a coloured strip. Valid session counters SHALL remain visible while their live thread is reported, with older values softened and their original age shown, rather than blanked after two minutes. Missing and disconnected data SHALL stay explicit. All mapped accounts SHALL show days/hours until their pacing window resets, in the same format for windows of any length, and valid reset-pass counts. The concise hover SHALL compare remaining and expected balance directly; visible pace SHALL avoid an unbounded relative ratio near reset.

#### Scenario: One host stops reporting
- **WHEN** a previously reporting host becomes unavailable
- **THEN** its threads disappear, its active-thread count becomes unavailable and reporting remains labelled as partial.

#### Scenario: Pacing thresholds and independent balance
- **WHEN** expected allowance is 70% and remaining allowance is 70%, 66%, 65%, 64% or 60%
- **THEN** pace colours are green, amber, amber, stronger amber and red respectively, while the theme-accent balance fill and foreground percentage remain independent of pace.

#### Scenario: Deficit hatch and signed pace
- **WHEN** remaining allowance is 66% against 70% expected
- **THEN** the main fill ends at 66%, a same-accent hatch spans 66% to 70%, and an amber −4.0% reading appears beside the remaining percentage without particles or glow.

#### Scenario: Long allowance window
- **WHEN** an available account's pacing window has a 30-day duration, 25% used and 15 days until reset
- **THEN** the balance shows 75%, expected remaining allowance is 50%, the signed pace reads +25.0% and the reset caption reads `15d 0h`, without any provider-specific presentation.

#### Scenario: Codex presentation preserved
- **WHEN** the existing Codex popover fixtures are rendered from provider-neutral rows carrying the same values
- **THEN** the rendered popover is byte-identical to the reference screenshots recorded before this change.

### Requirement: Compact desktop fit
The popover SHALL use the installed Omarchy panel lifecycle and keyboard dismissal, square corners, a single column, the standard Omarchy popup padding and native-provider-style section spacing. The Anton header SHALL use the native title size without state-count blocks. Thread rows SHALL prioritise a state-coloured project title, branch and visual metrics on a flat background. Branch or checkout SHALL appear subtly beside the project title. Cache percentage SHALL show one decimal place. Context and subagent completion SHALL use equally sized partial dials. Input/output SHALL use arrows. The adjacent equally weighted token column SHALL show uncached input and one-decimal cached input percentage, with a reuse-arrow symbol for cache. Numbers, gauges and metric hover tints SHALL use the thread status colour from the active theme. Idle and unknown metrics SHALL be muted monochrome. A restrained animated working ring, blocked exclamation, done tick or idle pause SHALL sit beside the project title. Metrics SHALL share four evenly spaced slots across the full row width when space permits. Concise pointer-following tooltips SHALL name the state and original sample age. Working threads MAY show an indeterminate activity animation that stops when hidden or no longer working. All threads SHALL remain reachable by scrolling, and metrics SHALL wrap at narrower widths. Hover SHALL highlight the complete row.

#### Scenario: Narrow popover
- **WHEN** available screen width narrows
- **THEN** titles and branches elide, metrics wrap and thread status icons remain visible.

### Requirement: Reversible local installation
The plugin SHALL install as a user-owned copy independent of a development worktree, with no independent helper daemon. The installer SHALL bundle native executable and QML assets without Python, web or HTTP server assets. It SHALL configure only required native harness reporting, removing obsolete callbacks only when ownership is proven. Optional peer installation SHALL use a separate marked plugin-specific directory and a removal receipt, never an unrelated Anton application path. An explicit migration SHALL preserve private mappings/preferences, remove retired forwarding configuration and convert only recognised legacy allowance-source fields. Uninstall SHALL remove only adapters still owned by this native installation, plus its private config and cache. Peer removal SHALL be an explicit separate operation limited to receipt-owned files; local removal SHALL NOT delete unrelated remote applications or services. Installation SHALL refuse to overwrite an existing plugin. Uninstallation SHALL disable the widget and remove only its known files, preserving unrelated plugins and refusing directories with unknown files.

#### Scenario: Remove the plugin
- **WHEN** the operator runs the installed uninstaller
- **THEN** the widget and owned directory are removed without depending on the source checkout or removing other plugins.

### Requirement: Private account identity display
Allowance rows SHALL display the verified account email instead of category labels. Clicking anywhere on any allowance row SHALL toggle all account emails together, using distinct randomly chosen Silicon Valley character aliases persisted locally. Names SHALL NOT be separate controls. Concealed email SHALL NOT appear in tooltip or accessibility text. Native read-only identity RPCs SHALL be matched against existing account mappings, with emails kept out of the shared API and source checkout. Uninstall SHALL remove the private identity file and preferences.

#### Scenario: Conceal account identity
- **WHEN** the operator clicks any allowance row
- **THEN** character aliases replace all emails and remain after a shell restart, until any row is clicked again.

### Requirement: Account refresh independent of agents
While enabled, the plugin SHALL periodically request bounded account refreshes using its native local account reader and explicitly configured SSH peer sources, even when no Herdr threads exist. This SHALL retain the sanitised cache/collector path and SHALL NOT start agents or install another persistent service. Missing readings SHALL remain unknown until measured data arrives.

#### Scenario: Remote host has no threads
- **WHEN** ws-255 is reachable but has no active Herdr thread
- **THEN** its account allowance is refreshed and displayed independently of agent hooks.

### Requirement: Machines within threads
Threads SHALL be grouped beneath compact machine headers, each showing the machine name and reporting status. Configured machines SHALL remain visible when connecting, empty or unreachable. A machine awaiting its first current sample SHALL show “Connecting”. Connected empty machines SHALL show “No threads”; failed or expired machines SHALL show “Unreachable” and SHALL NOT retain stale threads. Legacy online state SHALL remain compatible while the native connection phase distinguishes startup from failure. The separate Fleet section SHALL be removed. Keyboard navigation SHALL follow the visual group order and retain each thread's exact navigation target.

#### Scenario: Reporting machine has no active threads
- **WHEN** a fresh reporting machine has no threads
- **THEN** its header remains visible with “No threads”, distinct from an unavailable machine.

#### Scenario: Navigate across machines
- **WHEN** the operator moves past the final thread in one machine group
- **THEN** keyboard focus moves to the first thread in the next nonempty group and activation opens that exact thread.

#### Scenario: Startup and first failure
- **WHEN** a configured host has not yet produced its first sample after collector startup
- **THEN** its machine header says “Connecting” without presenting cached thread measurements as fresh.
- **AND** a successful current sample changes it to connected, while a failed collection changes it to unreachable.

### Requirement: Restrained visual feedback
Fine inset separators SHALL distinguish adjacent threads. Working titles MAY show a visible periodic glyph sheen; idle, done and blocked titles SHALL NOT shimmer continuously. A newly observed state MAY briefly highlight its row and icon. Effects SHALL match stable host/thread identities across reordering, and SHALL NOT treat initial snapshots or source disappearance as state changes. Hovering an allowance with positive displayed surplus SHALL show particles and glow confined to its measured surplus interval. Zero, deficit and unavailable pace SHALL NOT show those effects. Allowance hover glow and sparks SHALL be clipped to the separate pacing interval below the balance bar; the main fill and percentage SHALL retain their normal colour without a pace halo. Motion SHALL stop when the popover closes and support a reduced-motion override.

#### Scenario: Allowance hover
- **WHEN** an available allowance row is hovered
- **THEN** only a positive displayed surplus sparkles within its interval, stopping on mouse exit, source expiry, popover closure or a change to zero or deficit. The deficit hatch remains static.

#### Scenario: A genuinely new thread arrives
- **WHEN** a previously unseen thread arrives while its reporting host and group are already visible
- **THEN** it enters with a restrained approximately 300 ms transition.
- **AND** initial hydration, source reconnect, popover reopening, filtering, reordering and expand/collapse SHALL NOT replay that entrance.

#### Scenario: Stable interaction and compaction
- **WHEN** state changes while the operator is interacting with the popover
- **THEN** existing rows retain their order, and a trustworthy increased compaction count may briefly highlight its context dial without inventing a count from incomplete history.

### Requirement: Honest subagent observations
The popover SHALL omit start/stop event counts from the completion tooltip. The underlying sanitised telemetry MAY retain bounded current-turn subagent start and stop event counts. It SHALL preserve original observation age, distinguish missing coverage from zero, and SHALL NOT infer unique child counts, current running counts or successful completion percentages. Newly observed events MAY briefly highlight the subagent row; baseline, stale and reset values SHALL NOT trigger it.

#### Scenario: Resumed subagent stops again
- **WHEN** a valid stop count exceeds the start count
- **THEN** the completion dial does not turn those observations into a completion ratio or show them as a roster.

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
- **THEN** collection and all owned helpers stop, without leaving a local listening service.

### Requirement: Lean owner-bound native collection
The native plugin SHALL collect current thread, usage, lifecycle, account and connection state without unnecessary dashboard host metrics or observation histories. Dashboard publication, music observation and unused host metrics SHALL NOT run. Native Codex records on local and configured SSH sources SHALL update while the parent waits, independently of the next parent hook. Remote enrichment SHALL use an explicitly installed, plugin-owned native peer executable over existing authenticated SSH, invoked on demand without a listener or service. The supported runtime SHALL require neither Python nor Docker locally or on configured peers. Duplicate snapshots MAY be suppressed only while a bounded heartbeat preserves transport health independently of measurement freshness. The collector SHALL terminate with its owner and recover supported state after restart without another daemon or listener.

#### Scenario: Parent waits while a child completes
- **WHEN** a validated parent record on a local or configured SSH source reports a child completion without another parent hook
- **THEN** the next bounded native collection publishes the updated outcome with its original source timestamp.

#### Scenario: Idle transport and restart
- **WHEN** no measured value changes or the plugin restarts
- **THEN** transport health remains observable, previous usage timestamps are not renewed, and valid session bindings recover without publishing raw session material.

### Requirement: Provider and account collections
The native projection SHALL use stable provider/account identities for configured allowance rows and collapse preferences. Existing label-only mappings and private identity data SHALL remain compatible. Account expansion SHALL NOT broaden Work project disclosure or publish account email. Unsupported providers SHALL remain unavailable until an explicit adapter exists.

The popover SHALL project every allowance row with the same rules, whatever its provider: it SHALL NOT branch on a provider name, default a missing provider or duration, or synthesise a provider label other than the row's own provider id, which is the only fallback when the row's provider label is missing, empty, longer than 40 characters or contains a control character. A row without a valid provider, account identity, or a known status SHALL be treated as unavailable or omitted, never given invented values. Balance and pace SHALL come only from a row that is available, fresh and has exactly one valid pacing window; zero or several pacing windows SHALL leave balance and pace unknown. Window kinds the popover does not recognise SHALL be accepted and rendered through the same pacing-window rules. An unavailable or auth_needed row SHALL show its source-supplied status text in its hover and accessible description, or "Allowance unavailable" when none is supplied, and SHALL NOT show a balance, pace or expected-allowance marker. The popover SHALL NOT add a row, provider group or status text for a provider or account the snapshot does not contain. Saved concealment aliases for existing Codex Personal and Work accounts SHALL keep applying.

#### Scenario: More than two mapped accounts
- **WHEN** native configuration contains three valid mapped accounts
- **THEN** all appear under their provider with independent readings, shared concealment and no two-row limit.

#### Scenario: Another provider with a monthly window
- **WHEN** the snapshot contains a synthetic non-Codex provider row with a 30-day pacing window alongside Codex rows
- **THEN** it appears as its own provider group under its source-supplied provider label and renders through the same allowance card, with no provider-specific presentation code.

#### Scenario: Account needs authentication
- **WHEN** a row has status auth_needed and status text "Sign in required"
- **THEN** its card shows no balance or pace, and its hover and accessible description read "Sign in required".

#### Scenario: Unavailable without source text
- **WHEN** a row has status unavailable and null status text
- **THEN** its hover reads "Allowance unavailable".

#### Scenario: Absent provider
- **WHEN** the snapshot contains rows for only one provider
- **THEN** the popover shows only that provider's group and no placeholder row for any other provider.

#### Scenario: Ambiguous pacing window
- **WHEN** an available row has no window marked as pacing, or more than one
- **THEN** its balance, pace and expected-allowance marker are unknown, and the list order of its windows is not used to choose one.

### Requirement: Compact turn wall time
A thread SHALL show a small stopwatch and current-turn elapsed wall-clock time beside its title, or the last known finished-turn duration when idle or done. This SHALL NOT add another metric slot or increase the normal thread-row height. Hover SHALL distinguish current, last and accumulated turn time, and SHALL state when accumulated coverage is unavailable. Valid aborted turns SHALL contribute to time spent without being labelled successful. Elapsed time SHALL include waits within the turn and exclude gaps between turns. It SHALL NOT be described as model activity or inference time. Missing timing SHALL remain explicit, and an expired native observation SHALL freeze the displayed elapsed value at the observation time rather than continue ticking.

#### Scenario: Working turn and stale source
- **WHEN** a validated current turn is active with a fresh native observation
- **THEN** the title stopwatch advances from its source start time.
- **AND** if native observation freshness expires, it stops advancing until a fresh validated observation arrives.

#### Scenario: Completed or aborted turn
- **WHEN** exact saved start and end bounds establish a completed or aborted turn duration
- **THEN** the row can show that last duration and the hover can include it once in validated accumulated turn time, without adding idle gaps.

### Requirement: Bounded thread viewport
The Anton header and status filters SHALL remain fixed while thread rows scroll within their own bounded viewport. Allowances SHALL remain reachable without traversing the entire thread list and MAY have their own overflow viewport. Short lists SHALL preserve compact natural sizing. Section and machine expansion and collapse SHALL remain instant. Keyboard movement SHALL reveal the exact selected row in the thread viewport and preserve its navigation target. Tooltips SHALL follow their pointer in the correct surface coordinates and remain clamped to the available popover bounds.

#### Scenario: Many threads on a short display
- **WHEN** thread content exceeds the available popover height
- **THEN** the thread viewport scrolls while the header, filters and allowance access remain visible, and keyboard navigation reveals the selected row within that viewport.

#### Scenario: Short list and immediate collapse
- **WHEN** the thread list fits naturally or a group is collapsed
- **THEN** the popover avoids needless empty reserved space and applies the collapsed layout without a transition.

### Requirement: Subagent failure notch
The existing subagent completion dial SHALL show a small red notch only when a coherent typed outcome partition reports one or more failed children. Interrupted children SHALL NOT imply failure. The concise hover SHALL retain distinct available outcome counts and SHALL NOT present child completion as parent-task progress. Missing or legacy total/done-only outcomes SHALL NOT invent a failure mark.

#### Scenario: Failed, interrupted and legacy child outcomes
- **WHEN** a validated partition contains failed children
- **THEN** the dial adds a small failure notch without creating another metric, while an interrupted-only or legacy total/done-only reading has no failure notch.

### Requirement: Automatic saved fleet discovery
When fleet discovery is enabled, the plugin SHALL discover the bounded set of enabled saved Herdr machine profiles automatically at a ten-second cadence without requiring a refresh control or restart. Additions, removal, disablement and target/session changes SHALL reconcile thread collection. Stable profile identity SHALL be separate from display labels. Explicit private profile bindings SHALL preserve existing host identifiers and overrides without resurrecting a removed profile as a static host after restart. Local configured hosts SHALL remain supported. Discovery SHALL NOT install, remove or mutate remote helpers or services.

#### Scenario: Add or remove a machine
- **WHEN** an enabled saved profile appears, disappears or becomes disabled
- **THEN** the next successful discovery reconciles its machine group and owned collection work, without duplicating workers or changing unrelated hosts.

#### Scenario: Rename or reroute a profile
- **WHEN** a saved profile changes label, target or session
- **THEN** label changes preserve its identity, while route changes discard old route samples and checkpoints and reject late results.
- **AND** activating a stale thread cannot focus a different target or session.

#### Scenario: Discovery failure
- **WHEN** profile discovery fails or returns malformed, ambiguous or oversized data
- **THEN** the last accepted inventory remains, discovery is explicitly unavailable, and failure does not fabricate removals or refresh source timestamps.

#### Scenario: Missing native peer
- **WHEN** an enabled machine is reachable but its plugin-owned native peer is absent
- **THEN** its visible group says “Setup needed”, distinct from connecting, unreachable and a connected empty machine, without provisioning anything automatically.

#### Scenario: Owner closes during reconciliation
- **WHEN** the plugin owner closes while discovery or a retiring host request is in flight
- **THEN** all owned work stops within its bounded cancellation path, and retired results cannot recreate removed state.

### Requirement: Safe retirement of cached Codex hooks
When native migration retires an owned Codex callback, it SHALL preserve an inert compatibility helper at the exact former command path for already-running callers. The helper SHALL consume stdin without interpreting, storing or publishing it, emit no output and exit successfully. It SHALL NOT restore hook registration, telemetry collection, Python execution, forwarding or a persistent process. The native installation SHALL record the exact helper path and payload hash in its existing ownership receipt and retain it until explicit uninstall.

#### Scenario: Existing session keeps its old command
- **WHEN** a Codex session created before migration invokes the retired command after current hook registration is removed
- **THEN** the receipt-owned helper succeeds without affecting the session, producing output or collecting data.

#### Scenario: Recover an already-migrated installation
- **WHEN** the old helper is absent and an owned backup proves its exact prior command under the current verified native installation
- **THEN** recovery creates only the inert helper and its ownership metadata, preserving current hooks, Pi ownership and running processes.

#### Scenario: Repeat migration or conflicting path
- **WHEN** repair encounters its matching receipt-owned helper, or an unknown file, changed payload, symlink or conflicting owner
- **THEN** the matching helper remains idempotent, while conflicts are preserved and rejected before unrelated integration changes.

#### Scenario: Explicit uninstall
- **WHEN** the operator uninstalls the native plugin or peer
- **THEN** only the unchanged receipt-owned compatibility helper is removed, unknown content is preserved and ownership conflicts retain enough state for a safe retry.

### Requirement: Stable keyboard focus
Keyboard focus in the popover SHALL identify a thread by its stable host and thread identity, never by its position in the projected list. Focus SHALL stay on the same thread when other threads appear, disappear or reorder. Focus SHALL clear when its thread disappears, is hidden by a status filter, or its machine or the Threads section is collapsed. Activation with focus SHALL open exactly the focused thread. Activation without focus SHALL open the first thread in visual order, if one exists.

#### Scenario: Earlier thread disappears
- **WHEN** the operator has focused the second visible thread and the first thread disappears from the next snapshot
- **THEN** focus stays on the same thread, now shown first, and activation opens that thread rather than its new neighbour.

#### Scenario: Focused thread disappears
- **WHEN** the focused thread disappears from the next snapshot
- **THEN** no thread is focused, no other row shows the keyboard highlight, and activation opens the first visible thread.

#### Scenario: Focused thread is filtered or collapsed
- **WHEN** the focused thread's status is hidden by a filter, or its machine or the Threads section is collapsed
- **THEN** focus clears, and re-enabling the filter or expanding the group does not restore the old highlight.

### Requirement: Explicit collector refresh
The plugin-owned collector SHALL read newline-delimited commands from its owner pipe. A `refresh` command SHALL start an immediate sample of every local host, recompute account allowances from the local cache, and publish a snapshot with the new sample times without waiting for the next heartbeat. Refreshes SHALL be coalesced so that repeated requests cause at most one extra sample round per two seconds. A refresh SHALL NOT start SSH peer probes, remote or local account reads, or fleet discovery outside their normal cadence. Unknown commands, lines longer than the command limit and invalid text SHALL be ignored without affecting collection or growing memory. Closing the owner pipe SHALL still stop the collector and all owned helpers. The popover's refresh key, middle-click and refresh command SHALL send `refresh` to a running collector and restart a stopped one. Opening the popover SHALL restart a stopped collector but SHALL NOT send a refresh to a running collector.

#### Scenario: Operator refreshes
- **WHEN** the operator presses `r`, middle-clicks the bar icon or invokes the refresh command while the collector is running
- **THEN** a snapshot carrying a local host sample taken after the request arrives without waiting for the heartbeat.

#### Scenario: Repeated refresh requests
- **WHEN** twenty refresh requests arrive within a fraction of a second
- **THEN** local hosts are sampled at most once more than their normal cadence within the following two seconds, and SSH peers and account sources are not contacted early.

#### Scenario: Malformed owner input
- **WHEN** the owner pipe carries an unknown command, an oversized line or invalid UTF-8
- **THEN** the collector ignores it and keeps publishing snapshots, and closing the pipe afterwards still stops it cleanly.

#### Scenario: Collector has exited
- **WHEN** the operator requests a refresh while the collector is not running
- **THEN** the plugin restarts the collector rather than writing to a closed pipe.

### Requirement: Single transport freshness authority
Each snapshot SHALL state the collector's heartbeat interval in seconds. The popover SHALL derive the time after which a silent collector is treated as disconnected from that interval plus an explicit allowance for the collector's loop wait. A missing or malformed interval SHALL use a safe fallback equal to the current timeout. Transport freshness SHALL remain separate from measurement freshness and SHALL NOT renew any source timestamp.

#### Scenario: Heartbeat stated
- **WHEN** a snapshot states a four-second heartbeat
- **THEN** the popover keeps the snapshot while snapshots keep arriving, and treats the collector as disconnected six seconds after the last receipt.

#### Scenario: Heartbeat missing or malformed
- **WHEN** a snapshot omits the heartbeat or states a non-numeric, non-positive or implausibly large value
- **THEN** the popover uses the six-second fallback.

### Requirement: Presentation-free collection
The native collector SHALL NOT read theme or other presentation files, and snapshots SHALL NOT carry theme, profile, display, per-host trend or per-host metrics fields. The popover SHALL take its colours from the shell. The local collector SHALL still accept peer samples that include a theme, and the peer probe SHALL keep a null theme placeholder so older local collectors keep accepting new peers. Existing private configuration that names a theme host or theme path SHALL still load, with those keys validated as before and otherwise ignored.

#### Scenario: Sample without theme reads
- **WHEN** the collector samples local and peer hosts for any length of time
- **THEN** it opens no Omarchy theme file, and its snapshots omit theme, profile, display, trend and metrics fields.

#### Scenario: Older peer sample
- **WHEN** an installed peer that has not been updated returns a sample including a full theme
- **THEN** the local collector accepts the sample and reports the host as connected.

#### Scenario: Existing configuration with theme keys
- **WHEN** an installed configuration contains a valid `theme_host` or host `theme_path`
- **THEN** the collector starts normally and ignores those keys, while an invalid value is still rejected as before.

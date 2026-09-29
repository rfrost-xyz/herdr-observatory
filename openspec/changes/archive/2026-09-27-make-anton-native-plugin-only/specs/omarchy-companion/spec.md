# Spec Delta

## MODIFIED Requirements

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

### Requirement: Reversible local installation
The plugin SHALL install as a user-owned copy independent of a development worktree, with no independent helper daemon. The installer SHALL bundle native executable and QML assets without Python, web or HTTP server assets. It SHALL configure only required native harness reporting, removing obsolete callbacks only when ownership is proven. Optional peer installation SHALL use a separate marked plugin-specific directory and a removal receipt, never an unrelated Anton application path. An explicit migration SHALL preserve private mappings/preferences, remove retired forwarding configuration and convert only recognised legacy allowance-source fields. Uninstall SHALL remove only adapters still owned by this native installation, plus its private config and cache. Peer removal SHALL be an explicit separate operation limited to receipt-owned files; local removal SHALL NOT delete unrelated remote applications or services. Installation SHALL refuse to overwrite an existing plugin. Uninstallation SHALL disable the widget and remove only its known files, preserving unrelated plugins and refusing directories with unknown files.

#### Scenario: Remove the plugin
- **WHEN** the operator runs the installed uninstaller
- **THEN** the widget and owned directory are removed without depending on the source checkout or removing other plugins.

### Requirement: Account refresh independent of agents
While enabled, the plugin SHALL periodically request bounded account refreshes using its native local account reader and explicitly configured SSH peer sources, even when no Herdr threads exist. This SHALL retain the sanitised cache/collector path and SHALL NOT start agents or install another persistent service. Missing readings SHALL remain unknown until measured data arrives.

#### Scenario: Remote host has no threads
- **WHEN** ws-255 is reachable but has no active Herdr thread
- **THEN** its account allowance is refreshed and displayed independently of agent hooks.

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

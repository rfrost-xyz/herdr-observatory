## MODIFIED Requirements

### Requirement: Menubar and popover only
The plugin SHALL provide a manifest-backed Omarchy bar widget with an anchored popover. It SHALL NOT install or launch a companion window. Display data SHALL come from the bundled native collector over a bounded private process pipe, preserving the existing Observatory disclosure rules. No local HTTP listener, Docker service or independent autostart daemon SHALL be required. The collector SHALL stop when the plugin owner pipe closes. Configured remote publication MAY continue only while the plugin is enabled. The QML presentation SHALL NOT read raw terminal output, credentials or agent session files, send input to agents or change their lifecycle. The plugin-owned collector MAY read only validated session-bound Codex records on configured sources for allowlisted numeric and lifecycle enrichment, with bounded execution and no transcript publication. An explicit thread activation MAY focus the exact live agent through Herdr and raise or open its terminal view. Remote activation SHALL resolve one enabled saved machine profile and its exact target/session, and SHALL work without requiring remote machine API forwarding. The bounded remote operation SHALL resolve the installed executable in the non-interactive environment. New terminal launches SHALL preserve the selected remote target and session without allowing a launcher alias to replace them with a default local command. Activation SHALL make one focus attempt, retain most-recently-selected matching window behaviour, and report a concise relevant failure without retrying another host or falling back to Local.

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

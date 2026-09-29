# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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

## ADDED Requirements

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

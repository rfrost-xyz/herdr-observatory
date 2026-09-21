# Office display delta

## ADDED Requirements

### Requirement: Reactive rendered terminal
The display SHALL be a single rendered terminal scene rather than dashboard cards or charts. Meaningful sampled Herdr agent and source transitions SHALL create deduplicated terminal observations and scene-wide reactions. Routine polling, unchanged samples and metadata-only changes SHALL NOT create work impacts. Initial state SHALL establish a baseline without replaying historical impacts.

#### Scenario: Agent milestone
- **WHEN** an agent appears, changes status or disappears from a fresh source
- **THEN** the terminal records the observed change and the rendered scene reacts once, without claiming unsampled tool execution.

#### Scenario: Source unavailable
- **WHEN** a source or browser connection is lost
- **THEN** the terminal records loss once, removes stale agents from the live listing and does not report them as completed.

#### Scenario: Accessible one-screen rendering
- **WHEN** the scene runs at 720p or 1080p, or reduced motion is requested
- **THEN** the terminal remains bounded to one screen with an accessible text equivalent and reduced motion suppresses dynamic effects.

## REMOVED Requirements

### Requirement: Continuous status heartbeats
**Reason**: Replaced by event-driven rendered terminal reactions.
**Migration**: Use Reactive rendered terminal.

### Requirement: Living operations console
**Reason**: Cards are superseded by a single rendered terminal.
**Migration**: Use Reactive rendered terminal.

## MODIFIED Requirements

### Requirement: Differentiated technical activity
The display SHALL show available typed Herdr revision, state sequence and protocol metadata without implying measured model throughput, while retaining one-screen geometry and reduced-motion support.

#### Scenario: Multiple working entities
- **WHEN** multiple machines or threads are working
- **THEN** their current state and available counters remain readable, with reactions reserved for observed transitions.

#### Scenario: Missing or private telemetry
- **WHEN** metadata is unavailable, malformed or belongs to an excluded Personal agent
- **THEN** unavailable fields remain marked unavailable and excluded metadata never reaches the Work feed or display.

### Requirement: Single-screen activity
The display SHALL fit 1280x720 and 1920x1080 16:9 viewports without document scrolling, with bounded process pages and a count of all permitted live panes.

#### Scenario: Many agents
- **WHEN** more agents exist than fit on one page
- **THEN** the total includes all permitted live agents and labelled pages rotate with keyboard manual navigation that holds the selected page until rotation is resumed.

# Office display

## Purpose
Provide a continuously visible, private office activity display that operates independently of the remote operator's laptop.

## Requirements

### Requirement: Single-screen activity
The display SHALL fit 1280x720 and 1920x1080 16:9 viewports without document scrolling, with bounded process pages and a count of all permitted live panes.

#### Scenario: Many agents
- **WHEN** more agents exist than fit on one page
- **THEN** the total includes all permitted live agents and labelled pages rotate with keyboard manual navigation that holds the selected page until rotation is resumed.

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
Both profiles SHALL run from versioned application images under Compose with health checks, bounded logs and restart policies, independently of development checkouts. Runtime mounts SHALL be limited to explicit configuration, Herdr integration, palette and feed paths; the application SHALL NOT mount the Docker socket or an entire home directory.

#### Scenario: Restart and update
- **WHEN** the dashboard container or Docker engine restarts
- **THEN** the configured profile resumes without an interactive development session, and a documented previous image can be used for rollback.

### Requirement: Bounded socket observation
Local container collection SHALL issue only the read-only session snapshot request to a configured Herdr socket, bound response size and time, and preserve existing disclosure filtering.

#### Scenario: Invalid socket response
- **WHEN** Herdr is absent or returns malformed, oversized or mismatched data
- **THEN** the source is unavailable while independent sources continue and later valid samples restore it.

### Requirement: Differentiated technical activity
The display SHALL show available typed Herdr revision, state sequence and protocol metadata without implying measured model throughput, while retaining one-screen geometry and reduced-motion support.

#### Scenario: Multiple working entities
- **WHEN** multiple machines or threads are working
- **THEN** their current state and available counters remain readable, with reactions reserved for observed transitions.

#### Scenario: Missing or private telemetry
- **WHEN** metadata is unavailable, malformed or belongs to an excluded Personal agent
- **THEN** unavailable fields remain marked unavailable and excluded metadata never reaches the Work feed or display.

### Requirement: Reactive rendered terminal
The display SHALL render a fixed character-cell TUI with clearly readable thread states and a separate CLI feed using the pinned ttfx text-effects library. It SHALL NOT render decorative circles or background geometry. Only current profile-filtered source observations SHALL reach the renderer. Frame generation and playback SHALL be bounded, deduplicated by source capture, and fall back to plain text on errors. Reduced motion and pause SHALL keep plain observations readable while current thread state continues updating.

#### Scenario: Agent milestone
- **WHEN** an agent appears, changes status or disappears from a fresh source
- **THEN** the TUI updates the current state and records the observed change once, without claiming unsampled tool execution.

#### Scenario: Source unavailable
- **WHEN** a source or browser connection is lost
- **THEN** the terminal records loss once, removes stale agents from the live listing and does not report them as completed.

#### Scenario: Accessible one-screen rendering
- **WHEN** the scene runs at 720p or 1080p, or reduced motion is requested
- **THEN** the terminal remains bounded to one screen with an accessible text equivalent and reduced motion suppresses dynamic effects.

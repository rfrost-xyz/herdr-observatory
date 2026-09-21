# Office display

## Purpose
Provide a continuously visible, private office activity display that operates independently of the remote operator's laptop.

## Requirements

### Requirement: Single-screen activity
The display SHALL fit 1280x720 and 1920x1080 16:9 viewports without document or internal panel scrolling, while preserving readable bounded pages and aggregate counts.

#### Scenario: Many agents
- **WHEN** more agents or machines exist than fit on one page
- **THEN** counters include all permitted live agents and labelled pages rotate with pause and manual navigation controls.

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

### Requirement: Continuous status heartbeats
Machine and working-thread heartbeat animations SHALL retain their phase through routine refreshes and traverse the full track. Thread heartbeats SHALL indicate fresh working status only, respect reduced-motion preferences and preserve the fixed viewport layout.

#### Scenario: Refresh during a sweep
- **WHEN** routine rendering occurs before a sweep finishes
- **THEN** the indicator continues from its elapsed phase rather than restarting at the left edge.

#### Scenario: Inactive or disconnected thread
- **WHEN** a thread is idle, blocked, done or its source becomes stale
- **THEN** it has no animated heartbeat.

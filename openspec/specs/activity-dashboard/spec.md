# Activity dashboard

## Purpose
Provide truthful, passive visibility of Herdr agents and machine activity across a private fleet with display-specific disclosure.

## Requirements

### Requirement: Passive fleet observation
The dashboard SHALL collect Herdr agent metadata from configured local and SSH hosts without controlling panes or persisting remote files.

#### Scenario: Working agent
- **WHEN** a valid snapshot contains a working agent
- **THEN** the dashboard shows its machine, project, harness, task title and working state.

#### Scenario: Collector failure and recovery
- **WHEN** a host times out or returns invalid data
- **THEN** that host is unavailable and its previous agents are not counted as working; other hosts continue and a later valid sample restores it.

### Requirement: Display disclosure
The server SHALL enforce its selected Work or Personal profile, classify unknown paths as Personal and omit raw terminal output, full paths and native session identifiers from HTTP responses.

#### Scenario: Work display
- **WHEN** a personal or unclassified agent is collected in Work mode
- **THEN** its metadata and history are absent from browser responses, including requests containing a personal profile parameter.

#### Scenario: Personal display
- **WHEN** Personal mode is selected at server startup
- **THEN** personal and work agents can be viewed and filtered.

### Requirement: Honest telemetry
The dashboard SHALL show sampled status transitions and available resource metrics with timestamps and collection scope; unavailable metrics SHALL remain unavailable.

#### Scenario: Missing GPU and stale browser
- **WHEN** GPU telemetry is unavailable or the browser loses its connection
- **THEN** GPU values are marked unavailable and stale browser activity stops appearing live.

### Requirement: Omarchy display
The dashboard SHALL adopt valid active Omarchy palette changes without modifying desktop configuration and provide a usable fallback, responsive layout, keyboard controls and reduced-motion support.

#### Scenario: Theme changes
- **WHEN** the selected theme source publishes a different valid palette
- **THEN** the next successful refresh updates dashboard colours; missing or malformed colours use safe defaults.

### Requirement: Local access boundary
The service SHALL bind to loopback and reject browser requests with untrusted Host or Origin headers and expose no agent mutation endpoints.

#### Scenario: Unexpected origin
- **WHEN** a request comes with an external Host or Origin
- **THEN** the service denies it without returning activity data.

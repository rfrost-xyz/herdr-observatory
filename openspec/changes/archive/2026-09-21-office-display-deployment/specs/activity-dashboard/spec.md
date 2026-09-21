# Activity dashboard delta

## MODIFIED Requirements

### Requirement: Passive fleet observation
The dashboard SHALL collect Herdr agent metadata from configured local and SSH hosts and explicitly configured Work feed files without controlling panes. Feed persistence SHALL be limited to the configured private display state path.

#### Scenario: Working agent
- **WHEN** a valid snapshot contains a working agent
- **THEN** the dashboard shows its machine, project, harness, task title and working state.

#### Scenario: Collector failure and recovery
- **WHEN** a host times out or returns invalid data
- **THEN** that host is unavailable and its previous agents are not counted as working; other hosts continue and a later valid sample restores it.


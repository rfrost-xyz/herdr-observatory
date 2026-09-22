# Office display delta

## ADDED Requirements

### Requirement: Continuous status heartbeats
Machine and working-thread heartbeat animations SHALL retain their phase through routine refreshes and traverse the full track. Thread heartbeats SHALL indicate fresh working status only, respect reduced-motion preferences and preserve the fixed viewport layout.

#### Scenario: Refresh during a sweep
- **WHEN** routine rendering occurs before a sweep finishes
- **THEN** the indicator continues from its elapsed phase rather than restarting at the left edge.

#### Scenario: Inactive or disconnected thread
- **WHEN** a thread is idle, blocked, done or its source becomes stale
- **THEN** it has no animated heartbeat.

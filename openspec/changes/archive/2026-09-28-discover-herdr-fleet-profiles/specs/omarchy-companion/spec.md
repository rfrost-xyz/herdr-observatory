## ADDED Requirements

### Requirement: Automatic saved fleet discovery
When fleet discovery is enabled, the plugin SHALL discover the bounded set of enabled saved Herdr machine profiles automatically at a ten-second cadence without requiring a refresh control or restart. Additions, removal, disablement and target/session changes SHALL reconcile thread collection. Stable profile identity SHALL be separate from display labels. Explicit private profile bindings SHALL preserve existing host identifiers and overrides without resurrecting a removed profile as a static host after restart. Local configured hosts SHALL remain supported. Discovery SHALL NOT install, remove or mutate remote helpers or services.

#### Scenario: Add or remove a machine
- **WHEN** an enabled saved profile appears, disappears or becomes disabled
- **THEN** the next successful discovery reconciles its machine group and owned collection work, without duplicating workers or changing unrelated hosts.

#### Scenario: Rename or reroute a profile
- **WHEN** a saved profile changes label, target or session
- **THEN** label changes preserve its identity, while route changes discard old route samples and checkpoints and reject late results.
- **AND** activating a stale thread cannot focus a different target or session.

#### Scenario: Discovery failure
- **WHEN** profile discovery fails or returns malformed, ambiguous or oversized data
- **THEN** the last accepted inventory remains, discovery is explicitly unavailable, and failure does not fabricate removals or refresh source timestamps.

#### Scenario: Missing native peer
- **WHEN** an enabled machine is reachable but its plugin-owned native peer is absent
- **THEN** its visible group says “Setup needed”, distinct from connecting, unreachable and a connected empty machine, without provisioning anything automatically.

#### Scenario: Owner closes during reconciliation
- **WHEN** the plugin owner closes while discovery or a retiring host request is in flight
- **THEN** all owned work stops within its bounded cancellation path, and retired results cannot recreate removed state.

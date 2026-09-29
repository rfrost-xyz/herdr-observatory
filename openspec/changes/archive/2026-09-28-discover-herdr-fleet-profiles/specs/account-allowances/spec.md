## ADDED Requirements

### Requirement: Fleet-bound account sources
When fleet discovery is enabled, native remote allowance collection SHALL follow enabled saved machine profiles independently of whether their hosts have active threads. Explicit profile-bound source overrides SHALL retain private mappings and follow the profile's current target. A removed or disabled profile SHALL cease contributing an allowance source; an otherwise unreferenced target SHALL stop its probes and invalidate late results. Another enabled profile or explicit source for the same target SHALL retain that shared account worker. Label/session-only changes and unchanged discovery results SHALL preserve account collection cadence. Account identity SHALL remain authoritative: only explicitly mapped accounts SHALL appear, and observations of the same account on multiple machines SHALL produce one row. Discovery SHALL NOT create account mappings or expose account email in shared snapshots.

#### Scenario: Newly discovered empty machine
- **WHEN** an enabled saved machine has no threads and its native peer reports an already mapped account
- **THEN** its valid allowance is eligible independently of thread activity and is deduplicated with other readings of that account.

#### Scenario: Removed source has an in-flight result
- **WHEN** a profile is disabled, removed or changes target during an allowance request, leaving its former target without any active reference
- **THEN** the retired result cannot restore its old source, while unrelated valid account readings remain available.

#### Scenario: Stable shared account route
- **WHEN** only a profile label/session changes, the inventory repeats unchanged, or another enabled profile still uses the same target
- **THEN** the account worker and its refresh cadence remain unchanged.

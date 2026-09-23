# Spec delta

## MODIFIED Requirements

### Requirement: Glanceable activity presentation
The display SHALL give thread cards persistent semantic state colour, prominent native state and project identity, compact checkout/model identity, a current activity cue and visual context/cache telemetry when validated. Exact counts, source ages and coverage SHALL remain accessible without filling the card with repeated detail text. Missing usage SHALL have one visible cue and SHALL NOT appear as zero. Unknown subagent counts SHALL NOT become zero. Recent observations SHALL use native state colour when available. The header SHALL retain Idle, Working, Blocked and Done totals. Cards SHALL avoid repeating native state as hook activity and omit meaningless bare-repository checkout labels.

#### Scenario: Partial hook coverage
- **WHEN** a fresh hook sample has activity but no usage counters
- **THEN** its card preserves activity and shows a single pending-usage cue without invented gauges.

#### Scenario: Valid telemetry
- **WHEN** context, session token and cache composition values are valid
- **THEN** a card gives their labelled visual summary with exact values and provenance accessible.

#### Scenario: Stale usage
- **WHEN** usage counters are retained after the last usage observation
- **THEN** they remain visibly dated and cannot appear live because the native state is Working.

#### Scenario: Completed thread
- **WHEN** a thread is Done, including a tool observation associated with that state
- **THEN** its card and observation use the theme's green and the Done total counts it.

#### Scenario: Checkout disclosure
- **WHEN** a permitted thread has a reported checkout directory
- **THEN** only its sanitised leaf label reaches the browser and Work feed; full paths and excluded projects remain undisclosed.

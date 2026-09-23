# Spec delta

## MODIFIED Requirements

### Requirement: Glanceable activity presentation
The display SHALL give thread cards persistent semantic state colour, prominent native state and project identity, compact checkout/model identity, and one current activity cue that includes a reported tool name at lower emphasis. Cards SHALL separate validated context, cumulative session input/output and cumulative cache-read share into non-overlapping, responsive visual instruments. Last-response counters MAY replace a missing cumulative instrument only with explicit response scope; they SHALL NOT repeat available session totals. Exact counts, source ages and coverage SHALL remain accessible. Missing usage SHALL have one visible cue and SHALL NOT appear as zero. Unknown subagent counts SHALL NOT become zero. Recent observations SHALL use native state colour when available. The header SHALL retain Idle, Working, Blocked and Done totals. Cards SHALL avoid repeating native state as hook activity and omit meaningless bare-repository checkout labels.

#### Scenario: Partial hook coverage
- **WHEN** a fresh hook sample has activity but no usage counters
- **THEN** its card preserves the activity and reported tool in one summary, and shows a single pending-usage cue without invented gauges.

#### Scenario: Valid telemetry
- **WHEN** context, session token and cache composition values are valid
- **THEN** a card shows three distinct labelled instruments with exact values and provenance accessible, without overlapping regions or a repeated last-response row.

#### Scenario: Stale usage
- **WHEN** usage counters are retained after the last usage observation
- **THEN** they remain visibly dated and cannot appear live because the native state is Working.

#### Scenario: Completed thread
- **WHEN** a thread is Done, including a tool observation associated with that state
- **THEN** its card and observation use the theme's green and the Done total counts it.

#### Scenario: Checkout disclosure
- **WHEN** a permitted thread has a reported checkout directory
- **THEN** only its sanitised leaf label reaches the browser and Work feed; full paths and excluded projects remain undisclosed.

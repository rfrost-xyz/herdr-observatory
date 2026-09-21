# Office display delta

## ADDED Requirements

### Requirement: Differentiated technical activity
Each displayed machine and working thread SHALL have a stable identity-dependent heartbeat period and phase which survives redraws. The display SHALL show available typed Herdr revision, state sequence, focus/readiness flags and protocol metadata without implying measured model throughput, while retaining one-screen geometry and reduced-motion support.

#### Scenario: Multiple working entities
- **WHEN** multiple machines or threads are working
- **THEN** their heartbeat rhythms differ and remain consistent across redraws and card reordering.

#### Scenario: Missing or private telemetry
- **WHEN** metadata is unavailable, malformed or belongs to an excluded Personal agent
- **THEN** unavailable fields remain marked unavailable and excluded metadata never reaches the Work feed or display.

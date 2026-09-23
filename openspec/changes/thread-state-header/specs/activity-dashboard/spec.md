# Spec delta

## MODIFIED Requirements

### Requirement: Glanceable activity presentation
The display SHALL give thread cards persistent semantic state colour and a two-column top bar, with project and safe branch in the larger left region and a large icon-only native state in the right region. Current activity and reported tool SHALL sit beneath the state icon while remaining distinct from the native state. The branch SHALL sit in a subbox under the project name and inherit the state colour. Compact harness, host, pane and model metadata SHALL sit at the bottom left opposite compaction and source ages. Cards SHALL show validated context as the only dial, and cumulative session input/output and cumulative cache-read share as equally weighted non-dial blocks. Last-response counters MAY replace a missing cumulative instrument only with explicit response scope; they SHALL NOT repeat available session totals. Exact counts, source ages and coverage SHALL remain accessible. Missing usage SHALL have one visible cue and SHALL NOT appear as zero. Unknown subagent counts SHALL NOT become zero. Recent observations SHALL use native state colour when available. The header SHALL retain Idle, Working, Blocked and Done totals. Cards SHALL omit meaningless bare-repository checkout labels.

#### Scenario: Status header and footer
- **WHEN** a permitted thread has project, branch, state, activity and model metadata
- **THEN** the top bar gives the project two thirds with a state-coloured branch subbox, and gives the icon-only state one third with activity and tool beneath; harness, host, pane and model sit in the bottom left opposite age detail.

#### Scenario: Valid telemetry
- **WHEN** context, cumulative session tokens and cache composition are valid
- **THEN** the card shows one context dial and equally weighted token and cached-input blocks without clipping or overlap.

#### Scenario: Partial hook coverage
- **WHEN** activity exists but usage counters are absent
- **THEN** the state and current activity/tool remain readable, and one pending-usage cue appears.

#### Scenario: Response-only counters
- **WHEN** cumulative counters are unavailable but last-response counters are valid
- **THEN** their response scope is visible and missing context remains unknown.

#### Scenario: Stale usage
- **WHEN** retained usage is older than the latest hook
- **THEN** its instrument remains visibly dated regardless of native state.

#### Scenario: Completed thread
- **WHEN** a thread is Done, including a tool observation associated with that state
- **THEN** its card and observation use the theme's green and the Done total counts it.

#### Scenario: Checkout disclosure
- **WHEN** a permitted thread has a reported checkout directory
- **THEN** only its sanitised leaf label reaches the browser and Work feed; full paths and excluded projects remain undisclosed.


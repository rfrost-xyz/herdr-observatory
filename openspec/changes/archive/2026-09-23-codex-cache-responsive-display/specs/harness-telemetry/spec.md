# Spec Delta

## ADDED Requirements

### Requirement: Recent Codex cache activity
The display SHALL derive a bounded recent cache-read share from distinct, monotonically increasing cumulative input and cached-input samples for the same Codex session. It SHALL retain sample times, show measured missing intervals and mark observed model changes and compactions without attributing a cache miss cause. Repeated hooks, session changes, counter resets and incomplete or stale values SHALL NOT create invented token activity or a misleading percentage. The session-wide balance SHALL remain independently visible.

#### Scenario: Distinct cumulative samples
- **WHEN** two valid increasing cumulative usage samples arrive for one session
- **THEN** their input and cached-input differences form one recent cache-read observation with its actual source time.

#### Scenario: Repeated or reset sample
- **WHEN** a hook repeats the same usage record, a session changes, counters fall or required counters are absent
- **THEN** no new cache observation is added and the next valid sample establishes a fresh baseline.

#### Scenario: Model or compaction marker
- **WHEN** a supported hook observes a model change or compaction between usage samples
- **THEN** the recent view marks that event as an observation without claiming it explains cache reuse.

### Requirement: Opaque Codex turn association
The local Codex adapter SHALL retain bounded opaque turn and child association for supported hook events to permit later source attribution, while preserving the parent session binding. Browser and published Work telemetry SHALL NOT contain child identifiers, transcript paths, child content or an inferred complete subagent count. A child-stop event SHALL NOT independently increment usage totals without a distinct child usage measurement.

#### Scenario: Child hook
- **WHEN** a SubagentStop hook supplies a turn ID, child ID and child transcript path
- **THEN** only validated opaque association is kept in the local adapter and the parent event remains a sampled lifecycle observation.

#### Scenario: Invalid association
- **WHEN** identifiers or paths are malformed or a child usage record is not independently available
- **THEN** attribution is unavailable and parent usage is not duplicated.

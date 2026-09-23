# Spec Delta

## ADDED Requirements

### Requirement: Glanceable subagent observations
For a permitted Codex parent thread with a valid current-turn summary, the card SHALL show observed subagent starts and stops together with wording that distinguishes observations from a live roster or completion percentage. The summary SHALL retain its own observation time, mark old information as last known, and clear on source loss or session replacement. Pi and Codex threads without a valid summary SHALL not display zero counts.

#### Scenario: Delegated work is observed
- **WHEN** a current Codex thread reports three observed starts and two observed stops
- **THEN** its card shows those counts as parent-turn observations without changing the Herdr state or claiming one active child.

#### Scenario: Old or lost observation
- **WHEN** the summary becomes older than two minutes, its source disappears or its session changes
- **THEN** age is labelled when retained, and the summary clears when the source or session is replaced.

#### Scenario: Other harness or unavailable summary
- **WHEN** a Pi thread or a Codex thread without a validated turn baseline is displayed
- **THEN** the card does not imply that no subagents were used.

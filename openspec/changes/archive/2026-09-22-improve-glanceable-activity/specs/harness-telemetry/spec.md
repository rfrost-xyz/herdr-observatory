## ADDED Requirements

### Requirement: Bounded subagent observations
Codex adapters SHALL report supported SubagentStart and SubagentStop events as latest observed parent-bound presentation activity. They SHALL discard child identifiers, agent descriptions, transcript paths and content. The display SHALL NOT infer a complete child roster, running count or permanent completion from these events. Pi SHALL retain unknown subagent coverage when no general lifecycle event is available.

#### Scenario: Child lifecycle hook
- **WHEN** a Codex subagent hook identifies the matching native parent session
- **THEN** the card and sampled observation feed can show Subagent started or Subagent stopped without changing the parent's Herdr state or disclosing child content.

#### Scenario: Session mismatch
- **WHEN** a hook's parent session differs from the pane's native session
- **THEN** the reporter drops the event without writing metadata.

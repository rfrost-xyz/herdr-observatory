# Harness telemetry

## Purpose
Provide bounded harness activity metadata shared by native Herdr and the Observatory display without collecting private transcript content.

## Requirements

### Requirement: Supplementary harness reports
Installed Codex and Pi adapters SHALL report supported tool, model, phase and compaction events as expiring Herdr presentation metadata, without changing lifecycle authority, session references or sending agent input. Reports SHALL be limited to the matching native session in the configured local Herdr instance.

#### Scenario: Matching session
- **WHEN** a supported event occurs in the identified Herdr pane
- **THEN** Herdr exposes supplementary activity metadata and a concise display label while preserving its semantic state.

#### Scenario: Unavailable or replaced session
- **WHEN** Docker or Herdr is unavailable, the adapter runs outside Herdr, or its native session no longer matches
- **THEN** reporting fails silently within a bounded time without blocking or changing the harness operation.

### Requirement: Truthful and private telemetry
The system SHALL export only allowlisted metadata, omit raw arguments, output, prompts and reasoning content, enforce Work disclosure before publishing, reject stale or mismatched telemetry, and leave unsupported metrics unavailable. Pi context estimates SHALL be distinguished from reported usage; usage SHALL name its scope.

#### Scenario: Supported and unsupported metrics
- **WHEN** Pi supplies assistant usage and a context estimate while Codex hooks supply no usage
- **THEN** Pi shows last-response usage and estimated context, and Codex usage remains unknown.

#### Scenario: Personal activity
- **WHEN** telemetry belongs to a personal or unclassified project
- **THEN** it is absent from the Work browser and Work feed, even if metadata contains unexpected fields.

### Requirement: Sampled activity display
The TUI SHALL display the latest observed telemetry alongside clear thread state and add concise notable telemetry changes to its bounded CLI feed. It SHALL identify sampling and expiry rather than claim a complete event stream.

#### Scenario: Tool or compaction changes
- **WHEN** polling observes a changed tool or compaction event
- **THEN** the CLI shows a concise event with existing local motion and the thread retains its authoritative Herdr state.

### Requirement: Minimal adapter lifecycle
Telemetry processing and install payloads SHALL ship in the existing Docker image. Installation and removal SHALL be idempotent, preserve unrelated/native integrations and require no additional persistent service or network listener.

#### Scenario: Repeat installation and removal
- **WHEN** an operator installs twice and later removes the adapters
- **THEN** there is one owned integration per harness and removal leaves unrelated hooks and extensions intact.

### Requirement: Bounded subagent observations
Codex adapters SHALL report supported SubagentStart and SubagentStop events as latest observed parent-bound presentation activity. They SHALL discard child identifiers, agent descriptions, transcript paths and content. The display SHALL NOT infer a complete child roster, running count or permanent completion from these events. Pi SHALL retain unknown subagent coverage when no general lifecycle event is available.

#### Scenario: Child lifecycle hook
- **WHEN** a Codex subagent hook identifies the matching native parent session
- **THEN** the card and sampled observation feed can show Subagent started or Subagent stopped without changing the parent's Herdr state or disclosing child content.

#### Scenario: Session mismatch
- **WHEN** a hook's parent session differs from the pane's native session
- **THEN** the reporter drops the event without writing metadata.

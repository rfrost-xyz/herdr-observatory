# Spec Delta

## ADDED Requirements

### Requirement: Parent-turn subagent event summary
The Codex adapter SHALL carry bounded counts of SubagentStart and SubagentStop hooks observed since the current parent UserPromptSubmit. It SHALL reset counts at each new parent turn, bind them to the native parent session, retain them across unrelated hooks, and leave counts unavailable when no turn baseline has been observed. Counts SHALL represent observed events only, not a complete roster, current active count, successful outcome or permanent child completion. Child identifiers and content SHALL remain private.

#### Scenario: Starts and stops during one turn
- **WHEN** a matching parent turn receives two SubagentStart hooks, one SubagentStop hook and an unrelated tool hook
- **THEN** the reported summary retains two observed starts and one observed stop after the tool hook.

#### Scenario: New turn or session
- **WHEN** a new UserPromptSubmit arrives or the native parent session changes
- **THEN** previous-turn counts do not appear on the new turn or session.

#### Scenario: Missing baseline or invalid summary
- **WHEN** no parent turn baseline was observed or carried metadata is malformed
- **THEN** the summary is unavailable rather than zero or a reconstructed roster.

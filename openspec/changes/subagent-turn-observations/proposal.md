# Proposal

## Why

The dashboard shows only the most recent Codex subagent start or stop. A later tool hook replaces that cue, so the parent thread gives no lasting indication that delegated work is under way or has returned.

## What Changes

- Show bounded counts of subagent starts and stops observed since the current parent turn began, including source age and partial-coverage wording.
- Reset the summary on a new parent turn or session and preserve it across unrelated hooks in that turn.
- Keep parent Herdr state and child outcome separate. Do not claim an exact live roster, successful completion or a progress percentage.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `harness-telemetry`: carry parent-bound observed subagent event counts through the existing metadata path.
- `activity-dashboard`: show the observed counts on the parent Codex card with honest coverage and age.

## Impact

Codex hook reporter, telemetry validation, thread renderers, tests and documentation. No new process, network endpoint or private identifier disclosure.

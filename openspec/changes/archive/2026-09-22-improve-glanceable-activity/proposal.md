## Why
Thread titles and repeated missing telemetry obscure useful project and state information. Music and summary totals need stronger hierarchy, and observations should retain the thread's semantic state colour.

## What Changes
- Give cards persistent subtle state colour, prominent native state, checkout identity and compact visual telemetry with one honest coverage message instead of empty rows.
- Promote music title and artist, with bounded in-place text effects on actual track changes.
- Replace the profile beside Rich with the configured local host and source/client role; enlarge state totals.
- Add Codex subagent start/stop observations through the existing hook adapter; document unsupported counters and Pi subagent coverage without daemons or transcript reads.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `activity-dashboard`: glanceable identity, state, telemetry coverage and music presentation.
- `harness-telemetry`: latest observed Codex subagent start/stop activity.

## Impact
Browser assets, normalised safe checkout labels, Work feed validation, tests and operational documentation. Same Docker deployment and small harness adapters; no new services or mounts.

## Why
The thread card loses useful session values after two minutes even while the identified pane remains open. Cache balance has counts but no readable share, and Idle appears with Done's colour.

## What Changes
- Retain session-bound telemetry until replaced or the pane or session changes, with an explicit last-known age after two minutes.
- Show cache-read tokens as a percentage of total input only when the input balance is complete.
- Give Idle the muted top-bar colour while keeping Done green.

## Capabilities
### Modified Capabilities
- `activity-dashboard`: thread card colour, cache balance and last-known presentation.
- `harness-telemetry`: retained session-bound metadata and source-time validation.

## Impact
Existing Herdr reporter, probe and browser card. No new counters, raw content, service or network access.

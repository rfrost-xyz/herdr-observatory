## Why
The display repeats operational labels, Working resembles Idle, and cards lack useful interaction. Track effects currently omit the artist, and harness usage should be collected when a trustworthy local structured source exists.

## What Changes
- Remove footer sampling text and music source label, retaining operational guidance in documentation.
- Match Herdr's semantic theme roles and add a read-only, keyboard-accessible card inspector with exact metrics and recent observations.
- Animate both song title and artist together on track changes.
- Extend existing harness adapters to report supported numeric usage without an additional service; keep unsupported values unknown and disclose source/age.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `activity-dashboard`: state colours, card inspector and two-line music effects.
- `harness-telemetry`: bounded hook-time structured usage enrichment.

## Impact
Browser assets, existing hook payloads/installer, telemetry validation and tests. Same Docker deployment and no additional daemon. Any local structured-file reader must stay inside the installed adapter boundary, with identity validation and numeric-only export.

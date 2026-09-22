## Why
The cards duplicate state, hide useful cumulative usage and open an unwanted inspector. Fleet metrics lack the compact visual language of the supplied btop reference.

## What Changes
- Replace click inspection with hover/focus detail and a bounded decorative click response.
- Remove host/theme labels and keep Idle, Working, Blocked and Done totals in subtle header boxes.
- Use bordered, compact metric panels and measured history for CPU, memory, disk and network.
- Prioritise cumulative harness usage, context percentage, cached/uncached tokens and supported compaction counts, with explicit scopes and unknowns.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `activity-dashboard`: metric presentation, header and card interaction.
- `harness-telemetry`: cumulative usage and harness-aligned context accounting.
- `office-display`: preserve palette synchronisation while removing the visible theme name.

## Impact
Existing browser assets and image-supplied harness adapters. No new daemon, provider request, session mount or control of Herdr. Existing Work filtering, bounded execution and theme sync remain requirements.

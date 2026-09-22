# Proposal
## Why
The normal rendering path hides the collected event feed behind static snapshot rows. Large glyphs and minimal side padding reduce useful density.
## What Changes
- Scroll timestamped observations into the live terminal and stamp real sampled state transitions once.
- Show existing typed readiness/launch flags and sequence changes without inventing tool execution.
- Use restrained Delta Corps Priest 1 artwork for meaningful DONE/INPUT events.
- Use a denser shared grid with side padding, preserving whole-scene effects and all controls.
## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `office-display`: scrolling observation feed, sparse milestone artwork and denser typography.
## Impact
Browser, bounded terminal layout, tests, README and both container deployments. No new Herdr mutation or raw-output endpoints. Event subscriptions remain a documented future enhancement; current events are explicitly sampled observations.

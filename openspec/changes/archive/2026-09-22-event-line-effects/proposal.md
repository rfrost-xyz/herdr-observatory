## Why
Travelling row highlights and recurring CLI-wide effects distract from current activity. Events should remain compact and anchored while selected new status changes receive an effect.

## What Changes
- Retain brief thread impulses and remove horizontal travelling bands.
- Show time, project, state, thread number and pertinent update on one event line.
- Remove decorative project artwork and apply effects only to eligible incoming status lines in place.
- Preserve theme colours, full playback, motion controls, privacy and source expiry.

## Capabilities
### Modified Capabilities
- `office-display`: Compact event lines, arrival-triggered line effects and impulse-only row highlights.

## Impact
Browser rendering, obsolete artwork assets/routes, tests, README and both existing Docker deployments. No collector or harness configuration changes.

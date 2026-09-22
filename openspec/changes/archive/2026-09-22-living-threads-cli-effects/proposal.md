# Proposal

## Why
The thread list needs more visible activity without losing readable state. Whole-screen text effects obscure the useful live view.

## What Changes
Use Herdr state names, animate working rows independently and briefly highlight observed state/telemetry changes. Confine all text-effect rendering to the events CLI while threads and machine status remain live.

## Capabilities
### Modified Capabilities
- `office-display`: CLI-only effects and state-driven thread motion.

## Impact
Browser rendering, capture layout, UI tests, README and both image deployments. Existing effect duration/rotation/hold and privacy rules remain.

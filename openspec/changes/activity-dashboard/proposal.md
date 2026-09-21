# Proposal

## Why
Remote Herdr activity is invisible on the physical office display. A passive dashboard should show real agent progress across machines and reflect the user's Omarchy theme.

## What Changes
- Add a local browser dashboard with agent status, task titles, sampled transition history and machine telemetry.
- Collect local and SSH-accessible Linux/WSL Herdr snapshots without taking terminal ownership or installing remote services.
- Follow the active Omarchy palette, with a Tokyo Night fallback.
- Enforce Work or Personal disclosure profiles in the server.

## Capabilities
### New Capabilities
- `activity-dashboard`: Read-only fleet activity, scoped disclosure and a themed display.
### Modified Capabilities
None.

## Impact
New Python standard-library service and browser assets. Requires Python 3.11+, Herdr and optionally SSH on collector hosts. No changes to desktop configuration, agent hooks or remote services. No cloud telemetry hosting.

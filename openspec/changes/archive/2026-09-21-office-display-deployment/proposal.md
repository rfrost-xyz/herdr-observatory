# Proposal

## Why
The initial dashboard runs only on the laptop and its layout can scroll. The office display must run independently on the workstation, show only permitted Work activity, and inherit the laptop theme over the tailnet.

## What Changes
- Fit the display to a single 16:9 viewport, with bounded pages and readable activity summaries.
- Publish only Work-filtered laptop metadata and palette through existing SSH-over-Tailscale access.
- Run a persistent workstation service that combines local agents with the expiring laptop feed.
- Keep the laptop Personal view, source configuration and private activity separate from the office display.

## Capabilities
### New Capabilities
- `office-display`: Viewport layout, permitted feed transport and deployment behaviour.
### Modified Capabilities
- `activity-dashboard`: Extend passive observation to explicitly installed, private feed files.

## Impact
Python feed publisher/receiver, collector transport, frontend layout and deployment tooling. No public HTTP exposure or SSH server enablement. No changes to Herdr panes, desktop idle policy or Windows login settings.

# Proposal

## Why
Fleet metrics need quicker visual reading and thread identity needs plain labels. The display should share the operator's music and react through an Omarchy-inspired background while keeping thread state prominent.

## What Changes
- Add truthful resource gauges, labelled identities and default eight-thread auto-paging.
- Remove pause and effect-timer controls, keeping the internal 120-second cooldown.
- Read cliamp playback and real spectrum on iapetus inside the existing image, and publish the explicitly authorised title, artist and spectrum to ws-255.
- Add a theme-aware pixel background with music and background-click reactions.

## Capabilities
### Modified Capabilities
- `office-display`: visual fleet, simplified controls, optional bounded music disclosure and background.

## Impact
Browser assets, Python collector and music endpoint, optional Compose socket mount, private deployment configuration on both hosts, tests and README. No new daemon, database or package dependency. Spotify desktop audio capture is outside this change; cliamp playback is the selected available source.

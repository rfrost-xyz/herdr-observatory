# Proposal

## Why

Anton currently reads a fixed host list once. Adding or removing an enabled machine in Herdr therefore leaves the popover out of date until its private configuration is manually changed.

## What Changes

- Discover enabled saved Herdr machine profiles automatically and reconcile additions, removal, disablement and target/session changes while the plugin runs.
- Preserve explicit local hosts and private remote overrides through durable profile bindings. Keep navigation bound to the exact profile and route that produced a thread.
- Show a concise setup-needed machine state when the native peer is absent. Discovery never provisions or removes remote files.
- Reconcile allowance sources independently of thread activity, preserving mapped account identities and rejecting late results from retired routes.
- Preserve last known discovery on source failure, with an explicit unavailable discovery state rather than fabricated removals.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `omarchy-companion`: automatic fleet reconciliation, exact profile identity, truthful setup/discovery status and owner-bound worker retirement.
- `account-allowances`: fleet-bound remote sources remain independent of threads and stop when their profile is removed or disabled.

## Impact

Rust configuration, collection scheduling, remote protocol selectors, navigation and checkpoints; small QML/JavaScript status projection changes; synthetic lifecycle tests and installation documentation. Existing account mappings and UI design remain intact. No new runtime dependency, service, web publication or Git/forge action is included.

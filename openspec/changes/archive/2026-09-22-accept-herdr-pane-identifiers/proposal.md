# Proposal

## Why
The reporter rejects valid alphanumeric Herdr pane IDs, preventing telemetry after a session restart. The existing supplementary-report requirement already requires matching native sessions to work.

## What Changes
Accept bounded opaque pane identifiers and rely on Herdr pane lookup plus native-session verification. Add a regression for alphanumeric IDs and invalid input.

## Capabilities
No requirement changes. This repairs conformance to harness-telemetry.

## Impact
Image reporter and tests. Existing installed adapters remain compatible.

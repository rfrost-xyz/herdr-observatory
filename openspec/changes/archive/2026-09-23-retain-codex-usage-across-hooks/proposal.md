## Why

The active Codex thread briefly shows token usage, then loses it when a later tool hook has no readable usage. The pane and its previous valid sample remain available, so the display should keep that sample with its original age.

## What Changes

- Retain the last valid Codex usage sample across a hook without usage for the same bound session.
- Replace it only with a newer valid sample; clear it when the native session changes.
- Verify the behaviour and deploy one reviewed image to both displays.

## Capabilities

### Modified Capabilities

- `harness-telemetry`: preserve a valid session-bound usage observation across intermittent hook reads without renewing its source time.

## Impact

The image-owned telemetry reporter, its regression tests, the harness telemetry specification and both Observatory deployments. No adapter reinstall, new service or transcript disclosure is required.

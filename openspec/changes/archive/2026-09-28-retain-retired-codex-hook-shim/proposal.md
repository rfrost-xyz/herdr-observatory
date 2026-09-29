# Proposal

## Why

Existing Codex processes can retain hook commands after their configuration changes. Removing the old Observatory shell helper during native migration therefore breaks already-running sessions with exit 127.

## What Changes

- Retain a receipt-owned inert shell helper at the exact retired hook path for cached callers, without registering new callbacks or restoring telemetry code.
- Recover already-migrated installations only with exact prior-command evidence and current native ownership.
- Validate and remove the unchanged compatibility helper during explicit uninstall, preserving conflicts and unrelated integrations.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `omarchy-companion`: migration-safe retirement of cached Codex hook commands and receipt-bound compatibility removal.

## Impact

Native hook installation/retirement, receipt handling, uninstall checks, synthetic migration tests and documentation. No UI, collector, Python, web, service or account behaviour changes. Work stays local with parent-owned installed repair and no Git/forge action.

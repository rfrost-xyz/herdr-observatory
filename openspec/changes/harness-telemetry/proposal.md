# Proposal

## Why
Herdr's lifecycle state cannot explain which tool is running or whether a harness is compacting context. Supplementary harness metadata should serve Herdr and Observatory without another service or private transcript collection.

## What Changes
- Add image-owned Codex and Pi telemetry reporters with small, idempotently installed host adapters.
- Publish bounded, expiring presentation metadata to Herdr; preserve native lifecycle and session ownership.
- Show sampled tool, model, phase, compaction and available usage information in Observatory, with existing Work filtering.
- Document installation, supported fields, sampling limitations, removal and rollback; deploy to both authorised hosts and clean delivery artifacts.

## Capabilities
### New Capabilities
- `harness-telemetry`: Supplementary metadata reporting, disclosure, presentation and adapter lifecycle.
### Modified Capabilities
None. Collectors and HTTP remain passive; the explicitly installed reporter is a separate metadata writer.

## Impact
Python reporter and probe, container payload, Codex hooks.json, a separate Pi extension, browser rendering, README and tests. Existing native integrations remain authoritative. No new daemon, listener, package installation or whole-home mount.

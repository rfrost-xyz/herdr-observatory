# Compose deployment

## Why
The Personal service is transient and tied to a worktree; the Work service borrows an unrelated image and mounts a whole home volume.

## What Changes
Package Observatory in a dedicated versioned image, deploy both profiles with Compose, scoped mounts, health checks and restart policies, and document upgrades and rollback. Use the snapshot socket directly for local collection. Send Work feeds to the receiving application container over SSH.

## Capabilities
### Modified Capabilities
- office-display: reproducible, supervised deployment on both machines.

## Impact
Packaging, local probe, SSH publisher, operational docs and tests. Preserve existing CLI collection compatibility, Work disclosure and loopback access. No changes to Herdr or its agents.

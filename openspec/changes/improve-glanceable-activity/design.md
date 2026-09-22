## Context
Existing cards reserve six textual rows even when hooks omit counters. Workspaces include checkout_path; the probe currently discards it. Music already arrives at 10 Hz and uses the same locally served effect engine.

## Goals / Non-Goals
Improve glanceability without extra daemons, mounts, raw transcript access or inferred counters. Subagent visibility is limited to verified hook fields; unsupported coverage is documented, not simulated.

## Decisions
Use a sanitised checkout basename from native workspace metadata (cwd basename fallback), never infer a Git branch from a directory name. Keep project disclosure before normalisation/Work publication. A local display descriptor derives from configured transport and publication, not the Docker hostname.

Use semantic colours for observation state as well as event icons; retain real labels for non-colour identification. Compact context/usage tiles appear only for fresh measured data, and one coverage note explains absent counters. Music uses a dedicated bounded single-line effect controller and canvas aligned to its title, sharing the pinned library and completing playback before a latest-only pending change.

## Risks / Trade-offs
Missing workspace metadata yields an explicit checkout-unknown label. Local containers need no project mounts because checkout names come from Herdr. Hook counters remain limited by harness capabilities. Eight cards must fit 720p; browser geometry and synthetic mixed-state/long-label checks cover clipping.

## Migration Plan
Build one immutable image, transfer to ws-255, recreate only Observatory on both hosts and retain the previous image/config. No new host adapters unless verified telemetry support requires a payload update. Validate privacy, served assets and health; update README/AGENTS and archive evidence.

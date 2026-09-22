# Design
## Context
The early return in draw uses server snapshot rows and bypasses records. Existing normalisation already permits typed ready/launch/focus flags, revision and state sequence.
## Goals / Non-Goals
Restore visible observation flow. Keep data server-filtered and avoid raw pane reads or a new subscription transport in this UI change.
## Decisions
Use a shared 140×44 scene. Keep 12 thread rows, reserve nine rows for sparse milestone art and ten rows for the rolling feed. Derive the effect capture from the same live composition, including current records but replacing ephemeral artwork with its text event register. Capturing an effect must not activate or consume an artwork candidate. Interpolate feed movement over 260ms; reduced motion and pause snap without motion. Samples remain explicitly labelled, status stamps use browser observation time and known before/after states. Sequence-only changes disclose counters, not inferred intermediate states. Use generated fixed DONE/INPUT art from the installed Omarchy font with attribution; no font runtime dependency. Limit large stamps to six seconds and one per thirty seconds, discard obsolete or expired candidates. Current thread state and accessible transcript keep updating during complete WASM effects.
## Risks / Trade-offs
Polling can miss intermediate transitions: label observations honestly and document events.subscribe as a future loss-reducing transport. Font density must be checked visually at both resolutions. No perpetual motion without new data.

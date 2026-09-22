## Context
Existing cards and observations map Working/Idle to accent blue and cannot be inspected. Track effects currently target only title. Codex hook payloads include transcript_path but usage is in local structured token_count records; installed 0.155.1 has last_token_usage and model_context_window. Pi already emits usage via extension callbacks.

## Goals / Non-Goals
Keep the display passive and single-screen. Do not add a daemon, mount session directories into Docker, send transcript content, infer absent counters or control Herdr panes.

## Decisions
Map Herdr state roles through the current Omarchy palette. Card clicks open a contained inspector with exact numeric labels, source freshness and recent per-thread observations, keyboard access and focus restoration. Preserve identity through updates and close on disclosure/source removal.

Extend the existing Codex hook with an image-supplied short-lived Python helper installed next to its shell forwarder. It reads only the exact session file provided by the hook beneath CODEX_HOME/sessions, verifies ownership/no symlinks and matching header session ID, and scans a bounded tail for numeric token_count metadata. Only allowlisted fields leave the helper. Reporter revalidates numbers and source timestamp; no raw file is mounted or sent into Docker. Last response tokens are not cumulative totals, and context is explicitly an estimate from last reported total/window. Pi keeps its existing event API and can seed usage from its active in-memory session branch when reloaded. Absent/provider-specific counters remain unknown.

Title and artist use independent bounded in-place effect sessions driven by the same fresh track identity, preserving existing completion/nonrepeat/visibility guards and accessible text.

## Risks / Trade-offs
Codex rollout schema is not stable: reject unknown/invalid shapes, bound reads and document tested version. Latest usage can lag a hook until the next event; report its own age instead of stamping it as new. No cache-write zero is invented when omitted. Session-file parsing stays private on the host; a tiny additional adapter file is necessary because the container deliberately has no session mount.

## Migration Plan
Build one versioned image, transfer to both engines, preserve previous image/config and run the existing installer on each harness host. It adds one helper file and preserves unrelated hooks. Verify endpoints, numeric disclosure, geometry and cleanup. Existing Pi sessions need reload; ongoing Codex hooks use the existing shell entrypoint.

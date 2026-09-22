# Verification evidence

## Acceptance mapping

- Glanceable cards/totals/state observations: web/app.js, index.html and style.css; 49 UI tests cover semantic Done colour for tool observations, all-page totals, checkout replacement, zero-versus-missing usage, parent subagent observations and display identity.
- Safe checkout disclosure: probe/core/feed and core tests cover native leaf, cwd fallback, full-path rejection and a Personal workspace containing a Work pane. Native paths remain private until classification.
- Codex child hooks: reporter and installer register start/stop, preserve existing session binding and export only allowlisted event/phase. Child IDs/types/content are discarded; no roster or running-count claim. Official hook schema audited at https://learn.chatgpt.com/docs/hooks. Pi has no general equivalent event; Codex hooks do not supply usage/context counters.
- Track transitions: new music-title controller with seven tests for baseline, duplicates/pause, full playback, latest-only rapid changes, stale recovery, hidden/reduced motion, load failure and async cancellation. Existing WASM tests play all 37 effects through completion. Static accessible track text remains available during visual effects.
- Final local gates: 66 Python and 151 JavaScript tests pass (217 total), syntax, strict OpenSpec and diff checks pass. Socket tests require normal local socket permissions; restricted sandbox failures were rerun successfully with those permissions.
- Independent backend, music and presentation reviews are clean after resolving checkout privacy, track accessibility, partial-usage coverage and stale README wording findings.
- Browser QA: synthetic three-host/eight-card fixture, including full and partial usage. 1280x720 cards all clientHeight=scrollHeight=165; 1920x1080 all 265. Document dimensions match viewport; background ends exactly at footer (582/912 respectively). No console errors. Visual inspection covered native state tints, totals, music metadata, worktree ellipsis and metric tiles. No Windows desktop claim.

## Deployment

Both hosts run herdr-observatory:23eee37, identical image sha256:7b8e92ef6d44b65b7f3ffdee1286bcd5ba29eab8dc6254fec79b0a3c377d64a6, healthy with unless-stopped. The final image also passes all 66 Python tests. HTTP app.js/style.css/index.html/music-title.mjs hashes match checkout on both hosts. Work profile contains only Work agents, checkout values contain no paths, palette equality holds and music is available on both.

Display identities verified as iapetus Host and ws-255 Client. Existing browser shows the new DOM. Existing idempotent installer updated owned SubagentStart/SubagentStop registrations on both harness hosts, preserving native/unrelated entries and the same adapter files. Fresh sessions/reload are required; no live child hook execution was forced into ongoing user sessions. Hook lifecycle is verified by tests and registration checks, not claimed as observed in a fresh live harness.

.env.previous and config/config.json.previous preserve compatible baa42fe rollback configuration. Only current23eee37 and rollback baa42fe Observatory image tags remain on each engine; obsolete9826783 removed without global pruning. Temporary synthetic server stopped, QA tab closed and browser viewport reset. No new persistent service, mount or port installed. Temporary audit/test files removed before final publication.

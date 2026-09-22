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

Pending release verification; do not treat local checks as deployment evidence.

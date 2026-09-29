# Proposal

## Why

A review of the Anton popover found correctness defects and dead work on every sample. Keyboard focus is stored as an index into `overview.threads`, so when an earlier thread disappears Enter opens its neighbour. The `r` key, middle-click and IPC refresh only re-project the last snapshot while the collector runs, because the runtime drains and ignores stdin. The runtime reads the Omarchy `colors.toml` on every local and peer sample and emits `theme`, `profile`, `display`, `trend` and `metrics` fields that no QML reads, and `State.project` computes a set of fields nothing renders. Transport freshness is split between a 4 s Rust heartbeat and an unexplained 6 s QML constant. QML tests exist but CI never runs them.

This is change 1 of 3 in the Anton popover programme. It fixes correctness and removes dead work first, and it introduces the shared measurement harness, so that the allowance contract change (2) and the popover architecture change (3) start from a smaller, measured base.

## What Changes

- Keyboard focus tracks a stable thread key (`host:thread`). Focus follows its thread through reordering and clears when the thread disappears, is filtered or its machine or section is collapsed. Enter never opens a different thread.
- The runtime reads newline-delimited commands on its owner stdin. `refresh` starts an immediate local host sample round and allowance snapshot recompute, and forces an emission. It is coalesced to at most one round every 2 s and does not wake SSH peers, remote allowance workers, Codex account RPC or fleet discovery. Unknown, oversized and non-UTF-8 lines are ignored. EOF still stops the runtime.
- QML `r`, middle-click and IPC refresh write `refresh` to the running collector and restart a dead one. Opening the popover and the retry timer still restart a dead collector, but never send `refresh` to a running one.
- The snapshot carries `heartbeat_seconds`. The QML receipt timeout is derived from it, with an explicit margin for the runtime loop wait and a safe fallback. Freshness constants are documented in one place.
- **BREAKING (snapshot wire, local only)**: the runtime no longer collects the Omarchy theme and no longer emits the `theme`, `profile`, `display` snapshot fields or the per-host `trend` and `metrics` fields. QML already themes itself. The local runtime still accepts peer samples that include `theme`, and the peer probe keeps a `"theme": null` placeholder for older local runtimes. Installed private configs with `theme_host`/`theme_path` keep loading: both keys are still validated, then ignored.
- `State.project` stops computing `gpu`, `inference`, `activity`, `paceStrength`, `pace`, `discoveryState` and the per-host `activeThreads`, `cpu`, `memory`, `vram` fields and their helpers. Allowance wire rows are unchanged.
- CI runs `tests/run-qml.sh` on GitHub Actions.
- `tests/measure_anton_popover.mjs` measures runtime cost, projection cost, view churn and static code size, and all three changes reuse it unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `omarchy-companion`: adds requirements for stable keyboard focus, explicit collector refresh, a single transport freshness authority and presentation-free collection. Existing requirements are unchanged.

## Impact

- Runtime: `omarchy/anton-runtime/src/main.rs` (snapshot shape, stdin commands, refresh scheduling, heartbeat constant), `collection.rs` (theme collection removed, probe placeholder), `config.rs` (theme keys documented as ignored), `tests/native_process.rs`.
- Plugin: `Panel.qml`, `SnapshotStore.qml`, `ThreadCard.qml`, `State.js`, plugin README.
- Tests and CI: `tests/test_omarchy_state.cjs`, `tests/qml/anton/*` (a stub `Quickshell.Io` module for SnapshotStore tests), `.github/workflows/checks.yml`, `tests/measure_anton_popover.mjs`.
- Compatibility: installed SSH peers are not updated and keep working. Installed configs keep loading. The local runtime and QML ship together, so removing the snapshot fields has no external consumer.
- Out of scope: the allowance contract (change 2), splitting `Panel.qml`, tooltip, layout and Repeater refactors, moving age labels out of the projection, and `Color.currentThemePath` (change 3).

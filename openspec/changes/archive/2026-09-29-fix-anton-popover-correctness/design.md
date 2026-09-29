# Design

## Context

See proposal.md for motivation. Relevant current state at `746ca31`:

- `stream()` in `omarchy/anton-runtime/src/main.rs` runs one coordinator loop. It blocks on `rx.recv_timeout(1 s)`, recomputes allowances from the local cache every 2 s and emits when `State.revision` changes or 4 s have passed. A stdin thread reads and discards bytes until EOF, then stops the runtime.
- Each host has a worker thread that samples, sends `Event::Host` and waits on its own `Cancellation` condvar. Local hosts use a fixed 2 s cadence and SSH hosts use the configured `interval`. Changes that only move `sampled_at` do not bump the revision, so they reach QML only on the heartbeat.
- `collection::local` reads `colors.toml` and `theme.name` on every sample, locally and inside every peer probe. `State.snapshot()` emits `theme`, `profile`, `display` and per-host `trend` and `metrics`, and nothing reads them.
- `SnapshotStore.qml` re-projects every second while the popover is open and drops the snapshot 6 s after the last receipt. `refresh()` only restarts a dead collector and re-projects.
- `Panel.qml` stores `focusedThread` as an index. `FixtureUi.qml` and the inline test `ui` in `tst_components.qml` copy that logic by hand. Only `qs.Commons` is stubbed for tests, so `Panel.qml` and `SnapshotStore.qml` cannot be instantiated by `qmltestrunner`.

## Goals / Non-Goals

**Goals:**

- Put the fixes in code the tests actually execute: pure `State.js` helpers for focus and freshness, and a real `SnapshotStore` under a stub `Quickshell.Io`.
- Split the work into two lanes with disjoint files that can proceed in parallel against the contract below.
- Keep the visuals pixel-identical. The deterministic QML screenshots at `746ca31` are the reference.

**Non-Goals:**

- Moving age labels out of the projection, key-based Repeaters, splitting `Panel.qml`, tooltip consolidation and `Color.currentThemePath` belong to change 3. As a result, view replacements while the popover is open stay at about one per second in this change.
- Any change to allowance wire rows belongs to change 2. Only JS-only projected fields are removed here.
- Waking SSH peers, the Codex account worker or fleet discovery on refresh.

## Decisions

### D1. Focus is a key; the logic lives in State.js

`Panel.qml` keeps `property string focusedKey: ""`. Key order and movement come from pure helpers in `State.js`, which `Panel.qml`, `FixtureUi.qml` and the inline test `ui` all call. Tests therefore exercise the shipped logic rather than a copy. `ThreadCard` compares its own `State.threadKey(entry)` with `ui.focusedKey`, and both tap and keyboard activation pass a key to `openThread`. Focus is reconciled whenever the key list changes, so it clears on disappearance, filtering and collapse. It is not restored when the thread reappears.

Alternative considered: keep an index and remap it on each overview change. Rejected, because every consumer still has to reason about positions, and the index-keyed Repeaters that change 3 replaces would keep it fragile.

### D2. Owner pipe carries line commands; refresh is local and coalesced

The stdin thread frames lines itself, into a fixed buffer of 64 bytes plus the newline. Once a line exceeds that, it discards bytes up to the next newline, so the buffer never grows. It trims a trailing `\r`, ignores lines that are not valid UTF-8 or not a known command, and sends `Event::Refresh` for `refresh`. EOF or a hard read error stops the runtime, as it does today.

The coordinator honours at most one refresh per `REFRESH_SPACING` (2 s). A request inside that window sets a single pending flag, which is honoured when the window ends. An honoured refresh does three things:

1. It nudges every worker whose host has local transport. A nudge wakes the worker's wait. If the worker is mid-sample, the nudge stays pending and the worker samples once more as soon as the current sample finishes. The in-flight sample cannot count, because `collection::local` stamps `sampled_at` when a sample starts, which is before the request. This is still at most one extra sample per spacing window.
2. It recomputes allowances from the local cache immediately, which reads a file and makes no Codex RPC.
3. It sets `force_emit`. The coordinator emits after it accepts the next sample from each nudged local host, or on the next loop turn if there are no local hosts.

SSH workers, remote allowance workers, the local account refresh worker and discovery are not nudged.

A nudge is a per-worker `AtomicBool` plus a condvar notify on the worker's existing `Cancellation`. The `wait()` predicate treats a pending nudge like a stop and clears it on wake. A nudge set during a sample is still set when the worker next waits, so that wait returns at once. A shared global flag was rejected because a slow worker could consume another worker's nudge.

QML splits the two meanings of the old `refresh()`:

| Function | Collector running | Collector stopped | Used by |
| --- | --- | --- | --- |
| `SnapshotStore.refresh()` | `collector.write("refresh\n")`, then `update()` | `collector.running = true`, then `update()` | `r`, middle-click, IPC `refresh` |
| `SnapshotStore.restart()` | `update()` only | `collector.running = true`, then `update()` | `onOpenedChanged`, retry timer |

Opening the popover and the retry timer call `restart()`. This keeps today's behaviour of restarting a dead collector on open, but it never writes `refresh`, so opening never adds a host sample round to a running collector. There is no separate re-project-only entry point. Quickshell 0.3.1's `Process` exposes `write(string)`, as its `.qmltypes` confirms, and `stdinEnabled` is already true.

### D3. Heartbeat is stated in the snapshot

Rust gains three documented constants: `HEARTBEAT = 4 s`, `LOOP_WAIT = 1 s` and `REFRESH_SPACING = 2 s`. The snapshot includes `"heartbeat_seconds": 4`, taken from `HEARTBEAT`. `State.receiptTimeoutMs(raw)` returns `(heartbeat_seconds + 1 + 1) * 1000`. The first extra second is the loop wait, since the coordinator checks the heartbeat once per `recv_timeout(LOOP_WAIT)`. The second is scheduling and pipe margin. The function applies this when `heartbeat_seconds` is a finite number in `[1, 60]`, and otherwise returns `6000`. For today's heartbeat that is still 6000 ms, so behaviour is unchanged, but the value now has a single source.

The other freshness constants are measurement freshness, not transport. They stay where they are and get a comment each:

- In `State.js`: host age limit `interval + 20 s`, allowance age 600 s, usage and child staleness 120 s.
- In Rust: turn timing freshness `max(3 × interval, 12 s)`.

### D4. Theme and dead snapshot fields leave the runtime

The following are deleted:

- `collection::theme`, `theme_text` and `fallback_theme`. Theme parsing is the only use of the `toml` crate (grep at `746ca31`), so it is removed from `Cargo.toml` and `Cargo.lock`; the build stays locked and offline.
- `State.theme`, `HostState.trend` and `HostState.metrics`, the theme revision bump, and the `theme_host` sample selection.

`Sample` drops `theme`. Serde ignores unknown fields because `Sample` has no `deny_unknown_fields`, so old peers that still send a theme are accepted. `collection::local` emits `"theme": null`, and that output is also the peer probe result. Older local runtimes deserialise `Sample.theme: Value`, and a missing `Value` field fails in serde, so the null placeholder keeps new peers compatible with them. The placeholder is documented as removable once no pre-change local runtime can exist.

`config::validate` keeps validating `theme_host` (it must name a configured host) and `theme_path` (a non-empty string), unchanged, and documents both as accepted and ignored. `fleet::effective` keeps stripping `theme_host`.

### D5. Dead projection fields

The following are removed from `State.project`, along with the helpers `activityView` and `usedPercent` and the `gpu` local, after a grep confirms nothing else uses them:

- `gpu` (with its hard-coded host id), `inference`, `discoveryState`
- per host: `activeThreads`, `cpu`, `memory`, `gpu`, `vram`
- per allowance: `activity`, `paceStrength`, `pace`

`discoveryLabel`, `paceDifference`, `timeRemaining`, `remaining`, `reset`, `resetCount` and every thread field stay, because QML reads them. Tests that asserted only removed fields are deleted. Tests that asserted a shared behaviour through a removed field switch to a retained field that carries the same meaning; for example, `paceDifference` sign replaces `pace`.

### D6. QML tests run in CI

A new `qml` job on `ubuntu-latest` installs Qt 6.8.3 (LTS) with `jurplel/install-qt-action`, plus the few system libraries its `qmltestrunner` loads (`libgl1`, `libegl1`, `libxkbcommon0`, `libfontconfig1`, `libdbus-1-3`, `libgssapi-krb5-2`) and `fonts-dejavu-core`. It runs `tests/run-qml.sh` with `QMLTESTRUNNER` pointing at that Qt.

Ubuntu's apt Qt 6.4.2 was tried first and rejected. On it the suite passed while every `AntonSurface` colour binding threw (`Cannot read property ... of undefined`) and `AllowanceCard` logged `Unable to assign [undefined] to QColor`: the stub `qs.Commons.Color` read as undefined there, while Qt 6.8.3 and 6.11 resolve it with no warnings. The mechanism on 6.4 was not investigated further, because production Omarchy runs a current Qt.

`tests/run-qml.sh` fails when the runner fails and also when its output contains `TypeError`, `ReferenceError` or `Unable to assign`, which qmltestrunner reports only as warnings. The runner is `$QMLTESTRUNNER` if set, else `/usr/lib/qt6/bin/qmltestrunner`, else `qmltestrunner6` or `qmltestrunner` on `PATH`. Screenshot hashes are local evidence only, because CI fonts differ.

### D7. Measurement

`tests/measure_anton_popover.mjs`, committed first and frozen after the review remediation of change 1, is the programme harness. It takes `--binary`, `--source-root` and `--state-js`, so it can measure `746ca31` from a `git archive` extract. It reports missing fields and exports as null. It uses the optional `State.js` exports `receiptTimeoutMs(raw)`, `viewSignature(view)` and `storeStep(store, event, nowMs)` when present. Change 1 implements only `receiptTimeoutMs`. The other two are reserved for change 3.

`colors.toml` opens are counted with `inotifywait` (inotify-tools) on the fixture theme directory during the timed window, because it adds no load to the measured processes. Without it, and with `strace` available, a separate untimed window of the same length runs under `strace -f -ff -z` and counts successful opens, so ptrace overhead never reaches the CPU and RSS figures. With neither tool the count is `unmeasured`. `--colors-method auto|inotify|strace|none` selects the method explicitly.

The refresh probe sends `refresh` straight after an emission and accepts only a snapshot whose local `sampled_at` is at or after the request's wall-clock time (a full-precision stamp taken before the Herdr call). It reports whether that held as `refresh_answered_after_request`.

## Interface contract between lanes

Lane A (runtime) and lane B (QML, JS, tests and CI) implement against this contract and never edit each other's files.

**Snapshot, one JSON object per line on stdout**

- Top-level keys are exactly `at`, `interval`, `heartbeat_seconds`, `hosts`, `allowances` and `fleet_discovery`. `heartbeat_seconds` is a number, currently 4.
- `theme`, `profile` and `display` are absent.
- Each host has `id`, `label`, `navigation` (optional), `online`, `connection_state`, `error`, `sampled_at`, `agents`, and optionally `protocol` and `version`. `trend` and `metrics` are absent.
- Agent and allowance rows are unchanged.

**Owner stdin, runtime input**

- UTF-8 lines ending in `\n`. A trailing `\r` is trimmed.
- Known commands: `refresh`.
- Lines over 64 bytes, lines that are not valid UTF-8, and unknown commands are ignored.
- EOF stops the runtime.
- `refresh` produces a snapshot whose local hosts have `sampled_at` no earlier than the time of the request, typically within about 1 s in fixtures and at worst after the in-flight sample plus one more sample.
- A refresh that arrives during a local sample causes one more sample right after it, stamped after the request.
- Refreshes coalesce to one round per 2 s. SSH, account and discovery sources are never woken.

**Peer probe output (`--probe`)**

- Unchanged envelope. `result.theme` is `null`.
- The local runtime accepts results with or without `theme`, and with a theme object from old peers.

**Configuration**

- `theme_host` and `hosts[].theme_path` are still validated exactly as at `746ca31` and are otherwise ignored.

**State.js exports**

These are added to `module.exports` and are usable from QML.

- `receiptTimeoutMs(raw) -> number`. Defined in D3. `raw` may be `null`.
- `focusKeys(view, groups) -> string[]`. Thread keys in visual order, taken from `groups[i].indices` in group order. Returns `[]` when `groups` is empty.
- `reconcileFocus(keys, key) -> string`. Returns `key` if `keys` contains it, otherwise `""`.
- `moveFocus(keys, key, delta) -> string`. Returns `""` for empty `keys`. If `key` is not in `keys`, returns `keys[0]`. Otherwise returns the key clamped at `index(key) + delta`.
- `activationKey(keys, key) -> string`. Returns `key` if present, otherwise `keys[0]`, otherwise `""`.
- `threadForKey(view, key) -> thread | null`.
- `project()` no longer returns the D5 fields. Every other field is unchanged.

**QML `ui` surface used by components and fixtures**

- `focusedKey: string` replaces `focusedThread: int`.
- `threadKeys: var` replaces `threadOrder: var`.
- `openThread(key: string)` replaces `openThread(index: int)`.
- `ThreadCard.keyed` is `ui.focusedKey !== "" && ui.focusedKey === State.threadKey(entry)`.

## Risks / Trade-offs

- A refresh could still load hosts if the spacing were too short → `REFRESH_SPACING` (2 s) is at least the local cadence. SSH and Codex are never woken. The burst test and the harness burst probe count samples.
- An older local runtime could meet a new peer → the null theme placeholder keeps it compatible. The existing full-theme peer fixture in `native_process.rs` stays as the reverse compatibility test.
- CI Qt may be older than local Qt 6 → the package list is derived from the imports, and the job is iterated on the pushed head. If a component needs a newer Qt feature, the job pins `ubuntu-24.04` and records it.
- The stub `Quickshell.Io` could drift from the real API → the stub implements only `Process` (`command`, `running`, `stdinEnabled`, `stdout`, `write`, `exited`) and `SplitParser` (`read`), matching the names in the installed `.qmltypes`.
- Removing the snapshot fields breaks anything outside the plugin that parsed them → there is no such consumer: the runtime's stdout is a private pipe to QML, and the peer wire keeps its envelope.

## Migration Plan

This is source-only. The parent installs the local plugin after review. Installed SSH peers are not updated and need no action. Rollback means reinstalling the previous local build. Configs and private state are untouched.

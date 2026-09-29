# Evidence

## Baseline measurement (`746ca31`, origin/main)

The harness is `tests/measure_anton_popover.mjs`, added in the first commit on this branch. The release binary was built from unchanged `746ca31` with `omarchy/herdr.observatory/build-native.sh` into a scratch directory. The binary's SHA-256 was `d201aa09d9b68cfe0758759323468e0d235eb114a0def3648d9d74ac0f5443cc`. `State.js` and the static metrics were read from a `git archive 746ca31` extract. The run used this command:

```sh
git archive 746ca31 | tar -x -C <scratch>/src
node tests/measure_anton_popover.mjs --binary <scratch>/anton-runtime --source-root <scratch>/src --repeat 3 --json
```

It ran on a local 16-core Linux 7.2 workstation, using only synthetic fixtures. There were two hosts: a local socket and a fake-SSH native peer, each with 32 agents. Local sampling ran every 2 s and the peer interval was 5 s. Each runtime window was 30 s, and runtime figures are medians of three windows.

| Window | CPU (s) | Peak RSS (KiB) | Snapshots | Mean bytes | colors.toml opens |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.042 | 7976 | 9 | 27445.2 | 21 |
| 2 | 0.053 | 4352 | 10 | 27979.4 | 21 |
| 3 | 0.066 | 3980 | 10 | 27973.6 | 22 |

At this scale CPU and sampled peak RSS are noisy. The RSS peak depends on whether a 50 ms sample catches a short-lived peer probe. CPU is user+sys of the runtime and its reaped descendants. Remote hosts, Qt and GPU are not measured.

| Metric | Baseline |
| --- | --- |
| Snapshots in 30 s | 10 |
| Mean snapshot bytes | 27973.6 |
| Runtime CPU seconds (user+sys, including peer probes) | 0.053 |
| Runtime family peak RSS (median) | 4352 KiB |
| `colors.toml` opens in 30 s (inotify, local and peer) | 21 |
| Local and remote Herdr samples in 30 s | 15 / 6 |
| Snapshot `heartbeat_seconds` | absent |
| Snapshot top-level keys | `allowances, at, display, fleet_discovery, hosts, interval, profile, theme` |
| Host keys | `agents, connection_state, error, id, label, metrics, navigation, online, protocol, sampled_at, trend, version` |
| Refresh latency after `refresh` on stdin | 4995 ms (ignored; fresh sample only on heartbeat) |
| Local samples in 3.2 s after 20 refreshes | 2 |
| Keeps publishing after oversized, unknown and invalid UTF-8 stdin | yes (input drained) |
| Projection update median (p95), 32 threads | 0.062 (0.158) ms |
| Projection update median (p95), 128 threads | 0.131 (0.301) ms |
| Projected fields per thread, host, allowance (keys/leaves) | 17/45, 12/13, 13/20 |
| Top-level view keys | 11 |
| View replacements in a simulated 60 s open popover | 60 |
| Snapshot drops in that simulation | 0 |
| Receipt timeout | 6000 ms (constant) |
| `Panel.qml` / `State.js` / `PopupContent.qml` lines | 467 / 380 / 364 |
| `required property var ui` occurrences | 7 |
| ToolTips / AntonSurfaces per ThreadCard | 6 / 6 |
| CI runs QML tests | no |

### Visual reference

`tests/run-qml.sh` at `746ca31` passed 25 tests. Two consecutive runs wrote byte-identical screenshots, so these hashes are the preservation reference for local runs:

```text
682cb88630577fa9387ffc7ce42c6878dc87d54283d4ddb9baf037f7c7baf36d  anton-continuity-connections-missing.png
29a0bf6efece346967de085baaeca87dac2a6592cfc3aa31a4bb52811a1470d2  anton-continuity-dark.png
360fe121e4f4445c908a1927ee54efb3abac3d7aac7e410976be64881450d681  anton-continuity-discovery-setup.png
92d31b6e851138ee3e8e0fdfe37799d84248d5515687b07dc2a1bbcd9aa890d7  anton-continuity-discovery-unavailable.png
ea3542d2bffe801ff286b861e51d657ff6b601a0a8bd41e1802c32c6c6d61022  anton-continuity-light.png
d737e670b90ad7e2d3bc63d169ecff4a3aefa08468e64edddcf60d2dce26fbbd  anton-continuity-many-short.png
40e7ade4aa1002ceff4c159212aeeba3de0b83137062f2cd1c308eace2f7b318  anton-continuity-short-with-notice.png
```

The JS suites (`test_pi_hooks`, `test_omarchy_state`, `test_native_distribution`) passed 51 tests at baseline.

## After measurement (integrated head)

The release binary was built from the integrated source (the tree at `3127855`) with `omarchy/herdr.observatory/build-native.sh` into a scratch directory. Its SHA-256 was `fbaef034f64075265b5877ff81915f4607f435e1ff26c1a80da8e50a9b113695`. The command, fixture, machine and window count were the same as the baseline, with `--source-root` pointing at the worktree:

```sh
node tests/measure_anton_popover.mjs --binary <scratch>/anton-runtime --source-root . --repeat 3 --json
```

| Window | CPU (s) | Peak RSS (KiB) | Snapshots | Mean bytes | colors.toml opens |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.045 | 4448 | 9 | 27145.0 | 0 |
| 2 | 0.072 | 4248 | 10 | 27677.0 | 0 |
| 3 | 0.072 | 4208 | 10 | 27671.1 | 0 |

| Metric | Baseline (`746ca31`) | After | Change |
| --- | --- | --- | --- |
| Snapshots in 30 s | 10 | 10 | none |
| Mean snapshot bytes | 27973.6 | 27671.1 | -1.1% |
| Runtime CPU seconds (median) | 0.053 | 0.072 | within noise (window range 0.042 to 0.066 before, 0.045 to 0.072 after) |
| Runtime family peak RSS (median) | 4352 KiB | 4248 KiB | within noise |
| `colors.toml` opens in 30 s | 21 | 0 | removed |
| Local and remote Herdr samples in 30 s | 15 / 6 | 15 / 6 | none; no extra polling |
| Snapshot `heartbeat_seconds` | absent | 4 | stated |
| Snapshot top-level keys | 8, including `theme`, `profile`, `display` | 6: `allowances, at, fleet_discovery, heartbeat_seconds, hosts, interval` | contract |
| Host keys | 12, including `metrics`, `trend` | 10: `agents, connection_state, error, id, label, navigation, online, protocol, sampled_at, version` | contract |
| Refresh latency after `refresh` on stdin | 4995 ms (ignored; heartbeat) | 1.7 ms | the answering sample is stamped after the request |
| Local samples in 3.2 s after 20 refreshes | 2 | 2 | bounded; coalesced into the spacing window |
| Keeps publishing after malformed stdin, then stops on EOF | yes | yes | none |
| Projection update median (p95), 32 threads | 0.062 (0.158) ms | 0.052 (0.142) ms | -16% |
| Projection update median (p95), 128 threads | 0.131 (0.301) ms | 0.124 (0.250) ms | -5% |
| Projected fields per thread (keys/leaves) | 17/45 | 17/45 | unchanged (change 3 scope) |
| Projected fields per host (keys/leaves) | 12/13 | 7/8 | -5 keys |
| Projected fields per allowance (keys/leaves) | 13/20 | 10/10 | -3 keys, -10 leaves |
| Top-level view keys | 11 | 8 | -3 |
| View replacements in a simulated 60 s open popover | 60 | 60 | unchanged (change 3 scope) |
| Receipt timeout | 6000 ms (constant) | 6000 ms, derived from `heartbeat_seconds` 4 | same value, single authority |
| `Panel.qml` / `State.js` / `PopupContent.qml` lines | 467 / 380 / 364 | 462 / 376 / 364 | -5 / -4 / 0 |
| `required property var ui` occurrences | 7 | 7 | unchanged (change 3 scope) |
| ToolTips / AntonSurfaces per ThreadCard | 6 / 6 | 6 / 6 | unchanged (change 3 scope) |
| CI runs QML tests | no | yes | `qml` job |

Scope and limits:

- These figures come from the controlled synthetic fixture, not a live fleet. The fixture socket answers at once, so the 1.7 ms refresh latency is the runtime's own reaction time. On a live host it adds one local Herdr RPC. In three further probe runs of each binary, the new binary answered in 1 to 3 ms and the old binary only at its next heartbeat, about 5 s later. In every new run the answering local `sampled_at` was stamped after the request.
- CPU and peak RSS are too small and too noisy at this scale to claim a change in either direction. The CPU window ranges overlap.
- The byte saving is small because the theme object and empty `trend` arrays were a small part of a 32-agent snapshot.
- Remote hosts, Qt rendering, GPU and live SSH peers are unmeasured. The JS projection is measured in Node, not the QML engine.
- The 20-refresh burst count is from the harness probe. The runtime process test bounds the same burst at 2 to 4 local reads, with no SSH or Codex calls.

### Visual preservation

After integration, `tests/run-qml.sh` passed 38 tests. The seven `anton-continuity-*.png` screenshots written by that run are byte-identical to the baseline hashes above.

### Neighbour-jump reproduction (task 4.2)

The key-API regression test, written first against the old `focusedThread` index API, failed as predicted before any component changed:

```text
FAIL!  : qmltestrunner::AntonPopup::test_08_focus_survives_earlier_thread_disappearing() Compared values are not the same
   Actual   (): [workstation:c]
   Expected (): [laptop:b]
Totals: 9 passed, 1 failed
```

Against a `746ca31` extract, the new runtime process tests fail where expected:

- `owner_refresh_after_emission_yields_fresh_local_sample_promptly` took 4.01 s against a 1.5 s bound.
- `owner_refresh_during_inflight_local_sample_samples_again_at_once` waited 2.00 s after the in-flight reply against a 0.5 s bound.
- `owner_refresh_recomputes_allowances_from_local_cache_at_once` took 2.00 s against a 0.6 s bound.
- `snapshot_has_exact_contract_keys_and_probe_theme_is_null` failed on the key set.

`tst_store.qml` failed tests 02, 04 and 05 against the old `SnapshotStore`.

## Gates on the integrated head

All gates ran from the worktree root with `M=omarchy/anton-runtime/Cargo.toml`.

| Gate | Result |
| --- | --- |
| `cargo fmt --manifest-path $M --check` | pass |
| `cargo clippy --manifest-path $M --locked --all-targets -- -D warnings` | pass |
| `cargo test --manifest-path $M --locked` | pass: lib 51, bin 8, native_navigation 6, native_process 24 |
| `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs` | pass: 50. The baseline had 51; 5 tests that asserted only removed fields were deleted and 4 added |
| `bash tests/run-qml.sh` | pass: 38. The baseline had 25 |
| `openspec validate fix-anton-popover-correctness --strict` | valid |
| `grep -rn "colors.toml\|fallback_theme" omarchy/anton-runtime/src` | no output |
| Dead-field reader grep (task 6.1) | only `tests/bench_anton_native.mjs` `p.cpu`, a process-sampling field |
| Intermediate commit `cd5a9c5` alone | fmt, Clippy and `cargo test --locked` pass |
| Intermediate commits `34c3b41` and `884095a` alone | JS 47 and 49, QML 25 and 30, all pass |

One unrelated flake was seen once: `native::tests::checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation` in `src/native.rs`, which this change does not touch, failed once and passed on five reruns.

## Traceability

Status is one of `planned`, `implemented`, `verified`.

| Requirement / scenario | Implementation | Verification | Commit | Status |
| --- | --- | --- | --- | --- |
| Stable keyboard focus | `State.js` `focusKeys`, `reconcileFocus`, `moveFocus`, `activationKey`, `threadForKey`; `Panel.qml` `focusedKey`, `threadKeys`, `openThread(key)`; `ThreadCard.qml` `keyed` and tap by key | node and QML suites | `884095a` | verified |
| · Earlier thread disappears | `onThreadKeysChanged` reconciles by key | node "keyboard focus keeps the same thread…"; `tst_popup` test_08 (failed on the index API as shown above, passes now), including activation key `laptop:b` and a real click | `884095a` | verified |
| · Focused thread disappears | `reconcileFocus` returns `""` | `tst_popup` test_09, also stays clear on reappearance | `884095a` | verified |
| · Focused thread is filtered or collapsed | `threadKeys` from `groupThreads` and section collapse | `tst_popup` test_10 (status filter, hidden status by snapshot) and test_11 (machine and section collapse, no highlight after re-expansion) | `884095a` | verified |
| Explicit collector refresh | `main.rs` `OwnerLines`, per-worker nudges, `REFRESH_SPACING`, forced emission; `SnapshotStore.refresh()` and `restart()`; `Panel.qml` wiring | tasks 3.1, 3.2, 5.2, 5.3 | `8e07b35`, `08af1e5` | verified |
| · Operator refreshes | nudge of local workers, forced emission; QML writes `refresh\n` | `owner_refresh_after_emission_yields_fresh_local_sample_promptly`; `owner_refresh_during_inflight_local_sample_samples_again_at_once`; `owner_refresh_recomputes_allowances_from_local_cache_at_once`; `tst_store` test_02; harness refresh latency 4995 ms to 1.7 ms | `8e07b35`, `08af1e5` | verified |
| · Repeated refresh requests | `REFRESH_SPACING` coalescing, one queued event, local-only nudges | `owner_refresh_burst_is_coalesced_and_never_wakes_ssh_or_codex` (2 to 4 local reads, no SSH or Codex calls); harness burst: 2 local samples | `8e07b35` | verified |
| · Malformed owner input | 64-byte bounded framing, EOF stops | `owner_lines_frame_split_reads_crlf_and_known_commands_only`; `owner_lines_discard_oversized_and_invalid_utf8_without_growing`; `malformed_owner_input_then_eof_stops_and_reaps_descendants`; harness `survives_malformed_input` | `8e07b35` | verified |
| · Collector has exited | `SnapshotStore.refresh()` starts a stopped collector without writing | `tst_store` test_03; `restart()` never writes (test_04) | `08af1e5` | verified |
| Single transport freshness authority | `HEARTBEAT`, `LOOP_WAIT` and `REFRESH_SPACING` constants and `heartbeat_seconds`; `State.receiptTimeoutMs`; commented measurement-freshness constants | tasks 3.3, 5.1, 5.2 | `8e07b35`, `08af1e5` | verified |
| · Heartbeat stated | snapshot key; derived timeout | `snapshot_has_exact_contract_keys_and_probe_theme_is_null` (`heartbeat_seconds == 4`); node timeout test (4 gives 6000, 10 gives 12000); `tst_store` test_05 (with 1 s, still present at 2.2 s, dropped by 3 s) | `8e07b35`, `08af1e5` | verified |
| · Heartbeat missing or malformed | fallback 6000 ms in `receiptTimeoutMs` | node cases: absent, null, string, 0, negative, NaN, Infinity, 61, boolean, object | `08af1e5` | verified |
| Presentation-free collection | `collection.rs` theme code and `toml` removed; `State` and `HostState` theme, trend and metrics removed | tasks 2.1 to 2.3 | `cd5a9c5` | verified |
| · Sample without theme reads | no theme read; probe emits `theme: null` | key-set process test; source grep empty; harness `colors.toml` opens 21 to 0 | `cd5a9c5` | verified |
| · Older peer sample | `Sample` ignores unknown fields | `unchanged_peer_result_with_theme_object_reports_connected`; legacy `fleet_fixture` tests; `old_peer_theme_is_accepted_and_new_snapshot_has_contract_keys` | `cd5a9c5` | verified |
| · Existing configuration with theme keys | validation unchanged, keys ignored and documented | `config::tests::retired_theme_keys_still_load_and_still_validate`; `fleet::tests::retired_theme_host_may_name_a_removed_profile_host`; key-set process test config carries both keys | `cd5a9c5` | verified |
| Dead projection fields (proposal, no spec change) | `State.js` `project()`; `activityView` and `usedPercent` removed | task 6.1 grep; node suite; `projection omits unused presentation fields` | `34c3b41` | verified |
| QML tests in CI (proposal, no spec change) | `checks.yml` `qml` job | emulated on `ubuntu:24.04` (Qt 6.4.2, 38 passed); GitHub Actions on the pushed head: see PR | `dc3d959` | implemented |
| Visual preservation (AGENTS.md) | no presentation edits | seven screenshot hashes identical to baseline | all | verified |

# Evidence

## Baseline measurement (`746ca31`, origin/main)

The harness is `tests/measure_anton_popover.mjs`, added in the first commit on this branch. Both the baseline and the after figures below were rerun with its final version, after review remediation round 1 (strict refresh check, strace fallback for `colors.toml` counting). The release binary was built from unchanged `746ca31` with `omarchy/herdr.observatory/build-native.sh` into a scratch directory. The binary's SHA-256 was `d201aa09d9b68cfe0758759323468e0d235eb114a0def3648d9d74ac0f5443cc`. `State.js` and the static metrics were read from a `git archive 746ca31` extract. The run used this command:

```sh
git archive 746ca31 | tar -x -C <scratch>/src
node tests/measure_anton_popover.mjs --binary <scratch>/anton-runtime --source-root <scratch>/src --repeat 3 --json
```

It ran on a local 16-core Linux 7.2 workstation, using only synthetic fixtures. There were two hosts: a local socket and a fake-SSH native peer, each with 32 agents. Local sampling ran every 2 s and the peer interval was 5 s. Each runtime window was 30 s, and runtime figures are medians of three windows.

| Window | CPU (s) | Peak RSS (KiB) | Snapshots | Mean bytes | colors.toml opens |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.046 | 7932 | 9 | 27446.1 | 21 |
| 2 | 0.037 | 4456 | 9 | 27448.7 | 21 |
| 3 | 0.058 | 8112 | 10 | 27980.0 | 21 |

At this scale CPU and sampled peak RSS are noisy. The RSS peak depends on whether a 50 ms sample catches a short-lived peer probe. CPU is user+sys of the runtime and its reaped descendants. Remote hosts, Qt and GPU are not measured.

| Metric | Baseline |
| --- | --- |
| Snapshots in 30 s (median) | 9 |
| Mean snapshot bytes (median) | 27448.7 |
| Runtime CPU seconds (user+sys, including peer probes, median) | 0.046 |
| Runtime family peak RSS (median) | 7932 KiB |
| `colors.toml` opens in 30 s (inotify, local and peer) | 21 |
| Local and remote Herdr samples in 30 s | 15 / 6 |
| Snapshot `heartbeat_seconds` | absent |
| Snapshot top-level keys | `allowances, at, display, fleet_discovery, hosts, interval, profile, theme` |
| Host keys | `agents, connection_state, error, id, label, metrics, navigation, online, protocol, sampled_at, trend, version` |
| Refresh latency after `refresh` on stdin | 4994 ms (ignored; answered only by the next heartbeat sample) |
| Local samples in 3.2 s after 20 refreshes | 2 |
| Keeps publishing after oversized, unknown and invalid UTF-8 stdin | yes (input drained) |
| Projection update median (p95), 32 threads | 0.061 (0.146) ms |
| Projection update median (p95), 128 threads | 0.132 (0.267) ms |
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

The release binary was built from the integrated source (the runtime source is unchanged since `499ba00`) with `omarchy/herdr.observatory/build-native.sh` into a scratch directory. Its SHA-256 was `fbaef034f64075265b5877ff81915f4607f435e1ff26c1a80da8e50a9b113695`. The command, fixture, machine and window count were the same as the baseline, with `--source-root` pointing at the worktree:

```sh
node tests/measure_anton_popover.mjs --binary <scratch>/anton-runtime --source-root . --repeat 3 --json
```

| Window | CPU (s) | Peak RSS (KiB) | Snapshots | Mean bytes | colors.toml opens |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.046 | 7352 | 9 | 27146.6 | 0 |
| 2 | 0.059 | 4284 | 10 | 27676.3 | 0 |
| 3 | 0.066 | 7476 | 10 | 27677.1 | 0 |

| Metric | Baseline (`746ca31`) | After | Change |
| --- | --- | --- | --- |
| Snapshots in 30 s (median) | 9 | 10 | within window-edge noise (9 or 10 in both) |
| Mean snapshot bytes (median) | 27448.7 | 27676.3 | within noise; per-window pairs fall about 1.1% (27446.1 to 27146.6, 27980.0 to 27677.1) |
| Runtime CPU seconds (median) | 0.046 | 0.059 | within noise (window range 0.037 to 0.058 before, 0.046 to 0.066 after) |
| Runtime family peak RSS (median) | 7932 KiB | 7352 KiB | within noise (range 4456 to 8112 before, 4284 to 7476 after) |
| `colors.toml` opens in 30 s | 21 | 0 | removed |
| Local and remote Herdr samples in 30 s | 15 / 6 | 15 / 6 | none; no extra polling |
| Snapshot `heartbeat_seconds` | absent | 4 | stated |
| Snapshot top-level keys | 8, including `theme`, `profile`, `display` | 6: `allowances, at, fleet_discovery, heartbeat_seconds, hosts, interval` | contract |
| Host keys | 12, including `metrics`, `trend` | 10: `agents, connection_state, error, id, label, navigation, online, protocol, sampled_at, version` | contract |
| Refresh latency after `refresh` on stdin | 4994 ms (ignored; heartbeat) | 2.6 ms | answering local `sampled_at` at or after the request in both (`refresh_answered_after_request`) |
| Local samples in 3.2 s after 20 refreshes | 2 | 2 | bounded; coalesced into the spacing window |
| Keeps publishing after malformed stdin, then stops on EOF | yes | yes | none |
| Projection update median (p95), 32 threads | 0.061 (0.146) ms | 0.053 (0.133) ms | -12% |
| Projection update median (p95), 128 threads | 0.132 (0.267) ms | 0.122 (0.243) ms | -7% |
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

- These figures come from the controlled synthetic fixture, not a live fleet. The fixture socket answers at once, so the 2.6 ms refresh latency is the runtime's own reaction time. On a live host it adds one local Herdr RPC. In three further probe runs of each binary (`--seconds 5`), the new binary answered in 1.0, 2.2 and 2.1 ms and the old binary only at its next heartbeat sample, 4993 to 4996 ms later. The harness accepts only a snapshot whose local `sampled_at` is at or after the request's wall-clock time, and `refresh_answered_after_request` was true in all eight probe runs.
- `colors.toml` opens were counted with inotify. The strace fallback was checked separately in an Arch Linux container with `strace` and without inotify-tools (6 s windows): 5 successful opens for the `746ca31` binary and 0 for the new one, against 6 and 0 with inotify on the host.
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
| `cargo test --manifest-path $M --locked` | pass: lib 51, bin 8, native_navigation 6, native_process 24 (lib flakes noted below) |
| `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs` | pass: 50. The baseline had 51; 5 tests that asserted only removed fields were deleted and 4 added |
| `bash tests/run-qml.sh` (Qt 6.11, with the binding-error gate) | pass: 38, 0 QWARN. The baseline had 25 |
| `tests/run-qml.sh` on Qt 6.8.3 (aqtinstall in `ubuntu:24.04`) | pass: 38, 0 QWARN |
| `tests/run-qml.sh` on apt Qt 6.4.2 (`ubuntu:24.04`, the rejected CI setup) | fails as intended: 38 tests pass but the gate counts 1247 binding errors and exits 1 |
| `openspec validate fix-anton-popover-correctness --strict` | valid |
| `grep -rn "colors.toml\|fallback_theme" omarchy/anton-runtime/src` | no output |
| Dead-field reader grep (task 6.1) | only `tests/bench_anton_native.mjs` `p.cpu`, a process-sampling field |
| Intermediate commit `be16c42` alone | fmt, Clippy and `cargo test --locked` pass |
| Intermediate commit `499ba00` alone | `native_process` 24 pass |
| Intermediate commits `3be448f`, `f6e2f82` and `b6cb60f` alone | JS 47, 49 and 49; QML 25, 30 and 37; all pass |

Two lib tests flake in code this change does not touch: `native::tests::checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation` and `allowances::tests::account_rpc_is_read_only_and_retains_quota_when_usage_unsupported`. They failed in 3 of 12 lib runs on this branch and in 4 of 15 runs on a `746ca31` extract, so they predate this change and are left for separate work.

## Traceability

Status is one of `planned`, `implemented`, `verified`.

| Requirement / scenario | Implementation | Verification | Commit | Status |
| --- | --- | --- | --- | --- |
| Stable keyboard focus | `State.js` `focusKeys`, `reconcileFocus`, `moveFocus`, `activationKey`, `threadForKey`; `Panel.qml` `focusedKey`, `threadKeys`, `openThread(key)`; `ThreadCard.qml` `keyed` and tap by key | node and QML suites | `f6e2f82` | verified |
| · Earlier thread disappears | `onThreadKeysChanged` reconciles by key | node "keyboard focus keeps the same thread…"; `tst_popup` test_08 (failed on the index API as shown above, passes now), including activation key `laptop:b` and a real click | `f6e2f82` | verified |
| · Focused thread disappears | `reconcileFocus` returns `""` | `tst_popup` test_09, also stays clear on reappearance | `f6e2f82` | verified |
| · Focused thread is filtered or collapsed | `threadKeys` from `groupThreads` and section collapse | `tst_popup` test_10 (status filter, hidden status by snapshot) and test_11 (machine and section collapse, no highlight after re-expansion) | `f6e2f82` | verified |
| Explicit collector refresh | `main.rs` `OwnerLines`, per-worker nudges, `REFRESH_SPACING`, forced emission; `SnapshotStore.refresh()` and `restart()`; `Panel.qml` wiring | tasks 3.1, 3.2, 5.2, 5.3 | `499ba00`, `b6cb60f` | verified |
| · Operator refreshes | nudge of local workers, forced emission; QML writes `refresh\n` | `owner_refresh_after_emission_yields_fresh_local_sample_promptly`; `owner_refresh_during_inflight_local_sample_samples_again_at_once`; `owner_refresh_recomputes_allowances_from_local_cache_at_once`; `tst_store` test_02; harness refresh latency 4994 ms to 2.6 ms | `499ba00`, `b6cb60f` | verified |
| · Repeated refresh requests | `REFRESH_SPACING` coalescing, one queued event, local-only nudges | `owner_refresh_burst_is_coalesced_and_never_wakes_ssh_or_codex` (2 to 4 local reads, no SSH or Codex calls); harness burst: 2 local samples | `499ba00` | verified |
| · Malformed owner input | 64-byte bounded framing, EOF stops | `owner_lines_frame_split_reads_crlf_and_known_commands_only`; `owner_lines_discard_oversized_and_invalid_utf8_without_growing`; `malformed_owner_input_then_eof_stops_and_reaps_descendants` (after the malformed input, a snapshot arrives within 6 s and a valid `refresh` is answered within 1.5 s by a local sample stamped after it, then EOF stops the runtime and reaps descendants); harness `survives_malformed_input` | `499ba00` | verified |
| · Collector has exited | `SnapshotStore.refresh()` starts a stopped collector without writing | `tst_store` test_03; `restart()` never writes (test_04) | `b6cb60f` | verified |
| Single transport freshness authority | `HEARTBEAT`, `LOOP_WAIT` and `REFRESH_SPACING` constants and `heartbeat_seconds`; `State.receiptTimeoutMs`; commented measurement-freshness constants | tasks 3.3, 5.1, 5.2 | `499ba00`, `eccb9db` | verified |
| · Heartbeat stated | snapshot key; derived timeout | `snapshot_has_exact_contract_keys_and_probe_theme_is_null` (`heartbeat_seconds == 4`); node timeout test (4 gives 6000, 10 gives 12000); `tst_store` test_05 (with 1 s, still present at 2.2 s, dropped by 3 s) | `499ba00`, `eccb9db` | verified |
| · Heartbeat missing or malformed | fallback 6000 ms in `receiptTimeoutMs` | node cases: absent, null, string, 0, negative, NaN, Infinity, 61, boolean, object | `eccb9db` | verified |
| Presentation-free collection | `collection.rs` theme code and `toml` removed; `State` and `HostState` theme, trend and metrics removed | tasks 2.1 to 2.3 | `be16c42` | verified |
| · Sample without theme reads | no theme read; probe emits `theme: null` | key-set process test; source grep empty; harness `colors.toml` opens 21 to 0 | `be16c42` | verified |
| · Older peer sample | `Sample` ignores unknown fields | `unchanged_peer_result_with_theme_object_reports_connected`; legacy `fleet_fixture` tests; `old_peer_theme_is_accepted_and_new_snapshot_has_contract_keys` | `be16c42` | verified |
| · Existing configuration with theme keys | validation unchanged, keys ignored and documented | `config::tests::retired_theme_keys_still_load_and_still_validate`; `fleet::tests::retired_theme_host_may_name_a_removed_profile_host`; key-set process test config carries both keys | `be16c42` | verified |
| Dead projection fields (proposal, no spec change) | `State.js` `project()`; `activityView` and `usedPercent` removed | task 6.1 grep; node suite; `projection omits unused presentation fields` | `61644f6` | verified |
| QML tests in CI (proposal, no spec change) | `checks.yml` `qml` job on Qt 6.8.3; `tests/run-qml.sh` binding-error gate | GitHub Actions on `23cf82d`, push run 36641275420 and pull_request run 36641278438: `qml` 38 passed on QtTest 6.8.3 with 0 QWARN and 0 binding errors, `native` passed; the gate fails on apt Qt 6.4.2 (1247 binding errors) | `b7135da` | verified |
| Visual preservation (AGENTS.md) | no presentation edits | seven screenshot hashes identical to baseline | all | verified |

## Continuous integration

On `23cf82d`, both the `push` run (36641275420) and the `pull_request` run (36641278438) passed the `native` and `qml` jobs. The `qml` job ran QtTest 6.8.3 and reported `Totals: 38 passed, 0 failed`; its log has no QWARN lines and no `TypeError`, `ReferenceError` or `Unable to assign`.

The first `qml` job used Ubuntu's apt Qt 6.4.2 (runs 36638574975 and 36638583558 on the earlier head `b922a32`). It passed while logging 1225 QWARNs, all colour bindings reading the stub `qs.Commons.Color` as undefined (`AntonSurface.qml` lines 61, 62 and 68, and `Unable to assign [undefined] to QColor` in `AllowanceCard.qml`). The cause on 6.4 was not investigated; design D6 records the switch to Qt 6.8.3 and the binding-error gate, which fails that setup.

The old-code reproductions above were produced by the implementation lanes (a `746ca31` extract for the Rust process tests, the old index API for `test_08`, the old store for `tst_store`) and were not rerun at integration.

## Independent review

An independent adversarial review of the frozen source against the proposal, specs, design, tasks and AGENTS.md ran in two rounds. Round 1 findings were remediated on the branch (among them the strict refresh check and the strace fallback in the harness, recorded above). Round 2 at `9e62a89` was clean, with no findings. CI (`native` and `qml`) was green on that head.

## Live installed check

The parent ran this check on 2026-09-29 against the reviewed source at `9e62a89`, on the local workstation with the installed Omarchy plugin.

- The local plugin payload was replaced from this branch's reviewed source: every `install.sh` payload file plus the release binary, each byte-compared with its source after copying. `SnapshotStore.qml` was copied last.
- The existing private configuration validated with the new binary's `--migrate-config`.
- Omarchy was warm-restarted once.
- The owner marker kept its inode.
- The private `.accounts.json`, `.config.json` and `.peers.json` files were byte-identical before and after.
- `privacy.ini` was rewritten by the popover's existing preference persistence, with an unchanged key set.
- Exactly one collector process was running after the restart, and its stdin was the owner pipe.
- Diagnostics after the restart had the same shape as before the update: 3 configured hosts connected and reporting, 3 threads, one allowance account available and one unavailable.
- IPC `refresh`, `open`, `status` and `close` all succeeded.
- The SSH peers were not updated and kept reporting through the new local collector, confirming peer backward compatibility (scenario "Older peer sample" on a live fleet).

Not live-measured: CPU and RSS of the installed process, and Qt or GPU cost. The CPU and RSS figures above come only from the synthetic fixture.

# Evidence

## Baseline (`53f2407`, origin/main)

**Harness.** `tests/measure_anton_popover.mjs` at `a7ff5c0` (the `allowance_readings` metric was added later in `618d383`; its baseline row comes from a `--skip-runtime` run on the same unchanged source). That commit adds the `architecture` metrics and the `tst_metrics.qml` rendered-tooltip count, and changes no existing metric definition. The source measured is the unchanged `53f2407` tree plus that harness commit. `git_head` is `a7ff5c0`.

**Binary.** Built from the unchanged tree with `omarchy/herdr.observatory/build-native.sh` into a scratch directory. SHA-256 `74f50d695c89e4cc01198c77df9d31ab9b1230e8e5948a6a73dceb569d76931d`, which is identical to change 2's after-measurement binary, so the build is reproducible.

**Command.**

```sh
omarchy/herdr.observatory/build-native.sh <scratch>/anton-runtime
node tests/measure_anton_popover.mjs --binary <scratch>/anton-runtime --source-root <worktree> --repeat 3 --json
```

**Conditions.**

- A local 16-core Linux 7.2 workstation.
- Synthetic fixtures only: two hosts of 32 agents each, one a local socket and one a fake-SSH native peer.
- 30 s windows; runtime figures are medians of three windows.
- `colors.toml` opens were counted with inotify.
- Qt 6.11.2 `qmltestrunner` and `qmllint`; Node 26.10.

| Window | CPU (s) | Peak RSS (KiB) | Snapshots | Mean bytes | colors.toml opens |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.059 | 7584 | 10 | 27679.0 | 0 |
| 2 | 0.057 | 7480 | 10 | 27676.5 | 0 |
| 3 | 0.052 | 4456 | 10 | 27674.8 | 0 |

| Metric | Baseline (`a7ff5c0` harness on `53f2407` source) |
| --- | --- |
| Snapshots in 30 s (median) | 10 |
| Mean snapshot bytes (median) | 27676.5 |
| Runtime CPU seconds (user+sys, including peer probes, median) | 0.057 |
| Runtime family peak RSS (median) | 7480 KiB |
| `colors.toml` opens in 30 s | 0 |
| Local / remote Herdr samples in 30 s | 15 / 6 |
| Snapshot `heartbeat_seconds` | 4 |
| Refresh latency after `refresh` on stdin | 2.2 ms (answered by a post-request sample) |
| Local samples in 3.2 s after 20 refreshes | 2 |
| Keeps publishing after malformed stdin | yes |
| Projection update median (p95), 32 threads | 0.0525 (0.1351) ms |
| Projection update median (p95), 128 threads | 0.1223 (0.2866) ms |
| Projected fields per view, thread, host, allowance (keys/leaves) | 8/1483, 17/45, 7/8, 11/11 |
| View replacements in a simulated 60 s open popover | **60** (store emulation: `SnapshotStore 746ca31 logic`) |
| Snapshot drops in that simulation | 0 |
| Receipt timeout | 6000 ms |
| `Panel.qml` / `State.js` / `PopupContent.qml` lines | 456 / 422 / 364 |
| Total QML lines / QML files | 1977 / 12 |
| `required property var ui` occurrences | 7 |
| ToolTips / AntonSurfaces per ThreadCard | 6 / 6 |
| CI runs QML tests | yes |
| Allowance wire bytes per row / keys per row | 337 / 11 |
| Presentation `weekly_` / `604800` / `provider ==` / quoted `codex` | 0 / 0 / 0 / 0 |

Added architecture metrics (`a7ff5c0`):

| Metric | Baseline |
| --- | --- |
| `ui.` member references in QML | 149 |
| Preference parse calls in QML (`parseList(`, `parseObject(`, `JSON.parse(`) | 17 |
| `ToolTip` declarations in QML | 1 (in `AntonSurface.qml`) |
| Hard-coded `.local/state/omarchy` paths in QML | 1 |
| Clock-only view changes (one snapshot, 60 steps of 1 s) | 25 |
| Qt 6 qmllint warnings, `-I tests/qml/anton` | 149: unqualified 104, unused-imports 30, import 5, unresolved-type 3, missing-property 4, signal-handler-parameters 2, inheritance-cycle 1 |
| qmllint warnings per file | AllowanceCard 18, AntonSurface 14, AntonText 3, BurnEffect 4, MetricDial 5, Panel 27, PopupContent 38, SectionHeader 4, SheenTitle 17, SnapshotStore 1, ThreadCard 11, ThreadSignal 7 |
| Rendered ToolTip instances, standard fixture (`tst_metrics.qml`) | 29 |
| `allowance_readings` (`618d383`, `--skip-runtime` on unchanged source; source `projected fields`) | Codex weekly: remaining 60, time remaining 50, pace +10, reset `3d 12h`, age `30s ago`. Synthetic monthly: 75, 50, +25, `15d 0h`, `30s ago`. `auth_needed`: all null, age `source unavailable` |

A local qmllint run with a scratch `qs` symlink to the installed Omarchy shell resolves `qs.Ui` (evidence only, not reproducible in CI). It reported 148 warnings: unqualified 93, unused-imports 30, missing-property 22, signal-handler-parameters 2, property-override 1. By file: Panel.qml 9, PopupContent 44, AllowanceCard 21, AntonSurface 17, SheenTitle 17, ThreadCard 13, ThreadSignal 7, MetricDial 6, SectionHeader 5, AntonText 4, BurnEffect 4, SnapshotStore 1.

Scope and limits:

- These are controlled synthetic fixtures, not a live fleet.
- CPU and RSS are small and noisy at this scale.
- Remote hosts, Qt rendering and GPU are not measured.
- JS projection is measured in Node, not the QML engine.

### Visual reference

`bash tests/run-qml.sh` on the baseline tree passed 44 test functions, including the new `tst_metrics`, with no binding errors. The seven screenshots are byte-identical to the reference in both archived changes:

```text
682cb88630577fa9387ffc7ce42c6878dc87d54283d4ddb9baf037f7c7baf36d  anton-continuity-connections-missing.png
29a0bf6efece346967de085baaeca87dac2a6592cfc3aa31a4bb52811a1470d2  anton-continuity-dark.png
360fe121e4f4445c908a1927ee54efb3abac3d7aac7e410976be64881450d681  anton-continuity-discovery-setup.png
92d31b6e851138ee3e8e0fdfe37799d84248d5515687b07dc2a1bbcd9aa890d7  anton-continuity-discovery-unavailable.png
ea3542d2bffe801ff286b861e51d657ff6b601a0a8bd41e1802c32c6c6d61022  anton-continuity-light.png
d737e670b90ad7e2d3bc63d169ecff4a3aefa08468e64edddcf60d2dce26fbbd  anton-continuity-many-short.png
40e7ade4aa1002ceff4c159212aeeba3de0b83137062f2cd1c308eace2f7b318  anton-continuity-short-with-notice.png
```

### Baseline gates

- The JS suites (`test_pi_hooks`, `test_omarchy_state`, `test_native_distribution`) passed 66.
- The QML suite passed 44.
- `cargo test --locked --offline`:
  - Every non-lib target passed in all four runs: bin 8, `native_navigation` 6, `native_process` 26.
  - The lib suite (65 tests) failed in 2 of 4 runs, each time on different tests, and passed in the other two. The failures were `allowances::tests::unchanged_peer_row_converts_through_remote_sanitising` together with `native::tests::checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation` in one run, and `allowances::tests::window_order_does_not_select_the_pacing_window` in the other.
  - This flakiness is pre-existing and in Rust code this change does not touch. It must not be attributed to this change.

### Baseline oracles (`947ecc6`)

Generated by `tests/capture_popover_oracles.cjs` from the `53f2407` `State.js` and `Core.Settings` block, using synthetic data only:

- `tests/fixtures/popover-time-oracle.json`: 17 snapshots and 112 projections at instants on both sides of these thresholds:
  - host `maxAge` (default, interval 60, invalid interval);
  - a future host sample;
  - 120 s telemetry staleness, and the age-label seconds/minutes and minutes/hours boundaries;
  - future telemetry;
  - turn freshness (12 s and 3 s), one-second skew and a future observation;
  - 600 s allowance freshness with one-second skew;
  - reset passing, the reset-label hour and day boundaries, pass expiry and window coverage;
  - a monthly window;
  - a mixed fleet.
- `tests/fixtures/popover-diagnostics-oracle.json`: 13 IPC `diagnostics` outputs.
- `tests/fixtures/popover-privacy-v2.ini`: written by the baseline Settings block and write calls.
- `tests/fixtures/popover-privacy-v1.ini`: pre-v2 flags only.

## Lane results

Both lanes worked in the shared worktree and committed explicit paths only. The coordinator verified each lane B stage on a `git archive HEAD` export with only that stage's files overlaid, then committed it; lane A's A3 files were verified the same way and committed after 3.7 and 3.8.

| Task | Commit | Verification at that commit |
| --- | --- | --- |
| 2.1 (A1 helpers) | `a3e7822` | node 66/66; QML 44/44, 0 binding errors |
| 2.2 (A2 source times, readings) | `aadd09b`, `ccece10` | node 72/72; `tst_store` 9/9 including `test_07_now_advances_only_while_visual_updates` |
| 3.1 (theme, tooltip, stubs) | `f5d6d75` | QML 58 passed; archived hashes 7/7 |
| 3.2 (preferences) | `3d0719c` | QML 65 passed; archived hashes 7/7; `~/.config` listing checksum unchanged by the run |
| 3.3 (controller) | `b5cb155` | QML 79 passed; archived hashes 7/7 |
| 3.4 (typed components, FixtureUi removed) | `b152fa3` | QML 79 passed, 0 binding errors; corrected hashes 7/7; JS 82/82; `grep -rn "var ui\|ui\." omarchy/herdr.observatory/*.qml` prints nothing |
| 3.5 (keyed rows) | `95845c1` | QML 87 passed; corrected hashes 7/7; `tst_popup` test_13 and test_14 fail on the 3.4 code |
| 3.6 (named height rules) | `3a513c1` | QML 87 passed; corrected hashes 7/7; `test_03`, `test_04`, `test_06` unchanged |
| 3.7 (labels from readings) | `d8f7edc` | QML 90 passed; corrected hashes 7/7; test_12, test_16 and test_18 fail on the 3.6 code |
| 3.8 (IPC diagnostics, install lists) | `dea2925` | JS 83/83 including the every-file install-list test; `bash -n` on both scripts |
| 3.9 (qmllint) | `dd5b9d9` | `run-qmllint.sh`: no warnings outside `Panel.qml`; fails on an injected `ui.ink`, and (review round 1) on an injected `Panel.qml` member typo |
| 3.10 (README) | `8cd3cd8` | QML 90 passed; `rendered_tooltip_instances=1` |
| 2.3 (A3 structural view, `storeStep`) | `af28b74` | on an export with the 2.3 `State.js` only: node 87/87, QML 90 passed, corrected hashes 7/7 |
| 2.3 (A3 store) and 2.4 | `926fdfb` | node 87/87; QML 92 passed, 0 binding errors; corrected hashes 7/7; `grep -nE "Date\.now" State.js` prints nothing; leftover-field grep finds only two `delete` statements in a `tst_components` input builder |
| Qt 6.8 type cycle (folded into `f5d6d75`) | `f5d6d75` | CI on the first integrated head showed Qt 6.8.3 `qmltestrunner` stalling after `qt.qml.typeresolution.cycle` between `AntonToolTip.qml` and `AntonSurface.qml` (run cancelled after 15 minutes). `AntonSurface.tooltip` is typed as the base `ToolTip` from the commit that introduced it |
| Qt 6.8 qmllint (folded into `dd5b9d9`) | `dd5b9d9` | Qt 6.8.3 reports `Panel.qml`'s unresolved `qs.Ui` members as `missing-property` and a `Panel -> Panel` import cycle, and flags `SheenTitle`'s literal initial `phase` beside its value source. The gate accepts those two `Panel.qml` forms; the `SheenTitle` line has a scoped `qmllint disable`. Both Qt versions: no warnings outside `Panel.qml` |

Lane A regression checks: with the new `tst_store` and the A2 store, `test_08` and `test_09` fail; two mutations of `nextDeadlineMs` (2 ms late, and dropping the turn-freshness candidate) each fail both deadline tests. The 1 ms brute-force scan checked 85,010 start instants within 2 s of each oracle threshold and found 39 structural changes, none after its deadline.

Recorded contract details beyond D3:

- `timing.settled` (`source.active === false`) is kept in the structural timing, because the runtime emits `active: null` when the current turn is unknown, and baseline then shows no elapsed time even with a last duration. A node test pins this.
- Allowance `resetAt` and `durationS` are non-null whenever baseline showed a reset countdown, including when `used_percent` is null; `sampledAt` is gated on `remaining`. A node test pins this.
- A receipt that parses to null leaves `lastReceipt` unchanged; the timeout only applies to a non-null snapshot, so this is not observable.
- Repeated row keys (several hosts that project to the same id) get `#n` suffixes so every row still renders (`tst_keyed` test_02).
- `AntonToolTip` takes a `moving` input rather than reading `host.moving`; `controller.opened` is assigned in `Panel.onOpenedChanged` so the epoch and focus reset run before `store.restart()`.

### Visual reference correction

The five hashes below replace the archived values from task 3.4 onward (design D12). `FixtureUi.qml` formatted token counts with a trailing `.0` (`34.0K`), which production never does (`34K`). Two checks establish that the new set is the baseline popover with production formatting:

1. The unchanged `75d6548` tree, with only `FixtureUi.tokens` replaced by the `53f2407` `Panel.tokens` body, renders exactly the new set (`bash tests/run-qml.sh`, 44 passed, Qt 6.11.2).
2. The final tree, with only `ThreadCard` patched to call the fixture formatter (`tst_components` unchanged), renders exactly the archived set (lane B, repeated by the coordinator on the final tree).

With production formatting, the pixel differences from the archived images lie only in the token-label columns (x 128 to 264).

```text
682cb88630577fa9387ffc7ce42c6878dc87d54283d4ddb9baf037f7c7baf36d  anton-continuity-connections-missing.png (unchanged)
440a3db1a1a5158c7f32cdebd38641da55b6c9342ee77ce2c34e7b901765f8fd  anton-continuity-dark.png
360fe121e4f4445c908a1927ee54efb3abac3d7aac7e410976be64881450d681  anton-continuity-discovery-setup.png (unchanged)
197a1ff06eb14643adfca252471a897845380a6d78e7cd73054400d8fc5582a4  anton-continuity-discovery-unavailable.png
6da7ef879b84c0c458a10edc415e83c98bf16e54ff698f24e4f5c7447f7d944b  anton-continuity-light.png
4fc4759775d40e55567f8d2786e25d7a2049b023ad257fc5282602492f082354  anton-continuity-many-short.png
6ea070b37a61358091263130909e94e0b778023f6e691e761df3a23e5bd8b741  anton-continuity-short-with-notice.png
```

The final tree renders this set on Qt 6.11.2 and on Qt 6.8.3. `tst_components` keeps its original `inputTokens: 10000`; `capture()` now waits for rendering before grabbing, and the unchanged baseline still reproduces the archived set with that change.

## After measurement (`926fdfb` tree)

Same command, same fixtures, three 30 s windows. The harness ran on a head whose tree is identical to `926fdfb` (later commits change only this directory). The release binary rebuilt at the final head has SHA-256 `74f50d695c89e4cc01198c77df9d31ab9b1230e8e5948a6a73dceb569d76931d`, identical to baseline, and `git diff origin/main -- omarchy/anton-runtime` is empty. Runtime figures therefore measure the same executable. Because CPU and projection timings moved between sessions, the unchanged `a7ff5c0` source was measured again in the after session with the same binary ("baseline, same session").

| Metric | Baseline (recorded) | Baseline, same session | After | Note |
| --- | --- | --- | --- | --- |
| Snapshots in 30 s (median) | 10 | 9 | 10 | window alignment |
| Mean snapshot bytes | 27676.5 | 27150.2 | 27676.6 | follows the snapshot count |
| Runtime CPU seconds (median) | 0.057 | 0.071 | 0.082 | same binary; session noise |
| Runtime family peak RSS (median) | 7480 KiB | 4324 KiB | 4444 KiB | same binary; sampling noise |
| `colors.toml` opens | 0 | 0 | 0 | |
| Local / remote Herdr samples | 15 / 6 | 15 / 6 | 15 / 6 | |
| `heartbeat_seconds` | 4 | 4 | 4 | |
| Refresh latency | 2.2 ms | 2.8 ms | 2.9 ms | answered after the request in all runs |
| Local samples in the refresh burst | 2 | 2 | 2 | |
| Survives malformed stdin | yes | yes | yes | |
| Projection median (p95), 32 threads | 0.0525 (0.1351) ms | 0.1352 (0.335) ms | 0.1199 (0.2835) ms | same-session parity |
| Projection median (p95), 128 threads | 0.1223 (0.2866) ms | 0.4511 (0.8788) ms | 0.4425 (0.8169) ms | same-session parity |
| View / thread / host / allowance fields (keys/leaves) | 8/1483, 17/45, 7/8, 11/11 | same | 8/1447, 16/44, 6/7, 10/10 | A3 shape (D13) |
| **View replacements, simulated 60 s open popover** | **60** (`SnapshotStore 746ca31 logic`) | 60 | **1** (`State.storeStep`) | D13 expects 1 |
| Snapshot drops / receipt timeout | 0 / 6000 ms | 0 / 6000 ms | 0 / 6000 ms | |
| `Panel.qml` / `State.js` / `PopupContent.qml` lines | 456 / 422 / 364 | | 184 / 736 / 547 | |
| Total QML lines / files | 1977 / 12 | | 2315 / 17 | five new components |
| `required property var ui` | 7 | | **0** | |
| ToolTips / AntonSurfaces per ThreadCard | 6 / 6 | | 0 / 6 | one shared tooltip |
| CI runs QML tests | yes | | yes | |
| Allowance wire bytes / keys per row | 337 / 11 | | 337 / 11 | |
| Provider coupling (`weekly_` / `604800` / `provider ==` / `codex`) | 0/0/0/0 | | 0/0/0/0 | |
| `allowance_contract` time fields | 50, `3d 12h`; 50, `15d 0h` | | null | expected after A3 (D13) |

Architecture metrics:

| Metric | Baseline | After |
| --- | --- | --- |
| `ui.` member references | 149 | **0** |
| Preference parse calls | 17 | 9 (parse-once sites) |
| `ToolTip` declarations | 1 | 1 (`AntonToolTip.qml`) |
| Hard-coded `.local/state/omarchy` paths | 1 | **0** |
| Clock-only view changes in 60 s | 25 | **2** (D13 expects 2) |
| qmllint 6.11.2, `-I tests/qml/anton`, default import paths | 149 | 14: `Panel.qml` 13 (import 5, unresolved-type 3, unqualified 4, inheritance-cycle 1), `AntonController.qml` 1 (signal-handler-parameters, from the installed Quickshell's `QProcess::ExitStatus`) |
| `tests/run-qmllint.sh` (`--bare`, stubs only), Qt 6.11.2 and Qt 6.8.3 | not present | no warnings outside `Panel.qml` |
| qmllint against the real Omarchy shell (scratch `qs` symlink, evidence only) | 148 (Panel 9) | 23, all `missing-property` on members of the shell's `QtObject`-typed groups (`Style.font`, `Color.tooltip`, `Color.popups`), plus the `QProcess::ExitStatus` one; `Panel.qml` 4, 0 `unqualified`, 0 `unused-imports` |
| Rendered ToolTip instances (`tst_metrics.qml`) | 29 | **1** |
| `allowance_readings` | Codex 60 / 50 / +10 / `3d 12h` / `30s ago`; Synthetic 75 / 50 / +25 / `15d 0h` / `30s ago`; auth_needed null / `source unavailable` | identical |

Scope: controlled synthetic fixtures on one workstation, not a live fleet. The QML engine, rendering, GPU, remote hosts and live Omarchy integration are not measured; projection cost is measured in Node. The live install is performed by the parent after review.

### Final gates (source tree of `926fdfb`)

- `cargo fmt --check`: pass. `cargo clippy --locked --all-targets -- -D warnings`: pass.
- `cargo test --locked`, six runs on this branch: every non-lib target passed every time (bin 8, `native_navigation` 6, `native_process` 26). The lib suite (65) passed in three runs and failed one test in each of the other three, a different test in two of them (`native::…checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation` twice, `allowances::…malformed_or_oversized_values_stay_unknown_and_private_fields_never_pass` once). This is the pre-existing baseline flakiness; no Rust source changed, and it is not fixed here.
- `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs`: 87 passed, 0 failed.
- `bash tests/run-qml.sh`: 92 passed, 0 binding errors, on Qt 6.11.2 locally and Qt 6.8.3 (local download and CI); corrected hashes 7/7 (see the screenshot stability note).
- `bash tests/run-qmllint.sh`: no warnings outside `Panel.qml` on Qt 6.11.2 and 6.8.3.
- `openspec validate restructure-anton-popover --strict`: valid.
- GitHub Actions: `native` and `qml` passed on the pushed head (recorded in the PR).

### Per-commit check

Every source commit from `a3e7822` to `926fdfb` was exported and run through the JS suites, `run-qml.sh` on Qt 6.11.2 and Qt 6.8.3, and, where present, `run-qmllint.sh` on both Qt versions. All passed on every commit (JS 76 to 87, QML 44 to 92, 0 failures, lint exit 0). Screenshot hashes matched the archived set for commits that still have `FixtureUi.qml` and the corrected set from `b152fa3` onward, except in three of the fifteen runs (see below); rerunning each of those commits three times matched 7/7 every time.

### Screenshot stability

On the final tree, 1 of 8 consecutive runs produced four mismatching screenshots (dark, light, many-short, discovery-unavailable). In each, 41 or 82 pixels differ, all at x 64 to 69: the `%` glyph of a context dial is rasterised slightly differently. The same tree renders the reference set in the other runs, so this is rendering nondeterminism in the test process, not a change to the popover. Six runs of the unchanged baseline tree (44 tests) all matched. The likely cause is glyph-cache history, which lane B also observed; it is not fixed here. CI does not compare hashes.

## Review round 1

Findings from the first independent review, all accepted.

**qmllint gate accepted every `missing-property` in `Panel.qml`.** Confirmed: with `popoverTheme.reload()` changed to `relaod()` (both calls) and `barState` to `barStat` in `Panel.qml`, the old gate exited 0. `tests/run-qmllint.sh` now accepts a `Panel.qml` `missing-property` warning only as `Cannot assign to non-existent default property` (a child of an unresolved `qs.Ui` parent) or `Could not find property "<name>"` for the properties Panel writes on `Panel`, `BarIconButton`, `KeyboardPanel`, `PanelKeyCatcher` and their `anchors` group. Negative checks on a scratch copy of the tracked tree:

| Injected into `Panel.qml` | Qt 6.11.2 | Qt 6.8.3 |
| --- | --- | --- |
| `relaod()` twice, `barStat` | exit 1: `Member "relaod" not found on type "AntonTheme"` (lines 26, 88), `Member "barStat" not found on type "AntonController"` | exit 1: line 88 `relaod`, `barStat` |
| `printErors: false` on `FileView` | exit 1: `Could not find property "printErors"` | exit 1: same |
| none (current tree) | exit 0 | exit 0, 24 accepted `missing-property` |

Qt 6.8.3 does not report members used inside `onOpenedChanged` (a handler of the unresolved base), so the line 26 typo is caught only by Qt 6.11. Typos on `Color`, `Style`, `SnapshotStore` and `State` members are likewise reported as `Member ... not found` on Qt 6.11 and fail; misspelt ids were already caught by the `unqualified` rule.

**Unused `AntonController.now` and `State.threadIndex`.** Confirmed by grep: no reader of `controller.now`, and `threadIndex` was called only by its node test. Both are removed with the Panel binding and the test, and D7 and D9 now describe the implementation (cards take `now` directly; delegates resolve through PopupContent's per-group `entries` and `accounts` tables).

**Collector exit not covered.** Added `tst_store` test_10: a snapshot, then `running = false` and `exited(1, 0)`; `raw` is null, the view is disconnected, `lastReceipt` is unchanged, the retry restarts the collector within 7 s without writing to stdin, and the next line reconnects. The node `storeStep` test gains the matching `update` after `raw = null` (view empty, `deadline` null, `lastReceipt` unchanged).

The fixups were folded with an autosquash rebase, which rewrote `a5a69f1`, `92a2631`, `74d02c5` and `41c80a4` as `dd5b9d9`, `8cd3cd8`, `af28b74` and `926fdfb`; this file now cites the new hashes. Those commits change only `tests/run-qmllint.sh` and the README relative to the originals, so earlier results for their source trees still apply. The tightened lint gate exits 0 on every commit from `dd5b9d9` to this round's head on Qt 6.11.2 and Qt 6.8.3.

Gates on this round's head:

- `cargo fmt --check` and `cargo clippy --locked --all-targets -- -D warnings`: pass. `cargo test --locked`, three runs: two passed every target; one failed only the known flaky lib test `checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation` (see "Final gates"). No Rust source changed.
- `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs`: 87 passed.
- `bash tests/run-qml.sh`: 93 passed, 0 failed, on Qt 6.11.2 and Qt 6.8.3 (aqtinstall download). Corrected hashes 7/7 on Qt 6.11.2 and on two of three Qt 6.8.3 runs; the first Qt 6.8.3 run showed the four-image `%` glyph instability described under "Screenshot stability".
- `bash tests/run-qmllint.sh`: exit 0 on Qt 6.11.2 and Qt 6.8.3.
- Harness `--skip-runtime`: every metric as in "After measurement" (view replacements 1, clock-only changes 2, `required property var ui` 0) except line counts, which follow the removals: `Panel.qml` 183, `State.js` 731, total QML 2312. The runtime is unchanged, so runtime metrics were not re-measured.

## Review round 2

**The `Panel.qml` `missing-property` allowance matched message text only.** Confirmed on Qt 6.11.2 and Qt 6.8.3: with `text: "x"` or `Timer { interval: 1 }` inserted at line 48 inside `AntonController`, the round 1 gate exited 0. Either would stop `Panel.qml` loading at runtime. `tests/run-qmllint.sh` now ties each accepted warning to a location as well as a message:

- A brace-tracking awk pass gives each line of `Panel.qml` its enclosing object.
- `Could not find property "<name>"` is accepted only on a line whose enclosing object is the `Panel` root, `BarIconButton`, `KeyboardPanel` or `PanelKeyCatcher`, and only for a property that Panel writes on that type (per-type lists).
- `Cannot assign to non-existent default property` is accepted only on a line that opens a child object directly inside one of those four types.
- On Qt 6.11 or later every `missing-property` fails. The clean tree reports none there.

The finding also proposed rejecting every `missing-property` from Qt 6.9. That part was not adopted, because a clean tree would fail. Qt 6.9.3 and Qt 6.10.2 (aqtinstall, scratch only) report every read of a `qs.Ui` member: 48 and 49 `missing-property` warnings, for example `Panel.qml:24:41: Member "opened" not found on type "Panel"` (6.9.3) and `Member "opened" not found on type ""` (6.10.2). The round 1 gate already failed on both versions. The script now refuses them with `Unsupported qmllint 6.9: use Qt 6.8 or Qt 6.11 or later` and exit 1. CI uses Qt 6.8.3.

Negative checks: `bash tests/run-qmllint.sh` on a scratch copy of the tracked tree, one injection at a time into `Panel.qml`.

| Injected into `Panel.qml` | Qt 6.11.2 | Qt 6.8.3 |
| --- | --- | --- |
| none (current tree) | exit 0 | exit 0, 24 accepted `missing-property` |
| `text: "x"` at line 48, inside `AntonController` | exit 1: `48:9: Could not find property "text"` | exit 1: same |
| `Timer { interval: 1 }` at line 48 | exit 1: `48:9: Cannot assign to non-existent default property` | exit 1: same |
| three-line `Timer { interval: 1 }` block at line 48 | exit 1: same as above | exit 1: same |
| three-line `Timer` block at line 35, inside `AntonTheme` | exit 1: `35:9: Cannot assign to non-existent default property` | exit 1: same |
| `text: "x"` at line 37, inside `AntonTheme` | exit 1: `37:9: Could not find property "text"` | exit 1: same |
| `relaod()` twice, `barStat` (round 1 check) | exit 1: lines 26 and 87 `relaod`, `barStat` | exit 1: line 87 `relaod`, `barStat` |
| `printErors: false` on `FileView` (round 1 check) | exit 1 | exit 1 |
| `useActiveColour: false` on `BarIconButton` | exit 0 | exit 1: `106:9: Could not find property "useActiveColour"` |
| `tooltipText: "x"` on `KeyboardPanel` (a `BarIconButton` property) | exit 0 | exit 1: `151:9: Could not find property "tooltipText"` |

Residual: Qt 6.11 does not report writes to the unresolved `qs.Ui` types, so a misspelt `qs.Ui` property on `BarIconButton` or `KeyboardPanel` is caught only by the Qt 6.8.3 run in CI. Qt 6.9 and 6.10 are unsupported.

The lint commit changes only `tests/run-qmllint.sh` and the plugin README; no QML, JavaScript or Rust source changed. The measurement harness runs qmllint itself and does not use this script, so the measurements stand as recorded.

Gates on this round's head:

- `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked`: pass.
- `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs`: 87 passed.
- `bash tests/run-qml.sh`: 93 passed, 0 failed, on Qt 6.11.2 and Qt 6.8.3.
- `bash tests/run-qmllint.sh`: exit 0 on Qt 6.11.2 and Qt 6.8.3; exit 1 (refused) on Qt 6.9.3 and Qt 6.10.2.
- `openspec validate --all --strict`: 4 passed.

## Live install findings

The parent installed `8a3fe0e` into the live Omarchy shell (Qt 6.11.2, installed `qs.Ui` and `qs.Commons`), then rolled the plugin back to `53f2407` with a clean restart, and later installed `fa5e4a5`. Observed facts only:

1. **`8a3fe0e`.** `omarchy-shell herdr.observatory open` set status `open`, but screenshots taken about 2 s after the open showed no popover (the bar icon rendered). The shell logs show the popover had closed before each such screenshot.
2. **`53f2407` after the rollback.** The popover rendered in a screenshot taken 2 s after an IPC open. The logs show it then closed, with no close request, about 3 s after the open.
3. **`fa5e4a5` (parent's live re-test).** After an IPC open the popover rendered fully in a screenshot at 0.5 s. By 2 s its status was `closed`, and the parent had sent no close request. No terminal launch occurred after the install.
4. **Terminal launch on `8a3fe0e`.** The session manager logged one terminal launch (a remote herdr session) about 10 s after an earlier `omarchy restart shell`. Its origin could not be attributed from the logs.
5. **Unprompted close.** The close without a request is unexplained. It occurs on both the change-2 build (`53f2407`) and this change's build (`fa5e4a5`), so it is recorded as a pre-existing issue outside this change; see "Follow-up" below.
6. **Navigation guard.** The guard in `ae5859f` is verified by the QML suites (`tst_controller` test_13, `tst_popup` test_08b) and the installed-shell harness only. Live navigation was not deliberately exercised on `fa5e4a5`.

### Installed-shell reproduction

`tests/run-shell-harness.sh` (commit `493ab55`) runs `Panel.qml` in Quickshell 0.3.1, offscreen, against the installed `qs.Commons` and `qs.Ui`. Offscreen Quickshell has no layer-shell backend (`No PanelWindow backend loaded`), so a scratch copy of `qs.Ui` swaps `KeyboardPanel`'s `PanelWindow` for a `FloatingWindow`; its card, content holder, key catcher and every other shell file are the installed ones. A fake `anton-runtime` serves the synthetic `mixed-fleet` snapshot and logs every invocation. The run loads the widget asynchronously as the shell does, performs five hot reloads (two while open), opens, closes and reopens (once during the fade), moves focus and leaves the popover open for 3 s.

| Plugin tree | Result |
| --- | --- |
| `53f2407` | pass: card 360x540 with 3 of 3 thread rows at every check, stays open, 6 collector starts, 0 `--open-thread` |
| `8a3fe0e` | pass: identical |
| `493ab55` | pass: identical |
| `493ab55` with `popoverController.activate()` queued on open | fail: the popover closes after the fake navigation exits 0, 30 `--open-thread` calls |
| `493ab55` with `PopupContent` hidden | fail: content has no size, 0 of 3 rows |

One-off observation, not committed and not repeatable from the committed harness: an earlier scratch run of the same setup saved the card image for both trees, and with the synthetic snapshot and the workstation's theme palette the two 360x540 images were identical pixel for pixel (their PNG bytes differed). The invisible popover does not reproduce against the real modules on either tree offscreen, and the harness does detect both an unrequested navigation and missing content.

**What the harness does not cover.** The installed `KeyboardPanel` closes only on a pointer press on its dismiss area or on its other-monitor dismissal windows, plus its explicit close paths. The harness patches the layer-shell window, mask and dismissal windows out, so its "stays open" check does not explain the live close.

**Why the stub suite could not show it.** `tests/qml/anton` replaces `qs.Commons` with stubs and has no `qs.Ui`, so it never instantiates `Panel.qml`, `KeyboardPanel`, `PanelKeyCatcher`, `BarIconButton` or the real `Color`. A defect in how `Panel.qml` sits inside those types, or in real theme values, is outside its reach. The new harness covers that, but not layer-shell mapping, compositor keyboard focus or pointer delivery.

### Shell and session logs

Read-only; only event types and times relative to each open are recorded here.

- `8a3fe0e` session, open A: the popover opened 4 s after the restart with no IPC or `omarchy-shell` socket request; what opened it is not recorded. At +5.1 s the preferences file was written (acknowledgements are written after a successful navigation or by reconciliation), at +5.9 s the session manager logged the only terminal launch of the session, and the panel layer closed at +6 s.
- Open B (IPC): the layer closed about 1 s later with no close request and no launch. The screenshot came about 2 s after the open, after the close.
- Open C (IPC): a status query at +2 s returned `open`, the layer closed at +4 s with no close request and no launch, and the screenshot came at +5 s.
- `53f2407` session after the rollback: the popover opened by IPC, the screenshot at +2 s showed it, and the layer closed at +3 s, before the next IPC request. Later the popover was opened without any IPC request and closed 10 s later as focus moved to a terminal window.
- In every close in both sessions the panel layer took keyboard focus, focus then returned to an application window, and the layer closed. The logs do not record pointer or keyboard events, so they cannot show what closed the layer.

Conclusion: on `8a3fe0e` the popover was not drawn invisibly; it had closed before each screenshot that missed it. The same close without a request happened on `53f2407` and, in the parent's re-test, on `fa5e4a5`. The logs do not show what closed it, and nothing in them points to this change. A pointer press or other operator input is one possible cause, but it is a hypothesis the logs neither confirm nor rule out.

### Follow-up

The unprompted close is a pre-existing issue outside this change and is left open for a follow-up change: find what closes the layer-shell panel within about 1 to 3 s of an IPC open on a live desktop, starting from the `KeyboardPanel` dismissal surfaces that the harness patches out.

### Activation paths

Every route to `--open-thread` goes through `AntonController.openThread`, the only place that starts the navigation process:

- `PanelKeyCatcher.activateRequested` (Return, Enter or Space while the key catcher has focus) calls `controller.activate()`;
- the thread row `TapHandler.onTapped`;
- the thread row `Accessible.onPressAction` (an AT-SPI client's press).

No `Component.onCompleted`, `Connections`, `Qt.callLater`, timer, binding, view change, focus change or open/close handler calls `openThread` or `activate`. `53f2407` has the same three entry points (`root.openThread` from the key catcher, the row `TapHandler` and the row press action). By code inspection, a load, hot reload, focus change or snapshot update therefore cannot start navigation on either tree without one of those input events, and the harness shows 0 launches through reloads, open/close cycles and focus moves. The terminal launch in the `8a3fe0e` session followed an open that had no IPC request; its origin could not be attributed.

Two points for the parent:

- Return or Space with no thread focused opens the first visible thread. This is the specified behaviour ("Stable keyboard focus": activation without focus opens the first thread in visual order) and is unchanged, but it means any Return or Space delivered to the open popover navigates.
- Before this round, a tap or accessibility press during the 140 ms closing fade could still navigate, because the card stays visible while it fades. `openThread` now does nothing unless the popover is open (commit `ae5859f`), with regression tests `tst_controller` test_13 (activation and `openThread` while closed start no process, set no target or error, and work again once open) and `tst_popup` test_08b (a tap on a row after closing launches nothing, and the same tap after reopening builds the exact arguments). Both tests fail without the guard (93 passed, 2 failed) and pass with it. The delta spec gains "Navigation only from an open popover".

### Gates on this round

- `cargo fmt --check`, `cargo clippy --locked --offline --all-targets -- -D warnings` and `cargo test --locked --offline` (lib 65, bin 8, `native_navigation` 6, `native_process` 26): pass. No Rust source changed.
- `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs`: 87 passed.
- `bash tests/run-qml.sh`: 95 passed, 0 failed, 0 binding errors on Qt 6.11.2; the seven screenshots match the corrected reference.
- `bash tests/run-qmllint.sh`: exit 0, no warnings outside `Panel.qml`.
- `bash tests/run-shell-harness.sh` (local only; CI does not run it): pass (see above).
- `openspec validate restructure-anton-popover --strict` and `openspec validate --all --strict`: valid.
- Harness `--skip-runtime` against `8a3fe0e`: every metric identical except timing noise and `total_qml_lines` 2312 to 2316 (the guard).

## Traceability

| Requirement / scenario | Implementation | Verification | Commit |
| --- | --- | --- | --- |
| Clock-separated view updates | `State.project` (structural), readings, `nextDeadlineMs`, `storeStep`; `SnapshotStore` steps every path through `storeStep`; components read `State.*Reading(entry, now)` | node oracle equivalence (112 projections), signature constancy, deadline brute force; harness `view_replacements` 1, clock-only 2 | `aadd09b`, `ccece10`, `d8f7edc`, `af28b74`, `926fdfb` |
| Scenario: open popover with an unchanged snapshot | `storeStep` replaces only on signature change; `now` ticks while open | `tst_store` test_08 (same view over three ticks, `now` advances); `tst_popup` test_16 (stopwatch 12m 34s to 12m 39s, same view) | `926fdfb`, `d8f7edc` |
| Scenario: host stops reporting while closed | timer steps whether open or closed; host `sampled_at + maxAge` deadline | `tst_store` test_09 (60 s heartbeat, maxAge 25 s, stops within 1.5 s) | `926fdfb` |
| Scenario: threshold instants | readings and structural flags reproduce baseline | `readView of every oracle projection…`, readings at every oracle instant, pinned `settled` and reset cases; `tst_popup` test_17, test_18 | `aadd09b`, `af28b74` |
| Scenario: silent collector | `storeStep` tick drops raw after `receiptTimeoutMs` | `tst_store` test_05; node `storeStep` timeout case | `926fdfb` |
| Collector exit and restart (owner EOF) | exit handler nulls `raw` and steps an `update`; retry after 5 s | `tst_store` test_10; node `storeStep` update after `raw = null` | review round 1 |
| Stable delegate identity | `AntonKeyedModel` over `State.keyedEdits`; four keyed Repeater levels | node `keyed edits…` (2000 random cases); `tst_keyed` test_01, test_02; `tst_popup` test_14, test_15b | `a3e7822`, `95845c1` |
| Scenario: earlier thread disappears during a highlight | keyed thread rows | `tst_popup` test_13 | `95845c1` |
| Scenario: row reappears after filtering | entrance gated as before on keyed delegates | `tst_popup` test_15 (hydration, filter, machine and section collapse, reconnect, sort leave entrance at 1) | `95845c1` |
| Single popover tooltip | `AntonToolTip.qml`, one per `PopupContent`; `AntonSurface` requests it | `tst_tooltip` test_01 to test_05; `rendered_tooltip_instances=1` | `f5d6d75`, `b152fa3` |
| Scenario: hover a metric inside a thread row | surface suppression and shared source | `tst_components` test_14; `tst_tooltip` test_03 | `b152fa3` |
| Scenario: scroll while hovering | `moving` input bound to the viewports | `tst_tooltip` test_03 | `f5d6d75` |
| Preserved local preferences and shell commands | `AntonPreferences.qml` (same location, keys, types, migration); `Panel.qml` IPC target with the same six functions; `State.diagnostics` | `tst_preferences` test_01 to test_05 on byte copies of the captured ini files; node `diagnostics equal every recorded IPC string` (13) | `3d0719c`, `dea2925` |
| Scenario: existing preferences after the update | as above | `tst_preferences` test_01, test_02 (full expected file text) | `3d0719c` |
| Scenario: preferences from before concealment was unified | `privacyVersion < 2` migration | `tst_preferences` test_05 (migrates once; second load does not rewrite) | `3d0719c` |
| Scenario: diagnostics command | `State.diagnostics(store.view, Date.now())` | node diagnostics oracle | `aadd09b`, `dea2925` |
| Shell theme palette source | `AntonTheme.paletteUrl = Color.currentThemePath + '/colors.toml'`; reload on accent/background, file change, popover open and IPC `refresh` | `tst_theme` test_01 to test_04; open and refresh reloads by code inspection of `Panel.qml` (not instantiable under `qmltestrunner`); `hardcoded_omarchy_state_paths` 0 | `f5d6d75`, `b152fa3` |
| Scenario: theme switch | `watchChanges` and accent/background reload | `tst_theme` test_03 | `f5d6d75` |
| Navigation only from an open popover | `AntonController.openThread` does nothing unless `opened` | `tst_controller` test_13; `tst_popup` test_08b; installed-shell harness (0 `--open-thread` calls) | `ae5859f`, `493ab55` |
| Scenario: tap during the closing fade | as above | `tst_popup` test_08b (tap after closing launches nothing; same tap after reopening builds the exact arguments); `tst_controller` test_13 | `ae5859f` |
| Scenario: reload and reopen without input | no load, reload, open/close, focus or snapshot path calls `openThread` | installed-shell harness (hot-reload storm, reopen during the fade, focus moves, 3 s unattended: rows visible, 0 `--open-thread` calls); `tst_controller` test_13 | `ae5859f`, `493ab55` |

Programme brief acceptance items:

| Item | Result | Evidence |
| --- | --- | --- |
| (a) Split `Panel.qml` | Met. `AntonTheme`, `AntonPreferences`, `AntonController`; `Panel.qml` keeps the bar button, `KeyboardPanel`, key catcher, `.accounts.json`, IPC and wiring (184 lines, was 456) | `tst_theme`, `tst_preferences`, `tst_controller` |
| (b) Typed narrow properties | Met. `required property var ui` 0, `ui.` 0 | harness static and architecture metrics |
| (c) Pure formatters in `State.js` with node tests | Met | node formatter, parse and acknowledgement tests |
| (d) Time separation | Met. View replacements 60 to 1; thresholds equal the oracle, open and closed | node oracle and deadline tests; `tst_store` test_08, test_09 |
| (e) Keyed delegates | Met | `tst_keyed`, `tst_popup` test_13 to test_15b |
| (f) One shared tooltip | Met. Rendered instances 29 to 1 | `tst_tooltip`, `tst_metrics` |
| (g) Layout | Partly met. Named height properties replace the literals with unchanged formulas; `ColumnLayout` rejected because it moves rows by 1 px (design D11 Outcome) | `tst_popup` test_03, test_04, test_06; hashes |
| (h) Tests exercise real components; qmllint | Met. `FixtureUi.qml` removed; `run-qmllint.sh` clean outside `Panel.qml` on Qt 6.11.2 and 6.8.3, in CI; `Panel.qml` `missing-property` accepted only on `qs.Ui` objects (review round 2) | QML suite, CI |
| (i) Pixel-identical visuals | Met against the corrected reference: the baseline tree with production formatting renders the same seven images as the final head. Five archived hashes certified the fixture formatter and were replaced (see "Visual reference correction") | reverse and forward formatter checks |

Not yet done: task 4.4, the independent adversarial review. One point for it: after A3 the view can change at a deadline while the popover is closed, so the controller's acknowledgement reconciliation (which writes `privacy.ini`) can run between receipts, which baseline never did. Thresholds and outcomes are unchanged.

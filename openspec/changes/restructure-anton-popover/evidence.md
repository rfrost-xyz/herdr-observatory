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

To be recorded as lanes report.

## After measurement

To be recorded on the final head (task 4.2).

## Traceability

To be completed in task 4.3.

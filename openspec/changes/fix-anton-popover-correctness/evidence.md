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

## Traceability

Each row is filled in as work lands. Status is one of `planned`, `implemented`, `verified`.

| Requirement / scenario | Implementation | Verification | Commit | Status |
| --- | --- | --- | --- | --- |
| Stable keyboard focus | State.js focus helpers; Panel.qml, ThreadCard.qml | tasks 4.1 to 4.3 | | planned |
| · Earlier thread disappears | `focusedKey` and `reconcileFocus` | node regression test; `tst_popup` neighbour test (fails at `746ca31`) | | planned |
| · Focused thread disappears | `reconcileFocus`, `activationKey` | `tst_popup` clearing test | | planned |
| · Focused thread is filtered or collapsed | `threadKeys` from `groupThreads` | `tst_popup` filter and collapse tests | | planned |
| Explicit collector refresh | main.rs line reader, nudges, coalescing; SnapshotStore `refresh`/`restart` | tasks 3.1, 3.2, 5.2, 5.3 | | planned |
| · Operator refreshes | nudge and forced emission; QML write | process latency test; `tst_store`; harness `refresh_latency_ms` | | planned |
| · Repeated refresh requests | `REFRESH_SPACING` coalescing, local-only nudges | process burst test; harness burst probe | | planned |
| · Malformed owner input | bounded framing | framing unit tests; process malformed-then-EOF test; harness `survives_malformed_input` | | planned |
| · Collector has exited | `SnapshotStore.refresh()` restart path | `tst_store` stopped-collector test | | planned |
| Single transport freshness authority | `heartbeat_seconds`; `State.receiptTimeoutMs` | tasks 3.3, 5.1, 5.2 | | planned |
| · Heartbeat stated | snapshot key; derived timeout | key-set process test; node timeout test; `tst_store` drop timing | | planned |
| · Heartbeat missing or malformed | fallback in `receiptTimeoutMs` | node fallback cases | | planned |
| Presentation-free collection | collection.rs, main.rs, config.rs | tasks 2.1 to 2.3 | | planned |
| · Sample without theme reads | theme code and `toml` removed | key-set process test; harness `colors_toml_opens` = 0 | | planned |
| · Older peer sample | `Sample` ignores unknown fields; probe `theme: null` | legacy peer fixture; old-shape peer process test | | planned |
| · Existing configuration with theme keys | validation unchanged, keys ignored | config unit tests | | planned |
| Dead projection fields (proposal, no spec change) | State.js | task 6.1 grep and node suite | | planned |
| QML tests in CI (proposal, no spec change) | checks.yml `qml` job | task 7.1, green run on pushed head | | planned |
| Visual preservation (AGENTS.md) | no presentation edits | task 7.2 screenshot hashes | | planned |

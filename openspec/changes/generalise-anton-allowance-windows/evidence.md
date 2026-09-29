# Evidence

## Baseline measurement (`cc5f982`, origin/main)

**Harness.** The harness is `tests/measure_anton_popover.mjs`. The run used its version at `5bcfb18`, which adds the allowance metrics (`allowance_wire`, `allowance_contract`, `provider_coupling`) without changing any existing metric definition.

**Binary.** The release binary was built from the unchanged `cc5f982` worktree with `omarchy/herdr.observatory/build-native.sh` into a scratch directory. Its SHA-256 was `fbaef034f64075265b5877ff81915f4607f435e1ff26c1a80da8e50a9b113695`. That is identical to change 1's after-measurement binary, so the build is reproducible.

**Source.** `State.js` and the static metrics were read from a `git archive cc5f982` extract. `git_head` is therefore null in the JSON.

**Command.**

```sh
git archive cc5f982 | tar -x -C <scratch>/src
node tests/measure_anton_popover.mjs --binary <scratch>/anton-runtime --source-root <scratch>/src --repeat 3 --json
```

**Conditions.**

- A local 16-core Linux 7.2 workstation, using only synthetic fixtures.
- Two hosts, each with 32 agents: a local socket and a fake-SSH native peer.
- Windows of 30 s. Runtime figures are medians of three windows.
- `colors.toml` opens were counted with inotify.

| Window | CPU (s) | Peak RSS (KiB) | Snapshots | Mean bytes | colors.toml opens |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.049 | 7432 | 10 | 27674.6 | 0 |
| 2 | 0.058 | 7620 | 10 | 27676.7 | 0 |
| 3 | 0.070 | 7468 | 10 | 27674.6 | 0 |

| Metric | Baseline (`cc5f982`) |
| --- | --- |
| Snapshots in 30 s (median) | 10 |
| Mean snapshot bytes (median) | 27674.6 |
| Runtime CPU seconds (user+sys, including peer probes, median) | 0.058 |
| Runtime family peak RSS (median) | 7468 KiB |
| `colors.toml` opens in 30 s | 0 |
| Local and remote Herdr samples in 30 s | 15 / 6 |
| Snapshot `heartbeat_seconds` | 4 |
| Refresh latency after `refresh` on stdin | 2.5 ms (answered by a post-request sample) |
| Local samples in 3.2 s after 20 refreshes | 2 |
| Keeps publishing after malformed stdin, then stops on EOF | yes |
| Projection update median (p95), 32 threads | 0.054 (0.138) ms |
| Projection update median (p95), 128 threads | 0.124 (0.272) ms |
| Projected fields per view, thread, host, allowance (keys/leaves) | 8/1481, 17/45, 7/8, 10/10 |
| View replacements in a simulated 60 s open popover | 60 |
| Snapshot drops in that simulation | 0 |
| Receipt timeout | 6000 ms |
| `Panel.qml` / `State.js` / `PopupContent.qml` lines | 462 / 376 / 364 |
| `required property var ui` occurrences | 7 |
| ToolTips / AntonSurfaces per ThreadCard | 6 / 6 |
| CI runs QML tests | yes |

Added allowance metrics:

| Metric | Baseline (`cc5f982`) |
| --- | --- |
| `allowance_wire`: rows on the wire (local account and peer account) | 2 |
| `allowance_wire`: bytes per row (mean) / whole `allowances` array | 409.5 / 822 |
| `allowance_wire`: keys per row | 15: `account_id, available, daily_usage, label, lifetime_tokens, peak_daily_tokens, plan, provider, provider_label, reset_count, reset_expires_at, sampled_at, weekly_remaining, weekly_resets_at, window_seconds` |
| `allowance_wire`: windows per row | 0 (no `windows` field) |
| `allowance_contract`: Codex weekly neutral row projected with balance | no (`remaining` null) |
| `allowance_contract`: synthetic monthly neutral row projected with balance | no (`remaining` null) |
| `allowance_contract`: `auth_needed` neutral row | projected with no balance; no status text field |
| `allowance_contract`: projected fields per neutral row (keys/leaves) | 10/10 |
| `allowance_contract`: neutral wire row bytes (harness fixture) | 317 |
| Presentation `weekly_` / `604800` / `provider ==` / quoted `codex` occurrences | 2 / 1 / 3 / 5 |
| Presentation files naming a provider (proxy for files to change to add one) | 2 (`Panel.qml`, `State.js`) |

Scope and limits:

- These are controlled synthetic fixtures, not a live fleet.
- CPU and peak RSS are small and noisy at this scale.
- Remote hosts, Qt rendering and GPU are not measured.
- The JS projection is measured in Node, not the QML engine.

### Visual reference

`bash tests/run-qml.sh` on the unchanged `cc5f982` worktree passed 38 tests with no binding errors. The seven screenshots it wrote are byte-identical to change 1's reference hashes (`openspec/changes/archive/2026-09-29-fix-anton-popover-correctness/evidence.md`):

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

On `cc5f982`:

- `cargo test --locked` passed: lib 51, bin 8, `native_navigation` 6, `native_process` 24.
- The JS suites (`test_pi_hooks`, `test_omarchy_state`, `test_native_distribution`) passed 50.
- The QML suite passed 38.

## After measurement (integrated head)

Pending (task 4.2).

## Gates on the integrated head

Pending (task 4.1).

## Traceability

Status is one of `planned`, `implemented` or `verified`.

| Requirement / scenario | Implementation | Verification | Commit | Status |
| --- | --- | --- | --- | --- |
| account-allowances: Account-bound allowance observation | `allowances.rs` `snapshot` conversion | tasks 2.2, 2.4 | | planned |
| · Account moves machine | unchanged dedup in `snapshot` | task 2.2 "Several mapped accounts" | | planned |
| · Unrecognised account | unchanged mapping filter | existing tests; task 2.2 | | planned |
| · Small peer clock offset | unchanged `sanitise` | existing skew test; task 2.2 stale and future cases | | planned |
| · Several mapped accounts across machines | `snapshot` newest-per-account | task 2.2 | | planned |
| account-allowances: Account token activity | token fields not copied into `AllowanceRow` | tasks 2.1, 2.2 | | planned |
| · Supported account activity / Partial or unsupported response / Missing dates / Expired or inconsistent weekly window / Office sharing | unchanged source path (`summarise_usage`, `sanitise`, migration) | existing parity and unit tests | | planned |
| · Token activity stays off the popover wire | conversion omits token fields | task 2.2; task 2.4 process test; `allowance_wire` keys | | planned |
| account-allowances: Provider-neutral allowance rows | `model.rs` `AllowanceRow`, `AllowanceWindow`, `AllowanceStatus`; `allowances.rs` conversion and `status_text` validator | tasks 2.1, 2.2 | | planned |
| · Codex account with a weekly allowance | conversion | task 2.2 | | planned |
| · Window order does not select the pacing window | `summarise` duration selection feeding conversion | task 2.2 | | planned |
| · Past reset invalidates the balance | `sanitise` plus conversion | task 2.2 | | planned |
| · Mapped account without a current observation | conversion unavailable branch | task 2.2 | | planned |
| · Malformed or oversized values | `sanitise` bounds, typed model | task 2.2 | | planned |
| account-allowances: Legacy allowance source compatibility | unchanged `probe`, `receive`, `read_cache`, `remote`; conversion | task 2.3; task 2.4 process tests | | planned |
| · Unchanged peer row | `remote` then `snapshot` | task 2.3; task 2.4 fake-SSH legacy peer | | planned |
| · Cache written before the update | `read_cache` then `snapshot` | task 2.3; task 2.4 legacy cache start | | planned |
| · Older local runtime reads a new peer | unchanged `probe` output | task 2.3 key-set test | | planned |
| omarchy-companion: Truthful current state | `State.js` D5 projection | tasks 3.1, 3.3 | | planned |
| · One host stops reporting / Pacing thresholds and independent balance / Deficit hatch and signed pace | unchanged; D1 fixtures | converted node and QML tests | | planned |
| · Long allowance window | D5 projection, `resetLabel` | task 3.1; task 3.3 popup test | | planned |
| · Codex presentation preserved | D1 fixtures, no visible card change | task 3.3 screenshot hashes; task 4.1 | | planned |
| omarchy-companion: Provider and account collections | `State.js` D5; `AllowanceCard` hint; `State.accountAlias` | tasks 3.1, 3.2, 3.3 | | planned |
| · More than two mapped accounts | unchanged grouping | existing popup test; task 3.1 | | planned |
| · Another provider with a monthly window | D5 generic projection | task 3.3 popup test; task 3.1 | | planned |
| · Account needs authentication | D5 `statusText`; card hint | tasks 3.1, 3.2 | | planned |
| · Unavailable without source text | card hint fallback | tasks 3.1, 3.2 | | planned |
| · Absent provider | no synthesis in `project` or `providerGroups` | task 3.1 | | planned |
| · Ambiguous pacing window | D5 pacing selection | task 3.1 | | planned |
| Visual preservation (AGENTS.md) | no visible card change | screenshot hashes | | planned |
| Measurement comparability (programme) | harness additive commit `5bcfb18` | baseline above; task 4.2 | `5bcfb18` | implemented |

## Independent review

Pending (task 4.4).

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

**Source and binary.**
- The harness ran with `--source-root` set to the worktree at `b1eaf52`, so `git_head` is recorded. After the run, two Rust test functions were renamed (a fixup folded into `d8cb8c1`) and the branch was rewritten to `a2df47e`. `git diff b1eaf52 a2df47e` touches only those two test names.
- The release binary built by `omarchy/herdr.observatory/build-native.sh` has SHA-256 `74f50d695c89e4cc01198c77df9d31ab9b1230e8e5948a6a73dceb569d76931d`. The same hash was rebuilt from `a2df47e`, so the measured binary is the shipped source.
- Every later commit on the branch is documentation only.

**Command.** The harness, fixtures, conditions and repeat count are the same as the baseline:

```sh
omarchy/herdr.observatory/build-native.sh <scratch>/anton-runtime
node tests/measure_anton_popover.mjs --binary <scratch>/anton-runtime --source-root <worktree> --repeat 3 --json
```

| Window | CPU (s) | Peak RSS (KiB) | Snapshots | Mean bytes | colors.toml opens |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.054 | 7660 | 10 | 27677.1 | 0 |
| 2 | 0.061 | 4340 | 10 | 27676.8 | 0 |
| 3 | 0.055 | 4332 | 9 | 27148.9 | 0 |

| Metric | Baseline (`cc5f982`) | After (`a2df47e`) |
| --- | --- | --- |
| Snapshots in 30 s (median) | 10 | 10 |
| Mean snapshot bytes (median) | 27674.6 | 27676.8 |
| Runtime CPU seconds (median) | 0.058 | 0.055 |
| Runtime family peak RSS (median) | 7468 KiB | 4340 KiB |
| `colors.toml` opens in 30 s | 0 | 0 |
| Local and remote Herdr samples in 30 s | 15 / 6 | 15 / 6 |
| Snapshot `heartbeat_seconds` | 4 | 4 |
| Refresh latency after `refresh` on stdin | 2.5 ms, after request | 2.7 ms, after request |
| Local samples in 3.2 s after 20 refreshes | 2 | 2 |
| Survives malformed stdin, stops on EOF | yes | yes |
| Projection median (p95), 32 threads | 0.054 (0.138) ms | 0.055 (0.148) ms |
| Projection median (p95), 128 threads | 0.124 (0.272) ms | 0.127 (0.259) ms |
| Projected fields view / thread / host (keys/leaves) | 8/1481, 17/45, 7/8 | 8/1485, 17/45, 7/8 |
| `projection.allowance_fields` (legacy-shaped harness rows) | 10/10 | 12/12 |
| View replacements / drops in 60 s | 60 / 0 | 60 / 0 |
| Receipt timeout | 6000 ms | 6000 ms |
| `Panel.qml` / `State.js` / `PopupContent.qml` lines | 462 / 376 / 364 | 456 / 422 / 364 |
| `required property var ui` | 7 | 7 |
| ToolTips / AntonSurfaces per ThreadCard | 6 / 6 | 6 / 6 |
| CI runs QML tests | yes | yes |
| `allowance_wire`: rows | 2 | 2 |
| `allowance_wire`: bytes per row / whole array | 409.5 / 822 | 337 / 677 |
| `allowance_wire`: keys per row | 15 (Codex-shaped, with token activity) | 11: the D1 keys |
| `allowance_wire`: windows per row / window keys | 0 / none | 1 / `duration_s, kind, label, pacing, resets_at, used_percent` |
| `allowance_contract`: Codex weekly neutral row | remaining null | remaining 60, time 50, reset `3d 12h` |
| `allowance_contract`: synthetic monthly neutral row | remaining null | remaining 75, time 50, reset `15d 0h` |
| `allowance_contract`: `auth_needed` row | no balance, no status field | no balance, `status` and `statusText` present |
| `allowance_contract`: projected view fields | 10/10 | 12/12 |
| `provider_coupling`: `weekly_` / `604800` / `provider ==` / quoted `codex` | 2 / 1 / 3 / 5 | 0 / 0 / 0 / 0 |
| `provider_coupling`: presentation files naming a provider | 2 | 0 (see note) |

Reading the table:

- **Coupling note.** The quoted-`codex` pattern does not match the two keys `"codex:Personal"` and `"codex:Work"` in the `Panel.qml` legacy alias table (`grep -n 'codex:' omarchy/herdr.observatory/Panel.qml` gives lines 62 and 63). The presentation therefore still names Codex once, in one file, as preference-migration data. It has no projection branch. Adding a provider needs no presentation change.
- **`projection.allowance_fields`.** This metric projects the legacy-shaped rows in the harness `jsSnapshot`. Those rows now project as unavailable with the 12-key view, so the count moved from 10 to 12 with an unchanged definition (D9). `allowance_contract.view_fields` is the like-for-like figure.
- **Runtime metrics.** CPU, snapshots, bytes, samples and refresh behaviour are within noise. The runtime fixture has no configured accounts, so snapshot bytes barely move. The third window's 9 snapshots is a boundary effect. The peak RSS median fell from 7468 to 4340 KiB, but window 1 read 7660 KiB. RSS is sampled every 50 ms over a short-lived process family, so no memory improvement is claimed.
- **Wire bytes.** They fell by 72.5 bytes per row, because token activity left the wire. Codex `used_percent` is emitted as a float (`60.0`).

Scope and limits:

- These are controlled synthetic fixtures, not a live fleet. Remote hosts, Qt rendering and GPU are not measured.
- The JS projection is timed in Node, not the QML engine.
- The installed popover, both live hosts and live account refresh are not verified here. The parent does that after review.

## Gates on the integrated head

Branch `feat/anton-allowance-windows` was rebased on `origin/main` `cc5f982`. The gates ran on `b1eaf52`, and the Rust suite was rerun on `a2df47e` after the test renames.

| Gate | Result |
| --- | --- |
| `cargo fmt --manifest-path omarchy/anton-runtime/Cargo.toml --check` | exit 0 |
| `cargo clippy --manifest-path omarchy/anton-runtime/Cargo.toml --locked --all-targets -- -D warnings` | exit 0 |
| `cargo test --manifest-path omarchy/anton-runtime/Cargo.toml --locked` | lib 65, bin 8, `native_navigation` 6, `native_process` 26, all passing on `a2df47e` (see flakes) |
| `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs` | 65 passed, 0 failed |
| `bash tests/run-qml.sh` | 41 passed, 0 failed, 0 binding errors, exit 0 |
| Seven `/tmp/anton-continuity-*.png` SHA-256 | identical to the reference list above |
| `openspec validate generalise-anton-allowance-windows --strict` | valid |
| Runtime commit alone (`git archive` of the runtime commit): cargo test and JS suites | lib 65, bin 8, nav 6, process 26; JS 50, all passing |

**Flaky tests.** The two known flaky lib tests from change 1 are unrelated to allowance windows.
- On the head, `native::tests::checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation` failed in 2 of 8 runs where failures were named.
- One further head run had two lib failures whose names were not captured.
- Every other head run passed.
- On the unchanged `cc5f982` extract, both tests failed in 4 of 12 lib runs (checkpoint 3, account RPC 1).
- The tests were not masked or retried inside the suite.

## Traceability

Commits:
- `d8cb8c1` is the runtime commit.
- `1da8d5d` is the presentation commit.
- `a2df47e` is the plugin README.
- `5bcfb18` is the harness.

Rust tests are in `allowances.rs` (`A`), `model.rs` (`M`), `main.rs` (`B`) and `tests/native_process.rs` (`P`). Node tests are in `tests/test_omarchy_state.cjs` (`N`). QML tests are in `tests/qml/anton/tst_components.qml` (`C`) and `tst_popup.qml` (`Q`). Status is `verified` when the named test passed on the integrated head.

| Requirement / scenario | Implementation | Verification | Commit | Status |
| --- | --- | --- | --- | --- |
| account-allowances: Account-bound allowance observation | `allowances::snapshot`, `public_row` | A `codex_weekly_allowance_becomes_one_pacing_window`; P `unchanged_peer_allowance_row_streams_as_weekly_pacing_window` | `d8cb8c1` | verified |
| · Account moves machine | unchanged newest-per-account selection in `snapshot_at` | A `several_accounts_keep_newest_observation_one_row_each_in_account_key_order`; P `fleet_rename_preserves_account_worker_then_removal_retires_sources_and_restart` | `d8cb8c1` | verified |
| · Unrecognised account | unchanged mapping filter | A `mapped_account_without_current_observation_is_unavailable` (unmapped observation adds no row) | `d8cb8c1` | verified |
| · Small peer clock offset | unchanged `sanitise` | A `stale_or_future_observations_are_unavailable` (+1 s available, +1.001 s unavailable); N `stale and future-skewed samples are not current` | `d8cb8c1`, `1da8d5d` | verified |
| · Several mapped accounts across machines | `snapshot_at` newest-per-account, sorted by account key | A `several_accounts_keep_newest_observation_one_row_each_in_account_key_order` | `d8cb8c1` | verified |
| account-allowances: Account token activity | token fields not copied by `public_row`; `DailyUsage` removed | A `token_activity_stays_off_the_popover_wire`; `allowance_wire` keys | `d8cb8c1` | verified |
| · Supported account activity / Partial or unsupported response / Missing dates / Expired or inconsistent weekly window / Office sharing | unchanged `summarise_usage`, `sanitise`, cache | untouched `synthetic_python_oracle_parity` and existing unit tests; parity fixture unchanged | n/a (unchanged) | verified |
| · Token activity stays off the popover wire | `public_row` | A `token_activity_stays_off_the_popover_wire`; P legacy peer stream has no token text | `d8cb8c1` | verified |
| account-allowances: Provider-neutral allowance rows | `model.rs` `AllowanceRow`, `AllowanceWindow`, `AllowanceStatus`; `allowances::status_text` | M `allowance_rows_roundtrip_with_every_contract_key`, `allowance_rows_drop_unknown_and_legacy_fields_and_reject_invented_status`; A `status_text_is_bounded_and_printable`; B snapshot key-set test | `d8cb8c1` | verified |
| · Codex account with a weekly allowance | `public_row` | A `codex_weekly_allowance_becomes_one_pacing_window` | `d8cb8c1` | verified |
| · Window order does not select the pacing window | `summarise` 10080-minute selection | A `window_order_does_not_select_the_pacing_window` | `d8cb8c1` | verified |
| · Past reset invalidates the balance | `sanitise` then `public_row` | A `past_reset_and_pass_expiry_invalidate_only_their_own_fields` | `d8cb8c1` | verified |
| · Mapped account without a current observation | `public_row` unavailable branch | A `mapped_account_without_current_observation_is_unavailable`; P `profile_removal_cancels_inflight_host_and_allowance_process_groups` | `d8cb8c1` | verified |
| · Malformed or oversized values | `sanitise` bounds, typed model | A `malformed_or_oversized_values_stay_unknown_and_private_fields_never_pass`, `zero_values_stay_distinct_from_unknown` | `d8cb8c1` | verified |
| account-allowances: Legacy allowance source compatibility | unchanged `probe`, `receive`, `read_cache`; `remote` parsing moved to `peer_rows` with the same bounds | A tests below; fixture `tests/fixtures/native-allowances-legacy.json` | `d8cb8c1` | verified |
| · Unchanged peer row | `peer_rows` then `snapshot_at` | A `unchanged_peer_row_converts_through_remote_sanitising`; P `unchanged_peer_allowance_row_streams_as_weekly_pacing_window` | `d8cb8c1` | verified |
| · Cache written before the update | `read_cache` then `snapshot_at` | A `cache_written_before_the_update_converts_fresh_and_stale_rows`; P `legacy_allowance_cache_is_available_at_startup_when_codex_fails` (every Codex start fails, at most one start, cache byte-unchanged) | `d8cb8c1` | verified |
| · Older local runtime reads a new peer | unchanged `summarise`, `summarise_usage`, `receive` | A `older_local_runtime_reads_probe_and_cache_rows_unchanged` | `d8cb8c1` | verified |
| omarchy-companion: Truthful current state | `State.js` `allowanceView`, `pacingWindow`, `statusText` | N converted allowance tests (3d 12h, pace bounds, skew, reset metadata) | `1da8d5d` | verified |
| · One host stops reporting / Pacing thresholds and independent balance / Deficit hatch and signed pace | unchanged; D1 fixtures | N converted tests; Q existing popup tests | `1da8d5d` | verified |
| · Long allowance window | `allowanceView`, unchanged `resetLabel` | N `a long allowance window projects from its own duration`; Q `test_12_provider_neutral_rows_render_generically` (`75%`, `↻ 15d 0h`) | `1da8d5d` | verified |
| · Codex presentation preserved | D8 fixtures, card hint fallback | seven screenshot hashes identical | `1da8d5d` | verified |
| omarchy-companion: Provider and account collections | `allowanceView`; `AllowanceCard.hint`; `State.accountAlias` | N `providers group any configured accounts in configured order`, `account aliases prefer saved names, then the legacy table, then a stable hash` | `1da8d5d` | verified |
| · More than two mapped accounts | unchanged grouping | Q `test_06_short_popup_reserves_both_scroll_regions` (four accounts); N provider grouping | `1da8d5d` | verified |
| · Another provider with a monthly window | generic projection | Q `test_12_provider_neutral_rows_render_generically`; N `an unknown window kind projects generically` | `1da8d5d` | verified |
| · Account needs authentication | `statusText`; card hint | N `an account needing authentication shows source text and no balance`; C `test_16_auth_needed_uses_source_status_text`; Q `test_12` | `1da8d5d` | verified |
| · Unavailable without source text | card hint fallback | N `an unavailable account without source text keeps a null status text`; C `test_17_unavailable_without_text_keeps_existing_hint` | `1da8d5d` | verified |
| · Absent provider | no synthesis in `project` or `providerGroups` | N `only providers present in the snapshot form groups` | `1da8d5d` | verified |
| · Ambiguous pacing window | `pacingWindow` | N `the pacing window is the single flagged window, never list order` | `1da8d5d` | verified |
| Malformed, oversized, stale and zero values (AGENTS.md) | `allowanceView` bounds | N `malformed and oversized windows ...`, `zero stays distinct from unknown`, `status text is bounded and never synthesised`, `a row without a valid provider or account id is skipped`, `an unknown or missing status projects as unavailable`, `a legacy weekly_remaining row projects as unavailable`, `the projected allowance view has exactly the twelve contract keys` | `1da8d5d` | verified |
| Visual preservation (AGENTS.md) | no visible card change | screenshot hashes | `1da8d5d` | verified |
| Omarchy `limits` mapping (D7) | documentation only: design D7, plugin README "Allowance rows" | review | `a2df47e` | implemented (no adapter, by design) |
| Measurement comparability (programme) | additive harness commit | baseline and after tables above | `5bcfb18` | verified |

**Known deviation.** Task 2.4 originally asked for zero Codex invocations when starting from a legacy cache. At startup, `main.rs` starts the account refresh worker whenever accounts are configured, so zero cannot be shown without changing runtime behaviour, which is out of scope. The test instead proves the rows come from the cache: every Codex start fails, the cache is byte-unchanged, and starts are bounded at one. The task and spec wording were corrected in the `docs(openspec)` wording commit.

**Root README (task 3.4).** No change was needed. The root `README.md` does not describe allowance fields or Codex-only presentation; its only Codex mention concerns thread metrics.

**Alias fall-through.** `State.accountAlias` treats an empty or non-string saved or legacy alias as absent, and falls through to the hashed alias. Previously, `Panel` returned an empty `personalAlias` or `workAlias` as the displayed name. The plugin never writes an empty value: the defaults are non-empty and `toggleIdentity` only re-saves them. So this differs only for a hand-edited settings file, where the old output was a blank name. The screenshots are unaffected.

**Order.** Rows are ordered by private account key, because the configuration map is not order-preserving. That behaviour predates this change. The spec and design wording were corrected to match.

## Independent review

Pending (task 4.4).

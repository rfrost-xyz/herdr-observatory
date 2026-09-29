# Tasks

This change is programme change 2 of 3 (allowance contract, review finding 5). Ownership:

- **Lane A (runtime):** group 2.
- **Lane B (presentation):** group 3.
- **Coordinator:** groups 1 and 4, plus `tasks.md` checkboxes, `evidence.md`, `design.md` and the specs, and `tests/measure_anton_popover.mjs`.

Both lanes implement against the design D1 contract. Neither edits files owned by the other lane or by the coordinator. Lanes report their verification output and commit SHAs to the coordinator, who records them in evidence.md.

- **Lane A files:**
  - `omarchy/anton-runtime/src/allowances.rs`
  - `omarchy/anton-runtime/src/model.rs`
  - `omarchy/anton-runtime/src/main.rs`
  - `omarchy/anton-runtime/tests/native_process.rs`
  - `tests/fixtures/native-allowances-legacy.json` (new)
  - `omarchy/herdr.observatory/README.md`
  - Lane A must not edit `tests/fixtures/native-allowances-parity.json`. The legacy source shape is unchanged, so the parity fixture must still pass untouched.
- **Lane B files:**
  - `omarchy/herdr.observatory/State.js`
  - `omarchy/herdr.observatory/AllowanceCard.qml`
  - `omarchy/herdr.observatory/Panel.qml` (only `accountAlias`)
  - `tests/test_omarchy_state.cjs`
  - `tests/qml/anton/**`
  - `README.md`

## 1. Measurement baseline (coordinator)

- [x] 1.1 Add the allowance metrics (`allowance_wire`, `allowance_contract`, `provider_coupling`) to `tests/measure_anton_popover.mjs` as their own `test(bench)` commit, without changing existing metric definitions. Verify they run against the `cc5f982` binary and extract (commit `5bcfb18`).
- [x] 1.2 Build the unchanged `cc5f982` release binary, run the harness with `--repeat 3 --json`, and record the baseline, binary hash and exact command in evidence.md.
- [x] 1.3 Run `tests/run-qml.sh` on the unchanged tree and confirm that the seven `anton-continuity-*.png` hashes reproduce change 1's reference. Record the result in evidence.md.

## 2. Runtime rows (lane A)

- [x] 2.1 In `model.rs`, replace `AllowanceRow` with the D1 shape:
  - add `AllowanceStatus` (serde `snake_case`: `available`, `unavailable`, `auth_needed`) and `AllowanceWindow` (`kind`, `label`, `used_percent: Option<f64>`, `resets_at: Option<u64>`, `duration_s: u64`, `pacing: bool`);
  - remove `weekly_remaining`, `weekly_resets_at`, `window_seconds`, `available`, `lifetime_tokens`, `peak_daily_tokens`, `daily_usage`, `DailyUsage` and `AllowanceRow::expire`;
  - add model tests showing that an available row, an unavailable row (null fields, empty windows) and an available row with null `used_percent` and `resets_at` round-trip through serde with every D1 key present, and that unknown fields are not re-emitted.

  Verify with `cargo test --locked model`.
- [x] 2.2 In `allowances.rs`:
  - make `snapshot()` return `Vec<AllowanceRow>`, built by one conversion from the newest sanitised mapped row per account, as in D2;
  - add the bounded `status_text` validator from D2;
  - keep `summarise`, `summarise_usage`, `sanitise`, `read_cache`, `receive`, `remote` and `probe` output unchanged.

  Add unit tests for these spec scenarios:
  - "Codex account with a weekly allowance";
  - "Window order does not select the pacing window" (300-minute window first; assert the 10080-minute values);
  - "Past reset invalidates the balance" (window fields null, reset count kept);
  - "Mapped account without a current observation";
  - "Malformed or oversized values", covering used 101, `true`, negative reset count, more than four cache rows, and `email`/`theme` extra fields absent from the output;
  - "Several mapped accounts across machines", covering newest observation wins, one row per account and a stable order sorted by account key;
  - "Token activity stays off the popover wire";
  - zero values distinct from unknown: `used_percent` 100 (0% remaining) and reset count 0 stay numeric;
  - stale observations older than 600 s, and future skew greater than 1 s, give `unavailable`;
  - pass expiry nulls the count only;
  - the `status_text` validator accepts 80 characters and rejects 81, control characters and empty strings.

  Carry over the expiry cases from the deleted `AllowanceRow::expire` test. Verify with `cargo test --locked allowances` and the untouched `synthetic_python_oracle_parity`.
- [x] 2.3 Add `tests/fixtures/native-allowances-legacy.json` (synthetic data only). It holds a cache array in the `cc5f982` shape and two peer `--allowances-probe` outputs in that shape, with token activity plus `theme`, `email` and an unknown key. Add tests showing:
  - "Cache written before the update": `read_cache` and `snapshot` convert fresh entries to available rows and stale ones to unavailable;
  - "Unchanged peer row": the `remote` sanitising path followed by `snapshot` gives an equivalent weekly window and no extra fields;
  - "Older local runtime reads a new peer": the key sets of `probe` rows (via `summarise` plus `summarise_usage`) and of the cache written by `receive` equal the legacy key set exactly.

  Verify with `cargo test --locked`.
- [x] 2.4 In `main.rs`, assign typed rows from `allowances::snapshot` to `state.allowances`, removing the `from_value` round trip, and keep the revision bump only on change. Update the snapshot key-set test, which leaves top-level and host keys unchanged. In `native_process.rs`:
  - update the existing allowance stream assertions to D1 (`windows[0].used_percent`, `status`), keeping their values;
  - add a process test in which a fake-SSH peer returns a static legacy row with extra fields and token activity, and the stream shows an available row with a `weekly` pacing window and no token or extra fields;
  - add a process test that starts the runtime with a legacy `allowances.json` and shows the row available from the cache alone, with a counting Codex stub that always fails and a byte-unchanged cache. The startup refresh worker may start Codex once, so the test bounds starts at one rather than zero.

  Verify with `cargo test --locked --test native_process`.
- [x] 2.5 Document the D1 row contract, the unchanged peer and cache shapes, and the Omarchy mapping pointer in `omarchy/herdr.observatory/README.md`. Run and record:
  - `cargo fmt --manifest-path omarchy/anton-runtime/Cargo.toml --check`;
  - `cargo clippy --manifest-path omarchy/anton-runtime/Cargo.toml --locked --all-targets -- -D warnings`;
  - `cargo test --manifest-path omarchy/anton-runtime/Cargo.toml --locked`, reporting whether the two known flaky lib tests failed and how often in three runs.

## 3. Generic presentation (lane B)

- [x] 3.1 Rewrite the allowance part of `State.project` as in D5. Remove the `"codex"` provider default, the `"Codex"` label fallback, the `604800` duration default and every `weekly_` read. Convert the existing allowance tests in `test_omarchy_state.cjs` to D1 rows, keeping their asserted values. For example, `3d 12h`, the pace boundaries, the one-second skew and the reset metadata tests keep the same expectations. Add tests for:
  - "Long allowance window" (30 days, 25% used, 15 days left: remaining 75, time remaining 50, pace +25, reset `15d 0h`);
  - "Account needs authentication" (`statusText` `Sign in required`, no balance);
  - "Unavailable without source text" (`statusText` null);
  - "Ambiguous pacing window" (zero pacing windows, two pacing windows, pacing window listed second of two);
  - an unknown kind (`kind: 'credits_pool'`) projecting generically;
  - malformed and oversized windows (9 windows; used -1, 101, NaN, a string; `duration_s` 0, 31622401, 1.5; `resets_at` in the past; missing pacing flag), each leaving balance and pace unknown without throwing;
  - stale `sampled_at` older than 600 s, and future skew greater than 1 s;
  - zero distinct from unknown (used 100 gives remaining 0; reset count 0);
  - `status_text` bounds (80 characters accepted, 81, control characters and non-string values give null);
  - missing provider or invalid account id skipping the row;
  - an unknown status projecting as unavailable;
  - a legacy-shaped row (`weekly_remaining`, `available`) projecting as unavailable;
  - "Absent provider": two providers group correctly with `providerGroups`, and no row or group appears for a provider the snapshot lacks;
  - the projected allowance view having exactly the 11 D5 keys.

  Verify with `node --test tests/test_omarchy_state.cjs` and with `grep -nE "weekly_|604800|provider\s*===?|['\"]codex['\"]" omarchy/herdr.observatory/State.js` printing nothing.
- [x] 3.2 Change `AllowanceCard.hint` as in D6. Add the pure `State.accountAlias(account, saved, legacy, pool)` from D6 and make `Panel.accountAlias` delegate to it, passing the legacy table. `Panel.qml` changes only in `accountAlias`. Add node tests showing that:
  - saved aliases take precedence;
  - Codex Personal and Work rows get the legacy aliases;
  - a synthetic provider labelled `Personal` gets the deterministic hashed alias;
  - the result for an existing account key equals today's hash choice.

  In `tst_components.qml`, add tests showing:
  - an `auth_needed` entry with `statusText` has that hint and accessible name, and shows no balance fill, pace reading or expected tick;
  - an unavailable entry with null `statusText` hints `Allowance unavailable`.

  Verify with `node --test tests/test_omarchy_state.cjs` and `tests/run-qml.sh`.
- [x] 3.3 Convert the `tst_popup.qml` `account()` helper to D1 rows as in D8. Add a popup test rendering a Codex group, a synthetic `Synthetic` provider with a 30-day window (25% used, 15 days left) and an `auth_needed` synthetic account. Assert:
  - two provider groups labelled `Codex` and `Synthetic`;
  - the synthetic balance text `75%`;
  - the reset caption `↻ 15d 0h`;
  - the `auth_needed` card showing `—` with no fill.

  This test must not call `capture()`. Verify that `tests/run-qml.sh` passes with 0 binding errors, and that the seven `/tmp/anton-continuity-*.png` files are byte-identical to the evidence.md reference hashes (`sha256sum`). Explain any difference before accepting it. It must not be accepted for a Codex fixture.
- [x] 3.4 Update the root `README.md` only where it describes allowance fields or Codex-only presentation. Run and record `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs` and `bash tests/run-qml.sh` totals.

## 4. Integration and acceptance (coordinator)

- [x] 4.1 Integrate both lanes on the branch, rebased on `origin/main`. Run the full gates:
  - Rust fmt, Clippy (warnings denied) and locked tests;
  - the JS suites;
  - the QML suite with the binding-error gate;
  - `openspec validate generalise-anton-allowance-windows --strict`;
  - the distribution test.

  Confirm the screenshot hashes again on the integrated head.
- [x] 4.2 Build the integrated release binary and run `node tests/measure_anton_popover.mjs --binary <bin> --source-root <worktree> --repeat 3 --json`. Record the comparison with the baseline in evidence.md, including:
  - `allowance_wire` bytes and keys per row;
  - `allowance_contract` (all three neutral cases projected with values);
  - `provider_coupling` (expected: 0 `weekly_`, 0 `604800`, 0 `provider ==`, 0 quoted `codex`; the metric does not match the two `codex:` keys of the Panel legacy alias table, so evidence reports them with a separate grep);
  - the note from D9 on `projection.allowance_fields`.

  Existing runtime metrics are expected to stay within noise.
- [x] 4.3 Complete the traceability table in evidence.md, mapping every requirement and scenario in both delta specs to its implementation path, verification, result and commit.
- [ ] 4.4 Obtain an independent adversarial review of the frozen source against the proposal, specs, design, tasks and AGENTS.md. It must cover contract conformance, legacy compatibility, failure paths, data boundaries, visual preservation and tests. Fix every in-scope finding and repeat until clean. Then mark the PR ready.

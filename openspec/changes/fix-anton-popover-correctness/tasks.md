# Tasks

Lane A (runtime) owns groups 2 and 3. Lane B (QML, JS, tests, CI) owns groups 4 to 7. The coordinator owns groups 1 and 8, `evidence.md` and this file's checkboxes. Both lanes implement against the interface contract in design.md and never edit files the other lane owns.

- **Lane A files:** `omarchy/anton-runtime/src/main.rs`, `omarchy/anton-runtime/src/collection.rs`, `omarchy/anton-runtime/src/config.rs`, `omarchy/anton-runtime/src/fleet.rs`, `omarchy/anton-runtime/Cargo.toml`, `omarchy/anton-runtime/Cargo.lock`, `omarchy/anton-runtime/tests/native_process.rs`, `omarchy/herdr.observatory/README.md`.
- **Lane B files:** `omarchy/herdr.observatory/State.js`, `omarchy/herdr.observatory/Panel.qml`, `omarchy/herdr.observatory/SnapshotStore.qml`, `omarchy/herdr.observatory/ThreadCard.qml`, `tests/test_omarchy_state.cjs`, `tests/qml/anton/**`, `tests/run-qml.sh`, `.github/workflows/checks.yml`, `README.md`.

## 1. Measurement baseline (coordinator)

- [x] 1.1 Add `tests/measure_anton_popover.mjs` as its own `test(bench)` commit, with parameterised binary, source root and State.js, null-tolerant fields and optional exports; verify it runs against the `746ca31` release binary with both synthetic hosts online.
- [x] 1.2 Build the unchanged `746ca31` release binary, run the harness with `--repeat 3`, record the baseline in evidence.md, and record the hashes of the deterministic QML screenshots as the visual reference.

## 2. Presentation-free collection (lane A)

- [ ] 2.1 Remove `collection::theme`, `theme_text`, `fallback_theme`, the `toml` dependency, `State.theme`, `HostState.trend`, `HostState.metrics`, the theme revision bump and `theme_host` sample selection. Emit `"theme": null` from `collection::local`. Replace `theme_files_are_bounded_and_fifos_cannot_prevent_owner_eof` with a process test asserting the exact top-level snapshot keys and host keys from the contract and a null probe theme. Verify with `cargo test --locked --offline` and `grep -rn "colors.toml\|fallback_theme" omarchy/anton-runtime/src` returning nothing.
- [ ] 2.2 Keep `Sample` tolerant of unknown fields. Keep the legacy full-theme peer sample fixture and add a process test in which an old-shape peer result carrying a theme object reports the host connected. Verify both tests pass.
- [ ] 2.3 Keep `theme_host` and `theme_path` validation unchanged and document them as accepted and ignored in `config.rs` and the plugin README. Add unit tests showing a valid `theme_host` and `theme_path` config loads, while the existing invalid-value cases still fail. Verify with `cargo test config`.

## 3. Owner commands, refresh and heartbeat (lane A)

- [ ] 3.1 Replace the stdin drain with a bounded line reader: 64-byte limit, discard to the next newline when oversized, `\r` trimmed, non-UTF-8 and unknown commands ignored, EOF or a hard read error stopping the runtime. Unit-test the framing with split reads, oversized lines, CRLF, invalid UTF-8 and a known command after junk.
- [ ] 3.2 Add per-worker nudges, which only local-transport workers receive, plus `REFRESH_SPACING` coalescing, an immediate allowance cache recompute and forced emission after the nudged local samples arrive. Add process tests:
  - After an emission, `refresh` yields a snapshot with local `sampled_at` no earlier than the request within 1.5 s. This fails at `746ca31`.
  - A refresh sent while a local sample is in flight still yields a sample stamped after the request. Use a fixture socket that delays its reply.
  - A burst of 20 refreshes within 200 ms produces at most 4 local Herdr RPCs in the 3 s from the first request. That is two on the 2 s cadence, one immediate and one coalesced. There is no extra SSH probe or Codex invocation, counted through fixture stubs.
  - Malformed input followed by EOF still stops cleanly with descendants reaped.
- [ ] 3.3 Add documented `HEARTBEAT`, `LOOP_WAIT` and `REFRESH_SPACING` constants, emit `heartbeat_seconds`, and comment the turn-timing freshness rule. Verify the key-set test from 2.1 includes `heartbeat_seconds == 4`.
- [ ] 3.4 Document the owner-pipe commands, refresh bounds and heartbeat in the plugin README. Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked --offline`, and record each result.

## 4. Stable keyboard focus (lane B)

- [ ] 4.1 Add `focusKeys`, `reconcileFocus`, `moveFocus`, `activationKey` and `threadForKey` to `State.js` and export them. In `test_omarchy_state.cjs`, add a regression test that models the old index focus, shows it landing on the neighbour when an earlier thread disappears, and shows the key helpers keeping the same thread. Verify with `node --test`.
- [ ] 4.2 Before changing any component, write a `tst_popup.qml` neighbour-jump test against the existing `focusedThread` index API. It focuses thread `b`, removes thread `a`, and asserts that only `b`'s card is keyed. Run it, and paste the failure (thread `c` keyed) into evidence.md through the coordinator. Then switch `Panel.qml`, `ThreadCard.qml`, `FixtureUi.qml` and the inline test `ui` in `tst_components.qml` to `focusedKey`, `threadKeys` and `openThread(key)`. Port the test to the key API, adding an assertion that the activation key is `b`'s. Verify it passes. Squash the test into the `fix:` commit.
- [ ] 4.3 Add QML tests for focus clearing on thread disappearance, on status filtering and on machine and section collapse, including no highlight after re-expansion. Verify with `tests/run-qml.sh`.

## 5. Refresh and freshness in QML (lane B)

- [ ] 5.1 Add `State.receiptTimeoutMs(raw)` as defined in design D3, with comments on the other freshness constants. Test 4 s → 6000 ms, 10 s → 12000 ms, and absent, string, 0, negative, NaN and 61 → 6000 ms.
- [ ] 5.2 Add a stub `Quickshell.Io` (`Process` and `SplitParser`, matching the installed qmltypes names) under `tests/qml/anton`. Split `SnapshotStore` into `refresh()` and `restart()` as in design D2, and use `State.receiptTimeoutMs` for the drop timer. Add `tst_store.qml`, verifying that:
  - `refresh()` writes exactly `"refresh\n"` when running and starts the collector without writing when stopped;
  - `restart()` writes nothing and starts a stopped collector;
  - the view is dropped only after the derived timeout.
- [ ] 5.3 Wire `r`, middle-click and IPC `refresh` to `snapshot.refresh()`, and popover opening and the retry timer to `snapshot.restart()`. Verify by grep of the call sites and the store tests.

## 6. Dead projection fields (lane B)

- [ ] 6.1 Remove the design D5 fields and the `activityView` and `usedPercent` helpers from `State.project`. Delete tests that asserted only removed fields, and move shared behaviour assertions to retained fields. Verify with `grep -rnE "\.(gpu|inference|activity|paceStrength|pace|discoveryState|activeThreads|cpu|memory|vram)\b" omarchy/herdr.observatory tests` showing no remaining reader, and with `node --test` passing.

## 7. CI and visual preservation (lane B)

- [ ] 7.1 Add a `qml` job to `.github/workflows/checks.yml` that installs Qt 6 QML test packages via apt on `ubuntu-latest`, as in design D6, and runs `tests/run-qml.sh`. Verify the job passes on the pushed head, iterating on packages if needed.
- [ ] 7.2 Check that the deterministic QML screenshots (`/tmp/anton-continuity-*.png`) from the lane B head are byte-identical to the baseline hashes in evidence.md, and explain any difference before accepting it.
- [ ] 7.3 Update the root README where it describes refresh or theming. Run `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs` and `tests/run-qml.sh`, and record the results.

## 8. Integration and acceptance (coordinator)

- [ ] 8.1 Integrate both lanes on the branch. Run the full gates: Rust fmt, Clippy and tests; JS suites; QML suite; `openspec validate fix-anton-popover-correctness --strict`; manifest and distribution checks.
- [ ] 8.2 Build the release binary of the integrated head, run `tests/measure_anton_popover.mjs --repeat 3`, and record the comparison with the baseline in evidence.md. Expect no colors.toml opens, smaller snapshots, fewer projected fields, a refresh latency around 1 s or less, bounded burst samples and CI running the QML tests.
- [ ] 8.3 Complete the evidence.md traceability, mapping each requirement and scenario to its implementation path, verification, result and commit.
- [ ] 8.4 Obtain an independent adversarial review of the frozen source against the proposal, specs, design, tasks and AGENTS.md. Fix every in-scope finding and repeat the review until it is clean.
- [ ] 8.5 Rebase onto `origin/main`, rerun affected gates, push with `--force-with-lease`, update the PR and confirm CI is green on the pushed head. Spec sync, archive and live installation follow change-lifecycle and the parent.

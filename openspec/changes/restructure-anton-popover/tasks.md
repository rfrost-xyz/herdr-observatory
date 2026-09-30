# Tasks

This is programme change 3 of 3 (popover architecture, review findings 6 and 7). Ownership follows design D1:

- **Lane A (state and store):** group 2.
- **Lane B (popover structure):** group 3, including `tests/run-qml.sh`.
- **Coordinator:** groups 1 and 4, `tasks.md` checkboxes, `evidence.md`, `design.md` and the specs, `tests/measure_anton_popover.mjs`, `tests/capture_popover_oracles.cjs` and `tests/fixtures/popover-*`.

Both lanes implement against design D2 to D12. Neither edits the other lane's files, the coordinator's files or any Rust source. Lanes report their verification output and commit SHAs to the coordinator, who records them in evidence.md.

Each commit must leave its lane's suites green:

- Lane A: `node --test tests/test_omarchy_state.cjs`, plus `tests/run-qml.sh`, whose `tst_store` part lane A owns.
- Lane B: `bash tests/run-qml.sh` (0 binding errors, seven screenshot hashes identical) and `node --test tests/test_native_distribution.mjs`.

From task 3.4 onward, "the seven hashes" means the corrected reference set in evidence.md ("Visual reference correction", design D12): the archived set with five hashes re-rendered from the unchanged baseline tree using the production token formatter.

Gates:

- Lane B tasks 3.3 (for its commit), 3.4 and 3.5 need task 2.1 committed.
- Task 3.7 and task 3.8's diagnostics step need task 2.2.
- Task 2.3 needs tasks 3.7 and 3.8.

Stage explicit paths only. Use atomic Conventional Commits with no attribution trailers. Only the coordinator rebases.

## 1. Baseline and oracles (coordinator)

- [x] 1.1 Add the additive architecture metrics to `tests/measure_anton_popover.mjs` and the rendered-tooltip QML metric `tests/qml/anton/tst_metrics.qml` in their own `test(bench)` commit, without changing existing definitions (commit `a7ff5c0`). Verified: harness `--skip-runtime` and `tst_metrics` run on the unchanged tree.
- [x] 1.2 Build the unchanged `53f2407` release binary. Run `node tests/measure_anton_popover.mjs --binary <bin> --source-root <worktree> --repeat 3 --json` at `a7ff5c0` and run the QML, JS and Rust suites. Record the results, binary hash, command and seven screenshot hashes in evidence.md "Baseline".
- [x] 1.3 Capture the baseline oracles in commit `947ecc6`: `tests/fixtures/popover-time-oracle.json`, `popover-diagnostics-oracle.json`, `popover-privacy-v2.ini` and `popover-privacy-v1.ini`, from `53f2407` code, with the generator `tests/capture_popover_oracles.cjs`. The fixtures contain only synthetic data.
- [x] 1.4 Add the additive `allowance_readings` metric in its own `test(bench)` commit (`618d383`), so that the allowance values stay comparable after the view loses its time-derived fields. Record its baseline from a `--skip-runtime` run on the unchanged source.

## 2. State and store (lane A)

- [x] 2.1 **(A1)** Add the D2 pure helpers to `State.js` and export them:
  - `tokens`, `percentReading`, `paceText`, `parseList`, `parseObject`, `stateColourName`, `accountKey`;
  - `toggleListValue`, `boundAcknowledgements`, `reconcileAcknowledgements`, `acknowledgeNavigation`, `assignAliases`;
  - `keyedEdits` and `threadIndex`;
  - `keys` on `groupThreads` and `providerGroups` groups.

  Keep every existing export and output unchanged.

  Add node tests:
  - formatter boundaries (null, 999, 1000, 1.5e6, 1e9 and `.0` stripping for tokens; `>99%`, `<1%`, 0 and null for percentReading; unavailable, `.0` stripping and one decimal for paceText);
  - parse failures and wrong types;
  - acknowledgement bounding at 257 entries (drops the earliest key);
  - reconcile rules: not done, episode changed, and absent threads untouched;
  - `acknowledgeNavigation`, including when the thread changed before exit;
  - deterministic `assignAliases` with a seeded random, including more accounts than the pool;
  - `keyedEdits` on random permutations, insertions and removals. Applying the ops yields `after`, and no surviving key is removed.

  Verify with `node --test tests/test_omarchy_state.cjs`.
- [x] 2.2 **(A2, additive)** Add to `project` the D3 source-time fields: `usage.at`, `children.at`, `completion.at`, `timing.startedAt` and `timing.finishedTotal`, and allowance `resetAt`, `durationS` and `sampledAt`. Keep every baseline field.

  Add these functions:
  - `usageReading`, `childrenReading`, `completionReading`, `turnReading`, `allowanceReading` and `readView`;
  - `diagnostics(view, nowMs)`.

  Add `property double now` to `SnapshotStore.qml`, updated as in D3 A2. Leave the store logic otherwise unchanged.

  Node tests:
  - for every oracle projection, `readView(project(raw, t), t)` deep-equals the oracle view with `host.age` and `thread.age` removed;
  - for every diagnostics oracle case, `diagnostics(project(raw, t), t)` equals the recorded string;
  - readings on the A2 view equal that view's own baseline fields at the same instant.

  QML test in `tst_store.qml`: `now` advances while `visualUpdates` is true and stays unchanged while it is false.

  Verify with `node --test tests/test_omarchy_state.cjs` and `bash tests/run-qml.sh`.
- [x] 2.3 **(A3, after 3.7 and 3.8 are committed)**
  - Remove the D3 time-derived fields from `project`, and add `viewSignature`, `nextDeadlineMs` and `storeStep` exactly as in D3.
  - Rewrite `SnapshotStore.qml` so that `accept`, `refresh`, `restart`, collector exit and the 1 s timer all go through `State.storeStep`. The timer runs whether the popover is open or closed.
  - Keep `raw`, `lastReceipt`, `view`, `visualUpdates`, `now`, `refresh()` and `restart()`, and the oversize and malformed handling.

  Node tests:
  - the oracle equivalence from 2.2 still passes on the A3 shape;
  - for every oracle case, `viewSignature` is constant on `[t, nextDeadlineMs(raw, t))`, checked at the start, the last millisecond and random interior points;
  - `nextDeadlineMs` is never later than the first instant at which the signature changes (a brute-force 1 ms scan within ±2 s of each oracle threshold instant);
  - `storeStep` with the harness object shape: a receipt replaces the view; a tick without a deadline returns false; a tick at the deadline replaces it only when the structure changes; a timeout drops raw; a malformed receipt nulls raw without touching `lastReceipt`.

  `tst_store.qml` tests:
  - the existing six tests pass unchanged, except where they read removed fields;
  - an open popover with an unchanged re-sent snapshot keeps the same `view` object across three ticks, while `now` advances;
  - with `visualUpdates` false and a 60 s heartbeat, a host whose sample reaches `maxAge` stops reporting within 1.5 s without a new receipt (spec "Host stops reporting while the popover is closed").

  Verify with `node --test tests/test_omarchy_state.cjs` and `bash tests/run-qml.sh`. Run `node tests/measure_anton_popover.mjs --skip-runtime --json` and report `replacements.store_emulation` (`State.storeStep`), `view_replacements` and `architecture.clock_only_view_changes_60s`.
- [x] 2.4 Document the structural view, readings and store contract in the `State.js` header comments (no README change; lane B owns the READMEs). Confirm the following and report the output to the coordinator:
  - `grep -nE "Date\.now" omarchy/herdr.observatory/State.js` prints nothing (time is always passed in);
  - `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs` passes.

## 3. Popover structure (lane B)

- [x] 3.1 Extend the test stubs as in D12:
  - `qs.Commons` `Color` gains `currentThemePath`, `foreground`, `urgent` and `popups.text`;
  - `Quickshell.Io` gains `FileView` (`path`, `watchChanges`, `printErrors`, `text()`, `reload()`, and the `loaded` and `fileChanged` signals) and `StdioCollector` (`text`, `streamFinished`).

  Add `AntonTheme.qml` (D5) and `AntonToolTip.qml` (D10). Add QML tests:
  - palette parsing from a fixture file through the stub;
  - fallback to `Color.accent` and `Color.urgent`;
  - reload on the accent signal;
  - `stateColour`;
  - tooltip delay 450, timeout 6000, show/hide by source, delay restart on a source switch, hidden while `host.moving`, and geometry clamped inside the host.

  Verify with `bash tests/run-qml.sh`.
- [x] 3.2 Add `AntonPreferences.qml` (D6) with the baseline stored properties, migration and mutators. `location` is a required creation-time property. QML tests, in which `run-qml.sh` or the tests copy `tests/fixtures/popover-privacy-*.ini` byte-for-byte into a unique temporary location first:
  - the v2 file loads the expected typed values;
  - a toggle round trip rewrites only the changed key in the same encoding, compared against expected file text;
  - `setAcknowledgements` bounds to 256;
  - `setIdentity` writes aliases only when concealing;
  - the v1 file migrates to `namesHidden` true and `privacyVersion` 2 once, and a second load does not rewrite it.

  Verify with `bash tests/run-qml.sh`. Confirm that no settings file for `qmltestrunner` appears in `~/.config` after the run (list before and after, and report).
- [x] 3.3 **(commit needs 2.1)** Add `AntonController.qml` (D7) using the A1 helpers. QML tests, using the `Process` stub and a temporary preferences file:
  - `openThread` builds `[runtimePath, "--open-thread", host, id]`, with the binding JSON appended when present;
  - an invalid route sets `Invalid thread route. Try again shortly.` and launches nothing;
  - a second open while running is ignored;
  - exit 0 on a done target acknowledges and persists, emits `closeRequested` and clears the error;
  - a non-zero exit with empty stderr sets `Could not open this thread in Herdr.`;
  - a later non-done or new-episode view removes the acknowledgement;
  - `barState` ignores acknowledged episodes;
  - `moveFocus`, `activate`, focus reconciliation, and `toggleList` clearing focus and bumping `visualEpoch`;
  - `observedChange` and `newThreads` fire only when open with motion enabled and the epoch is unchanged.

  Verify with `bash tests/run-qml.sh`.
- [x] 3.4 **(needs 2.1)** Convert every component to the D8 typed inputs and remove every `required property var ui`, `ui.` reference and formatter duplicate. Use `State.tokens`, `State.percentReading` and `State.paceText`, and the theme for colours.

  In `Panel.qml`, instantiate `AntonTheme`, `AntonPreferences` and `AntonController`, pass them to `PopupContent`, and keep the bar button, `KeyboardPanel`, key catcher, IPC functions and open behaviour as in D7. The IPC `diagnostics` body stays the baseline code until task 3.8.

  Remove the palette `FileView` path literal in favour of `AntonTheme.paletteUrl`. Port `tst_popup.qml` and `tst_components.qml` to the real theme, controller and preferences with the former fixture colours, and delete `FixtureUi.qml`. Keep each test's assertions, adapting only how it reaches the component. Keep `tst_metrics.qml`'s scene.

  Verify with `bash tests/run-qml.sh` (0 binding errors, the seven hashes identical to evidence.md) and `grep -rn "var ui\|ui\." omarchy/herdr.observatory/*.qml` printing nothing.
- [x] 3.5 **(needs 2.1)** Add `AntonKeyedModel.qml` and key the four Repeater levels (D9). First add a QML test that establishes Repeater retention (same delegate object after `move` and after removing an earlier key). Then add the spec scenarios:
  - "Earlier thread disappears during a highlight": the delegate object for the key is identical, `stateFlash` is still running, and no other card flashes;
  - reorder keeps objects;
  - "Row reappears after filtering": entrance stays at 1;
  - hydration, reconnect, section and machine collapse and expand, and sorting do not change `entrance`.

  The focus tests (`test_08` to `test_11`) must still pass. Verify with `bash tests/run-qml.sh` and the seven hashes.
- [x] 3.6 Replace the `PopupContent` column arithmetic with the D11 `ColumnLayout` and named properties. Keep `threadViewport`, `allowanceViewport`, `threadContent`, `moving` and `implicitHeight` semantics. Verify with `bash tests/run-qml.sh`: `test_03`, `test_04` and `test_06` unchanged, and the seven hashes identical. Record any fallback used. **Deviation:** `ColumnLayout` moved rows by 1 px and the pinned-height fallback did not help, so `PopupContent` keeps a `Column` with the named height properties and unchanged formulas (design D11 Outcome, commit `3a513c1`).
- [x] 3.7 **(needs 2.2)** Move every time-derived display to readings of `entry` and `now`:
  - `ThreadCard` row hint and the three usage hints: `State.usageReading(entry.usage, now)`;
  - the stopwatch and its hint: `State.turnReading`;
  - the child dial hint: `State.completionReading`;
  - `AllowanceCard`: the expected tick, pace reading, pace strip and hatch, `reset` caption and pace text use `State.allowanceReading(entry, now)`.

  `PopupContent` passes `now` from the store. Tests pass a fixed `now` equal to the former fixture time, which keeps the screenshots identical.

  Add QML tests:
  - advancing `now` by 5 s changes a fresh active turn's stopwatch text from `12m 34s` to `12m 39s` without changing `view`;
  - a stale turn's stopwatch does not change;
  - advancing `now` changes the allowance reset caption at the hour boundary.

  After this task no QML reads `usage.age`, `timing.elapsed`, `timing.total`, `timing.age`, `completion.age`, `children.age`, `entry.timeRemaining`, `entry.paceDifference`, `entry.reset` or `entry.age` from the view. Check with a grep and report its output. Verify with `bash tests/run-qml.sh` and the seven hashes.
- [x] 3.8 **(diagnostics step needs 2.2)**
  - `Panel.qml` IPC `diagnostics` returns `State.diagnostics(snapshot.view, Date.now())`, and the IPC function set is unchanged.
  - Add the five new QML files to `install.sh` `files` and to both `uninstall.sh` lists (the `case` allowlist and the `rm` loop).
  - Update `tests/test_native_distribution.mjs` so the navigation assertion reads `AntonController.qml` (`State.navigationArgs(entry)`) and the file-list checks cover the new files. Assert that every plugin `*.qml` and `State.js` appears in all three lists.

  Verify with `node --test tests/test_native_distribution.mjs` and `bash -n` on both scripts.
- [x] 3.9 Add `tests/run-qmllint.sh` (D12) and a `qml` job step in `.github/workflows/checks.yml` that runs it with the installed Qt's qmllint. Remove unused imports and fix every warning so that the plugin files other than `Panel.qml` report 0 warnings with `-I tests/qml/anton`, and `Panel.qml` reports only `qs.Ui` import and unresolved-type warnings. (Result: `Panel.qml` also reports the consequences of the unresolved base, `unqualified` for `anchors.fill: parent` and `inheritance-cycle`, and on Qt 6.8.3 `missing-property` for members of the `qs.Ui` types; the gate accepts only those forms. See evidence.md.)

  Confirm which module qmllint resolves for each import and that the non-Panel files import only stubbed or Qt modules. The CI run with Qt 6.8.3 is authoritative; fix or document any warning that appears only there.

  Also run qmllint locally with a scratch `-I` directory whose `qs` symlink points at `~/.local/share/omarchy/shell`. Report `Panel.qml` warnings by category (target 0 `unqualified` and 0 `unused-imports`). Record the output for the coordinator; do not commit any path from the home directory.

  Verify with `bash tests/run-qmllint.sh`.
- [x] 3.10 Update `omarchy/herdr.observatory/README.md` where it describes the popover file structure, fixtures or QML tests (component responsibilities, keyed delegates, the shared tooltip, time separation). Run and report:
  - `node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs`;
  - `bash tests/run-qml.sh` totals and `ANTON_METRIC rendered_tooltip_instances` (expected 1);
  - `bash tests/run-qmllint.sh`.

## 4. Integration and acceptance (coordinator)

- [x] 4.1 Integrate both lanes rebased on `origin/main`, then run the full gates:
  - Rust fmt, Clippy (warnings denied) and locked tests, which must be unchanged, with baseline flakiness noted;
  - the JS suites;
  - `tests/run-qml.sh` with the binding-error gate;
  - `tests/run-qmllint.sh`;
  - `openspec validate restructure-anton-popover --strict`.

  Confirm that `git diff origin/main -- omarchy/anton-runtime` is empty and that the seven screenshot hashes on the integrated head are byte-identical to evidence.md.
- [x] 4.2 Build the integrated release binary and confirm its SHA-256 equals the baseline `74f50d69…`. Run the same harness command with `--repeat 3 --json` on the final head. Record the comparison in evidence.md with the D13 expectations: existing metrics with identical definitions, the architecture metrics, and `rendered_tooltip_instances`. Explain any deviation.
- [x] 4.3 Complete evidence.md traceability, mapping each requirement and scenario in the delta spec, and each acceptance item (a) to (i) of the programme brief, to its implementation path, verification, result and commit.
- [ ] 4.4 Obtain an independent adversarial review of the frozen source against the proposal, spec, design, tasks and AGENTS.md. It must cover:
  - time-rule equivalence (open and closed);
  - preference compatibility;
  - IPC shape;
  - delegate identity and entrance rules;
  - tooltip behaviour;
  - qmllint;
  - install lists;
  - visual preservation;
  - private-data boundaries.

  Fix every in-scope finding, rerun the affected gates and repeat until the review is clean. Then mark the PR ready for review. Live installation is performed by the parent, not by this change's workers.

# Local acceptance evidence

## Scope and ownership

This change is local development in the existing dirty worktree. Earlier accepted changes are preserved. No commit, forge operation, Docker mutation, remote installation or service change is part of this delivery. Root owns installed writes, shell restart and live acceptance. Collection and UI workers have disjoint ownership; this coordinator owns specifications, documentation and combined gates.

## Source evidence

Installed Codex saved-event field inspection was numeric-only: `started_at` and `completed_at` are integer Unix seconds, while `duration_ms` is integer milliseconds. One historical interval differed materially between timestamp bounds and `duration_ms`, so the implementation consistently uses validated wall-clock timestamp bounds. Raw event content and identifiers were not copied into evidence.

## Requirement-to-check mapping

| Requirement | Evidence |
| --- | --- |
| Turn wall time, aborts, duplicate/coverage guards | Native parser and collection Python regressions; projection/Qt timing checks |
| Private restart checkpoint validation and bounded writes | Python security/restart/expiry/identity/throttle regressions; root warm-restart acceptance |
| Native-only timing / unchanged Work contract | Python technical/Work projection regression; root live publication check |
| Connection phases | Core and projection tests; synthetic connecting/unreachable capture; root restart |
| Fixed header, bounded threads, allowance access | Production PopupContent Qt tests, crowded short-height capture and root live view |
| Monochrome stopwatch / failure notch | Production ThreadCard and MetricDial Qt checks; dark/light captures |
| Installation and guarded removal | Temporary-install Python fixture verifies added components and owned checkpoint removal |
| Existing allowance visual contract | Existing allowance projection/Qt tests and unchanged AllowanceCard source in this change |

## Synthetic visual inspection

Production components were rendered in offscreen Qt using synthetic data and theme stubs. The coordinator inspected all four temporary captures: dark and light themes with long title/branch/timer; a crowded twenty-thread, six-machine short viewport; and connecting/unreachable/missing readings with nearly empty allowance. Titles elide beside the timer, header and allowance access remain visible, dials and failures fit, and the normal three-thread view remains compact. No user theme was changed and no live screenshot is committed.

## Gates

Source frozen after worker edge review. Coordinator reran the complete Python suite with permission for isolated Unix-socket fixtures: **192 tests passed**. The eight JavaScript suites passed; direct native projection run passed **36 tests**. Production Qt 6 offscreen tests passed **24 tests**, without warnings. Manifest validation, installer/uninstaller shell syntax, JavaScript syntax, `git diff --check` and strict OpenSpec validation passed (six items).

Commands:

```sh
python -m unittest discover -s tests -v
node --check web/app.js
node --test tests/test_ui.cjs tests/test_wasm.mjs tests/test_background.mjs tests/test_title.mjs tests/test_music_title.mjs tests/test_pi_hooks.mjs tests/test_allowances.mjs tests/test_omarchy_state.cjs
node tests/test_omarchy_state.cjs
QT_QPA_PLATFORM=offscreen QT_QUICK_CONTROLS_STYLE=Basic /usr/lib/qt6/bin/qmltestrunner -import tests/qml/anton -input tests/qml/anton -o -,txt
omarchy-plugin-validate omarchy/herdr.observatory
bash -n omarchy/herdr.observatory/install.sh omarchy/herdr.observatory/uninstall.sh
openspec validate --all --strict
git diff --check
```

## Narrow source inventory

Collection: `hooks/codex_usage.py`; `observatory/native.py`, `core.py`, `probe.py`; `tests/test_turn_continuity.py` (new), `tests/test_native_collection.py`, `tests/test_omarchy_runtime.py`; `omarchy/herdr.observatory/runtime.py`.

UI and ownership: `omarchy/herdr.observatory/Panel.qml`, `State.js`, `ThreadCard.qml`, `MetricDial.qml`, `PopupContent.qml` (new), `SectionHeader.qml` (new), `install.sh`, `uninstall.sh`; `tests/test_omarchy_state.cjs`, `tests/test_omarchy_install.py`; `tests/qml/anton/tst_components.qml`, `FixtureUi.qml` (new), `tst_popup.qml` (new), `qs/Commons/Style.qml`.

Coordinator: `AGENTS.md`, `omarchy/herdr.observatory/README.md`, and this change's proposal, design, tasks, evidence and two specification deltas. Canonical specifications were synchronised after final live acceptance; all six delta requirements match and unrelated requirement/scenario blocks were preserved. `AllowanceCard.qml` and existing remote/web implementation are unchanged by this change. The runtime entry point adds ownership-gated checkpoint retirement only.

## Independent review and installed acceptance

Independent review identified three issues: late old completions overwrote newer timing; extreme short-height allowance allocation could hide all threads; and uninstall could race a final checkpoint write. All three were remediated with regression coverage. Timing preserves safe newer evidence, the layout accounts for notice height and reserves both scroll regions at 240px, and bounded checkpoint/ownership retirement prevents late writes or startup. An injected post-retirement removal failure verifies guarded uninstall retry while preserving unrelated state. Follow-up review extended chronology protection to backwards/conflicting starts and applied the existing installed-owner guard to registry mutation and loaded reporter/allowance-receiver state writes. Registry reads no longer create files. In-flight/late writes and retirement when state never existed have regression coverage. Final combined gates above include these fixes.

Renewed independent exact-source review approved the final source with no remaining actionable findings. The reviewer independently reran 15 continuity, five runtime and one installation test on the final backend; the unchanged UI had independently passed 34 projection and 23 Qt tests without warnings. Root subsequently completed live acceptance, as recorded below.

## Live-discovered clock skew remediation

Root live acceptance observed native remote timing intermittently dropping because
the remote host clock was approximately 16–19 ms ahead. Incoming native cursors
were caught up but strict future validation rejected them. The final source allows
at most one second for native cursor retention and native timing observation
validation only. Original cursor/observation timestamps remain unchanged; display
age floors at zero inside the tolerance. Usage and allowance freshness rules are
unchanged. Tests cover 20 ms and exact-one-second offsets through checkpoint
prune/load/outbound replay, a newly started remote turn, expiry and rejection
beyond the bound. The generated ephemeral SSH module test verifies transport
imports after the shared constant change. Coordinator final gates: 192 Python,
36 projection and 23 Qt tests passed. Independent renewed review approved this
source after rerunning 16 continuity, 25 native collection and 36 projection
tests, with no actionable findings. Root's real read-only SSH probe then accepted
all five samples, retaining original small negative ages (approximately 19–45 ms
ahead), with caught-up cursors and native timing present. Corrected installed
visual and warm-restart acceptance subsequently passed, as recorded below.

## Initial installation and resolved desktop-lock interruption

Root installed the first reviewed build with all 21 staged files matching and
private configuration preserved. Cold startup showed both hosts connecting at
4.36 seconds, both connected with all three usage readings at 4.79 seconds, and
all three timing readings (two current, three complete totals) at 26.51 seconds.
The runtime remained one collector, 31,476 KiB RSS and seven threads, with no
local port 8789 listener. Existing remote Work publication remained current for
both hosts with metrics and both allowances; native timing was excluded.

The desktop lock temporarily prevented installing the later reviewed skew
correction; the shell explicitly refused restart. The user subsequently unlocked
the desktop and the correction was installed and verified, as recorded below.
Synthetic visuals were inspected by root and coordinator. All five task-owned
captures were removed without touching source fixtures. No worker-owned temporary
scripts or directories remain. Root retained staged update and rollback artefacts
through final acceptance, then removed its nine explicit task-owned temporary
items (checked public backup/stage directories, diagnostic script, aggregate
results and live captures). No private installed configuration or user data was
touched. All task-owned temporary artefacts are now removed.

## Final installed acceptance, 26 September 2026

The user unlocked the desktop. Root confirmed the desktop was no longer secure,
verified source/stage byte parity including every source member of the runtime
bundle, installed the reviewed skew correction (21 public files, private config
preserved), refreshed owned hooks and restarted the shell. Corrected warm startup
reported all three thread timings, all usage and both allowances in 2.1 seconds.
Private checkpoints contained three rows across two hosts, 6,552 bytes, mode 0600.
One collector remained at 31,688 KiB RSS with seven threads; no local port 8789
listener or Anton QML errors were present.

The live layout was inspected: both current timers advanced, the idle last-turn
duration remained fixed, and allowances retained their accepted design. Existing
remote Work feed readings were current (3.0 and 6.1 seconds for the two hosts),
with both metrics and allowances present and native timing absent from web output.
Corrected source, staging and installed bytes matched.

A live cursor warp appeared not to trigger the timer hover. Real Qt mouse movement
against the unchanged production component proved its hit area, hover highlight,
parent-tooltip suppression and exact current/total timing tooltip. A regression
was added without changing production implementation. Coordinator independently
reran all 24 Qt tests without warnings. Cursor-warp behaviour is not treated as
evidence of a UI defect. Root authorised canonical sync and local archive after
this hover verification passed.

Final gates: 192 Python, 36 native projection and 24 Qt tests; eight JavaScript
suites; manifest, syntax, strict specification and whitespace checks. All source
findings were independently reviewed and resolved. No Git/forge, Docker or remote
service mutation occurred.

## Specification finalisation

All six delta requirements were synchronised into the two canonical capabilities
with exact block parity; prior unrelated requirements and scenarios were preserved.
Strict validation passed before archive. The final hover regression and final
installed acceptance evidence were independently approved, with no remaining
source findings. All twelve tasks are complete.

Archived locally as `2026-09-26-improve-anton-thread-continuity`. Post-archive
strict validation passed all five canonical capabilities; the active change list
is empty. Archived delta/canonical parity, preserved metadata, twelve checked
tasks and whitespace validation were confirmed. No installed plugin files changed
during specification finalisation.

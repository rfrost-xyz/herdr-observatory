# Local verification evidence

## Scope and ownership

- Worktree: `feat-omarchy-companion-concept`. Existing uncommitted session work preserved.
- Coordinator: `anton_refinement`. Workers: `anton_ui`, `anton_collection`, both Astra/high.
- UI worker owns QML/projection, manifest and owned-file installer lists. Collection worker owns native/runtime/adapters and Python tests. Coordinator owns documentation/specification and combined acceptance.
- Local development only. No commits, push, merge, Docker change or remote installation.
- Primary agent owns independent review, installed-plugin replacement and actual graphical/runtime acceptance. Those gates completed as recorded below.

## Requirement traceability

| Requirement | Implementation | Verification |
| --- | --- | --- |
| Consistent allowance pace | `State.js`, `AllowanceCard.qml` | Projection covers reserve, every deficit, exact equality, expiry and near-reset denominator avoidance; no signed-balance ambiguity |
| Provider/account collections and private concealment | `allowances.py`, `refresh-identities.py`, `Panel.qml` | Native mapping compatibility, three-account handling and identity tests; provider-group projection includes reserved-object-name case |
| Compaction/outcome truthfulness | `native.py`, `codex_usage.py`, `probe.py`, `ThreadCard.qml` | Complete/incomplete history, original timestamps, malformed partitions, legacy totals and outcome wire tests |
| Hook-independent local and SSH updates | `native.py`, `core.py`, `runtime.py` | Synthetic start/completion/resume/interruption and restart without hooks; ephemeral subprocess fixture verifies no remote files or raw paths/content |
| Lean owner-bound runtime | `core.py`, `probe.py`, `runtime.py` | Native profile skips metrics/history/trends and local subprocess; default web path and explicit publisher/music preserved; owner EOF test |
| Stable interaction and acknowledgement | `State.js`, `SnapshotStore.qml`, `Panel.qml` | Stable row order, source reconnect and native generation/episode projection; acknowledgment only in successful exact-target navigation exit branch |
| Restrained motion | Eight reusable QML components | Real component Qt fixture: initial static, approximately 300 ms arrival, instant collapse cancellation, compaction feedback, closed/reduced-motion cancellation |
| Clean install and removal | `build-runtime.py`, `install.sh`, `uninstall.sh` | Native bundle excludes server/web assets, hook round-trip preserves unrelated entries, installer fixture removes new components and registry |

## Completed gates

- `python -m unittest discover -s tests -v`: 173 tests passed after final review remediation (5.612 seconds); socket fixtures ran outside the sandbox. Native coverage includes 25 cases and 5,280 malformed lexer/state continuations.

- `node --check web/app.js`: passed.
- `node --test tests/test_ui.cjs tests/test_wasm.mjs tests/test_background.mjs tests/test_title.mjs tests/test_music_title.mjs tests/test_pi_hooks.mjs tests/test_allowances.mjs tests/test_omarchy_state.cjs`: eight test files passed.
- `node tests/test_omarchy_state.cjs`: 28 projection cases passed (worker final run).
- `QT_QPA_PLATFORM=offscreen QT_QUICK_CONTROLS_STYLE=Basic /usr/lib/qt6/bin/qmltestrunner -import tests/qml/anton -input tests/qml/anton -o -,txt`: 12 Qt tests passed after final smoothing/cancellation remediation; no warnings.
- `python -m unittest discover -s tests -p test_omarchy_install.py -v`: passed guarded install/uninstall fixture.
- `omarchy-plugin-validate omarchy/herdr.observatory`: passed.
- Qt 6 `qmlformat` parsed production QML, including typed IPC in `Panel.qml`.
- `openspec validate --all --strict`: six items passed during integration; repeat after final spec synchronisation.
- `git diff --check`: passed during integration.

## Independent review and live acceptance

The independent reviewer accepted the final source with no actionable findings.
Review remediation covered unterminated oversized tails, shared cursor limits,
allowance numeral colour, mid-animation cancellation and malformed resumed lexer
state. All findings were fixed and regression-tested. The final reviewer also
exercised 500 valid nested JSON records across varied chunk boundaries, and all
25 native tests. The QML reviewer independently reran all 12 component tests.

Actual fleet data exposed ordinary oversized `CommandExecution`, `McpToolCall`,
`response_item` and `compacted` records. The bounded streaming classifier now
preserves complete compaction coverage for those validated types, rejects
ambiguous/invalid records, and retains only grammar state and fixed candidate
masks. Explicit publication metrics remain collected for their downstream user;
unpublished Anton hosts avoid that work.

Primary-agent installed/live acceptance on 25 September 2026:

- All 19 staged plugin files matched source byte for byte. The runtime bundle
  contained the final reviewed modules, owned native hooks were refreshed and
  the shell was restarted.
- Both configured hosts reported, all three threads had usage and both account
  allowances were available. Measured local compaction counts were zero and
  eight; the remote count was one. The remote completion dial showed seven of
  seven, while the active local parent correctly retained an unfinished child.
- The actual unlocked-desktop popover showed three compact rows without overflow,
  muted idle metrics, amber active metrics, row hover, aligned allowance numeral
  and graph colour, and tooltips beside the pointer/metric.
- Arrival/expansion/reduced-motion behaviour was exercised through production QML
  components in the Qt fixture, not by injecting lifecycle events into live work.
- Exactly one plugin-owned runtime was present (33,264 KiB RSS, seven threads).
  No local port 8789 listener or independent observer service existed. Observation
  history remained empty.
- The unchanged remote Work receiver continued to receive current host/account
  data. Both host metric objects were present after restoring explicitly used
  publication metrics. No remote installation or container change was made.
- Install/uninstall fixtures prove the new QML files and native module are bundled,
  the populated private session registry is removed, and unrelated hooks/plugins
  and unknown-file safeguards remain intact.

## Local completion

Canonical `omarchy-companion` and `harness-telemetry` specifications were merged
from the reviewed deltas, with every existing scenario retained and exact delta
parity checked. All nine tasks are complete. Final strict validation and archive
are recorded by the archive operation. No commit, push, merge, MR update, remote
service mutation or branch/worktree removal was performed. Existing session work
remains in the authorised development worktree. Task-owned temporary fixtures and
logs were removed; permanent regression fixtures remain under `tests/qml/anton`.

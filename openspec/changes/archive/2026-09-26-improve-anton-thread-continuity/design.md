## Context

The native plugin owns one collector and follows validated local and ephemeral SSH Codex sources. Replay cursors are currently memory-only. The QML popover has compact four-slot metrics but shares one scroll region. The existing worktree contains earlier accepted changes; this change uses an explicit file inventory and independent review rather than treating all dirty files as its delta.

## Goals / Non-Goals

Goals: show useful turn wall time; recover validated replay efficiently; distinguish initial connection from failure; keep thread navigation and allowances usable at constrained heights; indicate confirmed child failure subtly; verify production components across synthetic themes and states.

Non-goals: inference-time measurement, raw transcript display, a fifth metric, new services, changed web schema, remote deployment, redesigned allowance styling or animated collapse.

## Decisions

### Wall-clock turn bounds

Read saved `task_started`, `task_complete` and `turn_aborted` event fields within the existing bounded native parser. Unix-second `started_at` and `completed_at` define elapsed time. Native `duration_ms` is not a substitute: live numeric inspection found a historical duration differing from timestamp bounds. Completed and aborted intervals contribute once to accumulated turn time; idle gaps do not. Unsupported or ambiguous coverage remains unknown. Hashed bounded associations prevent duplicate events from double-counting and never enter public telemetry.

The native-only `technical.turn_timing` object carries `active`, `started_at_s`, `observed_at_s`, `last_duration_s`, `last_outcome`, `total_finished_duration_s`, `complete` and `freshness_seconds`. The latter is at least twelve seconds and otherwise three source polling intervals. A successful, exact-bound, caught-up source read may advance observation time. Loading a checkpoint cannot. Native cursor retention and native timing observation validation tolerate at most one second of transport clock skew, preserving original timestamps; display age floors at zero only within that tolerance. Larger future values and expired readings remain rejected. Existing usage and allowance timestamp validation is unchanged. Presentation freezes elapsed time when observation freshness expires. Existing Work publication must strip the native-only object and preserve its downstream contract.

### Private restart checkpoints

Retain replay state in an owner-only local `replay-checkpoints.json`, including configured SSH cursors, bounded globally to 32 sessions, 256 KiB and 24 hours. Keys and associations are hashed; payload content, source paths and raw native IDs are excluded. Version, exact session/header and file identity, replacement/truncation and expiry checks gate reuse. Checkpoints are parser progress, not ready-made current readings. Source reads still validate freshness after restart. Writes are atomic, meaningfully changed state is throttled, and final flush is attempted within owner shutdown. Access-time-only churn must not cause a full rewrite every poll. Guarded uninstall owns this filename and its private writer lock. Flush opens an existing owner lock and rechecks its current inode under the lock; it never creates the lock. Installed owner-marker locking gates constructor startup. After disabling the widget, uninstall retires checkpoint state under both ownership locks and marks the installed ownership marker retired before deleting runtime files. The retired marker blocks startup and permits guarded cleanup retry after a later removal failure. The same guard covers collector binding-registry mutation and already-loaded reporter/allowance-receiver state preparation and writes. This excludes in-flight writers and prevents delayed startup or final flush from recreating removed owned state; unsafe or busy retirement fails visibly within a bounded wait.

### Connection and viewport boundaries

Add `connection_state` while retaining legacy `online`. Hosts start connecting, become connected after a valid sample, and unreachable after failed/expired collection. No stale cached threads are relabelled as current. Extract reusable production popup content so tests exercise the actual fixed header, thread viewport and allowance viewport. Bound crowded content without reserving needless empty space for short lists. Keyboard selection reveals rows in the thread viewport and pointer-following tooltips use the correct surface coordinates.

### Compact signals

Place a stopwatch and duration beside the title without increasing row height. A confirmed failed count produces a small red notch on the existing subagent dial, while hover retains distinct completed/running/interrupted/failed/unknown counts. All collapse stays immediate. Existing arrival, status, sheen and reduced-motion guards remain intact. Preserve the accepted allowance fill, static deficit hatch, positive-only surplus strip and hover effects exactly.

## Risks / Trade-offs

- Historical records may lack complete timing bounds. Preserve last/current independently when safe; accumulated time remains unavailable until coverage is established.
- A private checkpoint can be malformed, stale or copied across sessions. Fail closed and replay from the validated source; never use it to refresh source time.
- Two viewport regions complicate keyboard reveal and tooltip placement. Production QML fixtures must exercise scrolling and short-height layouts.
- Checkpoint progress improves warm restart but cold replay remains bounded over multiple polls. Connecting is distinct from a failed source and incomplete metrics remain explicit.

## Migration Plan

No remote change is needed. Existing installations retain stable plugin ID and private account/preferences files. Add only explicitly owned component/checkpoint files to install/uninstall lists. Root installs reviewed source locally, verifies one owner runtime and no listener, checks warm restart and current local/remote readings, then authorises canonical specification sync and archive.

## Verification

Run focused timing/checkpoint/security and state-projection tests; full Python suite with socket permissions; production Qt tests and synthetic visual captures; plugin/installer/uninstaller checks; strict OpenSpec validation. Independent exact-source review precedes root-owned installation and live acceptance. Archive only after that acceptance.

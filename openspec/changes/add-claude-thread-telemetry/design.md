# Design

## Context

This change implements decisions D1 to D9 of the archived research change
`2026-10-01-research-claude-code-parity`, whose design.md holds the evidence:
transcript layout, usage records, turn and child records, and the review
history. They are restated below as this change's authoritative design, with
refinements marked. The synthetic fixtures and implementation review of this
change take precedence where they differ from the research.

Existing Codex behaviour, output and checkpoints must not change.

## Goals / Non-Goals

**Goals:**

- Claude Code threads, local and peer, show deduplicated token totals, last-response usage, context occupancy, model, compactions, child outcomes and turn timing, under the AGENTS.md native data contracts.
- Fail closed, to unknown, on every ambiguous, unsafe or unrecognised input.

**Non-Goals:**

- The context window and percentage. Change 3 adds a reporter.
- Allowances and identity. That is change 4.
- Peer protocol changes.
- Reading subagent transcripts.

## Architecture

- **`src/claude.rs` (new):**
  - discovery (D1), identity (D2) and replay (D3, D5, D6, D7) for one session file;
  - the bounded oversized-record classifier;
  - `ClaudeCursor` (D8);
  - the projection into the existing telemetry and turn-timing values.
- **`src/native.rs`:**
  - `NativeTelemetry::enrich` dispatches on the agent;
  - Codex keys stay `anton-native-session-v1:<id>`; Claude keys are `anton-native-session-v1:claude:<id>`, hashed;
  - `Cursor` gains `claude: Option<ClaudeCursor>`, omitted when absent, so Codex rows serialise unchanged;
  - `Cursor::validate` accepts the child status `unknown` only on rows with a Claude block, and requires a valid block on Claude rows;
  - local retention, rediscovery of positive bindings, and the all-null caught-up sample.
- **`src/turns.rs`:** narrow public helpers so `claude.rs` can start, finish and abort intervals with the same invariants. The `Turns` schema is unchanged.
- **`src/telemetry.rs`:** `claude-transcript` in the `usage_source` allowlist.
- **`src/main.rs`:**
  - `State::sample` re-runs `telemetry_view` and `turn_timing_view` on peer agents;
  - it keeps local retention of peer Claude samples (D3).
- **Tests:**
  - Rust unit fixtures in `claude.rs` and `native.rs`;
  - process fixtures in `tests/native_process.rs` (peer probe, old and new cursor round trip);
  - additive metrics in `tests/measure_anton_popover.mjs` and an opt-in Claude home in `tests/bench_anton_native.mjs` (`--claude-agents N`, default 0, so a default run stays comparable with earlier default runs);
  - a State.js case and a shell-harness thread for a Claude agent with telemetry.
- **Corpus check:** `omarchy/anton-runtime/examples/claude_corpus.rs`, built only by `cargo run --example` and never shipped. It replays a local projects root through `claude.rs` and prints aggregate counts only.

## Decisions

### D1. Binding: Herdr session id to exact file, verified by content

**Acceptance.** Accept a Claude pane only when Herdr reports `agent == "claude"` and `agent_session` has `source == "herdr:claude"`, `kind == "id"` and a value that passes `safe_id(value, 128)`.

**Lookup.**
- Look for `<root>/<entry>/<id>.jsonl` at depth exactly two, within the Codex discovery entry and time budgets.
- Require exactly one match. Zero or several matches leave telemetry unknown, and so does a scan truncated by the entry or the 100 ms discovery budget (`native.rs:651-670`). A discovery or predecessor scan cut short because the shared probe deadline passed is a deadline skip: it changes nothing, publishes nothing, and keeps the cursor row and the retained sample.
- **Predecessor after `/clear`.** If Herdr reports an id whose file has ended because a successor exists, the binding would show a stale session. Change 2 first settles from the binary which id SessionStart(clear) delivers. Until that is proven, the fallback is fail-closed. Within the discovery budget, files in the same directory that are newer than the bound file's last record are scanned to their first record carrying `session_id`, bounded at 256 KiB and 512 records. In observed successors it first appears at 0-based record 16 to 19, in a record ending 69 to 76 KiB into the file. There are four cases, with two results:
  1. End of file within the bound with no `session_id`: not a successor.
  2. Bound exhausted first: unknown (fail closed).
  3. First `session_id` equals the bound id: unknown.
  4. First `session_id` is any other id: not a successor.

  A scan cut short by the entry or time budget, or by an IO error, is unknown and is not cached, like a truncated discovery. Exceeding the 256 KiB or 512-record bound, an unparseable record or an unsafe file is unknown and is cached.

  A negative result is not cached while the candidate is still growing. The fixture's first `session_id` record starts after 16 records and more than 64 KiB. Change 2 adds a synthetic fixture.
- Positive bindings are re-scanned every 60 seconds too, unlike today's cache, which re-discovers only missing paths (`native.rs:698-711`). A second match reverts the session to unknown.
- Never derive the slug, and never use `cwd` or pids.

**Root.**
- The root is `$CLAUDE_CONFIG_DIR/projects` from the collector's own environment, otherwise `~/.claude/projects`, exactly as Codex uses `CODEX_HOME`. Peers use their own environment. (Refinement: the research's optional `claude_projects_root` config key is dropped for Codex parity; it can be added later without a protocol change.)
- The collector never reads another process's environment. A mismatched config directory leaves the thread unknown.

**Rejected alternatives.**
- **pid or cwd to newest file.** Sibling sessions share slug directories, cwd drifts, `/clear` races the old file, and nothing proves identity.
- **A binding file written by a hook.** It duplicates Herdr's id.

### D2. Identity and confinement

- Open with `common::open_owned`: per-component no-follow, regular file, owner uid. A symlink anywhere fails closed.
- The header is the first line: at most 64 KiB, newline-terminated, with `sessionId == id`. Any first-record type is tolerated. A pass that starts fresh applies the header as the first replayed record, with its usage, turn, child and timestamp fields; a resumed pass never applies it again.
- Any later record that carries a `sessionId` different from the bound id makes the session's telemetry unknown for the rest of the binding (the block's `foreign` flag): later records feed nothing, no turn timing is published, the retained sample is dropped, the all-null sample is published even when the pass does not catch up (so a peer's local never re-emits the old sample), and only a fresh replay clears it.
- Fork and branch paths copy the parent's records into the new file and rewrite `sessionId`, adding `forkedFrom` [bin]. A record carrying `forkedFrom` is inherited history and feeds no total, last-response value, turn, child or compaction. Only its presence is read, never the nested id. Change 2 adds a synthetic fork fixture. So a response is never counted in both sessions.
- Records without `sessionId` (`file-history-snapshot`, `file-history-delta`) are ignored for identity and feed no metric.
- The snake-case `session_id` is not an identity check for the bound file, because it differs from the stem in 5 of 18 files (4 name a predecessor after `/clear`). It is used only by the D1 predecessor check.
- Thread replay never reads `<id>/subagents/**`, `tool-results/**`, `memory/**` or `~/.claude.json`.

### D3. Deduplicated usage replay

**Groups.**
- A group is the run of assistant records sharing one `message.id`. Only an assistant record with a different `message.id` closes the open group. User, attachment, system, queue and other records never close or reset it, and skipped `<synthetic>` lines are neutral. (Tool results routinely sit between lines of one response: 142 interleavings across 8 of 18 files, 0 reopenings among assistant records.)
- The cursor keeps the open group's id hash and its counted contribution. A later line of the same group replaces that contribution; it is never added. This holds across replay passes.
- The cursor also keeps a ring of the last 32 closed group ids. Each id, like the open group's, is the first 16 hex characters (64 bits) of sha256(`message.id`). An assistant record whose id matches one of them makes totals unknown. Main files never did this. Subagent files did, which is one reason they stay out of totals.

**Counted groups.**
- `<synthetic>` records are skipped.
- A group whose last line has `stop_reason: null` (aborted) still counts towards totals, because the request was billed. It is never the last-response or context source.
- A missing or non-numeric counter in a counted group makes totals unknown. It never becomes zero. So does a group whose four counters sum above 2^53 or whose response fails validation; the row stays resumable.
- Each condition that makes totals unknown also nulls the last-response values, `context` and `model` (the block's `last_valid` flag) until the next complete counted group. These conditions are a ring match, an unclassifiable assistant record, an unparseable line and a missing counter and a four-counter sum above 2^53. (A D2 mismatch is permanent for the binding instead; see D2.)
- Totals use the top-level usage, which equals the sum of the `message` iterations. Advisor iterations are excluded, matching Claude Code's own accounting.

**Replay and coverage.**
- Totals come from the replay cursor, not a tail read. Codex reads provider-cumulative totals from a 512 KiB tail (`native.rs:504-629`), but Claude has no cumulative record.
- The whole Claude numeric sample is published only when the cursor has `caught_up && !skipping`. That sample is the totals, last-response values, `context`, `model` and `usage_seq`. This follows the gating Codex applies to turns, children and compactions (`native.rs:764, 783, 815`).
- Each pass reads at most `TAIL` bytes and resumes from the checkpoint, so a cold multi-megabyte transcript takes several passes before values are known.
- `NativeTelemetry` keeps the last published Claude sample in memory per key. A pass that is not caught up re-emits it unchanged, with its original `usage_seq`. Retention covers only an incomplete replay of a bound, identity-checked file. Any of these drops it:
  - a binding change;
  - file replacement (dev/inode, header or tail mismatch);
  - zero, several or truncated discovery;
  - an open, ownership, header or D2 identity failure;
  - a positive or truncated D1 predecessor result;
  - a later record naming another session (D2);
  - a peer sample that revalidation rejects.

  Growth does not drop it. A pane skipped by the shared deadline re-emits the retained sample when its cached binding is current (under 60 s old and not due for rescan) and its cursor row is present; replacement of the file is then detected at the next pass that runs. This is the retention the spec requires for an intermittent read on local hosts. A deadline that expires inside discovery or the predecessor scan is handled as the same skip.
- Peers start a fresh `NativeTelemetry` on every probe (`main.rs:1026`), so peer threads are retained locally. `State::sample` keeps the last validated Claude telemetry per agent id and `session_generation`, filled only from live peer samples in this owner run. Every caught-up Claude pass publishes a sample, with `seq = coverage_seq`, `event:"session"` and `phase:"ready"`, even when every value is null. A peer that cannot bind or verify the file publishes an all-null Claude sample stamped with the incoming cursor row's `coverage_seq`, which is an original source time. The causes are the drop triggers listed for local retention, including a record naming another session in a pass that does not catch up. Its arrival replaces the retained sample. A caught-up peer sample is stored only when the response has a Claude cursor row for every bound Claude pane on that host. The retained sample is re-emitted only when the peer agent's `technical.telemetry` is absent and both the request and the response had such a row for every bound pane, because without the request row the peer replays from the header and cannot detect a replaced file; otherwise it is dropped. The counts are per host, so a new pane without a row drops the retained samples of that host's other panes for that probe. It is dropped when the generation changes or the pane disappears. It retains the same numeric subset as the local rule (totals, last-response values, `context`, `model`, `usage_seq`), with child fields null. A caught-up peer sample with null totals replaces it, as on a local host. It is never stored in the checkpoint, because a loaded checkpoint is never a current measurement. Change 2 adds peer fixtures for three passes: one that does not catch up, a caught-up pass with invalid totals, and a caught-up pass where everything is unknown, and a bound session whose file becomes ambiguous after one caught-up sample.
- A line of 64 KiB or less that serde rejects goes through the oversized-record classifier, so a record is classified the same way at any line size. `invalid()` is called only when the classifier also rejects it, or when the parsed value is not an object; for Claude rows it also clears `totals_valid`. A pass also ends at an empty read, when the file shrank after the pass took its length.

**`usage_seq`.**
- `usage_seq` is the largest validated timestamp among the counted lines of all counted groups, in microseconds.
- It is non-decreasing despite non-monotonic file order, so the `usage_seq >= previous` merge (`native.rs:772-779`) never rejects newer totals.

**Oversized lines.**
- An oversized line (over 64 KiB) goes through a new bounded Claude envelope classifier. The Codex `Envelope` (`envelope.rs`) cannot extract these fields.
- The classifier extracts only the fields D3, D5, D6 and D7 consume: `type`, `subtype`, `sessionId`, `uuid`, `timestamp`, `isMeta`, `origin.kind`, `commandMode`, `message.id`, `stop_reason`, `model`, `usage`, `toolUseResult.{status, agentId, resumedAgentId, success, totalDurationMs}`, `interruptedMessageId`, `isAbortedMidStream`, the presence of `forkedFrom` and `isCompactSummary`, the bounded `task-id` and `status` tags, the leading wrapper tag of user text, and `operation` (queue-operation records).
- `forkedFrom`, `isCompactSummary`, `origin`, `toolUseResult` and `interruptedMessageId` are presence-only: a string value is recorded at its first byte, never buffered, and survives a pass boundary.
- A lost `type` or `sessionId` (a value over the capture bound, or cut by a pass boundary) makes the record invalid for every kind, so identity is never taken as absent.
- An unpaired `\uD800`–`\uDFFF` escape reads as one replacement character, and a surrogate pair as one character, on both paths.
- Persisted classifier state keeps the raw bytes of a key only while they remain a prefix, written plainly or with `\uXXXX` escapes, of a consumed key at that parent; otherwise the bytes are dropped and the key is marked non-matching.
- Its state persists across pass boundaries in the `claude` block, because lines larger than `TAIL` exist.
- An oversized record of a relevant type that cannot be classified makes the dependent coverage unknown:

  | Record type | Coverage made unknown |
  |---|---|
  | assistant | totals, compactions, turns and `last_valid` |
  | system | compactions and turns |
  | user or attachment | children and turns |

  Large `prompt_snapshot` attachments and tool results are common (10 of 18 files) and are classified, not dropped.

### D4. Context occupancy

- `context` is the occupancy of the last counted group with a non-null `stop_reason`, by Claude Code's rule: `input + cache_creation + cache_read` of the selected iteration or the top level.
- The last-response fields use that same selected usage object, so last-response input never exceeds `context`. Top-level usage feeds only the totals.
- In change 2, the replay object omits `window` and `context_percent` rather than writing nulls, because the merge at `native.rs:770-780` lets replay keys replace metadata keys. Change 3 supplies them from the reporter (D10) and owns the merge order.

**Rejected sources for the window.**
- **A model-to-window table.** A 1M-context session is indistinguishable in transcripts, and Claude Code's own rule depends on beta headers and environment overrides.
- **`~/.claude.json` `lastModelUsage` `[1m]` keys.** They are per project rather than per session, are written outside the session, and sit inside the D11-gated file.

### D5. Compactions

- Count `system/compact_boundary` records, excluding `microcompact_boundary`. Only count with complete coverage from the header and a known `usage_seq`, as for Codex.
- If any counted group has a usage iteration of type `compaction`, `compactions` becomes unknown. That path has no boundary record, and its meaning is unverified.

An assistant record that cannot be classified makes `compactions` unknown, because its usage may carry a compaction iteration.

### D6. Children and completion

**Source and keys.**
- Children come from the main file only.
- Associations use the existing row-level `children` map and `valid` flag, keyed by `sha256(agentId)` and capped at 128, as for Codex.
- `Cursor::validate` additionally accepts the status `unknown` on rows with a Claude block; a Codex row carrying it is rejected and replayed fresh. An older binary rejects such a row and replays it fresh, which is the same cost as D9.

**Records.**
- **Launch.** A `toolUseResult` with `status: "async_launched"` and an `agentId` starts a child as running. A synchronous Agent result with `agentId` and `totalDurationMs` records it as completed.
- **Launch without `agentId`.** An `async_launched` result with no `agentId` is a workflow or teammate launch, and is ignored.
- **Resume.** A `toolUseResult` with `resumedAgentId` naming a known child and `success: true` returns that child to running. This is the Claude form of "resumed work invalidates old completion". A repeat launch of a known child does the same. A resume naming a child that was not launched in this file is ignored (observed 2 of 5 times; the child belongs to another session).
- **Completion.** Completion is a user record whose `origin.kind` is `task-notification`, or a `queued_command` attachment with `commandMode: "task-notification"`.
  - A bounded tag grammar extracts only `task-id` and `status`. Nothing else in the block is read.
  - A notification whose task id matches no known Agent child is ignored. These come from shell tasks, workflows and teammates.

**Status mapping.**

| status | outcome |
|---|---|
| `completed` | completed |
| `failed` | failed (the `errored` bucket) |
| `killed` | interrupted |
| `blocked` or any unrecognised status | `subagent_unknown` |

`subagent_unknown` keeps the partition summing to `subagent_total`.

**Invalidation.** A recognised record with a malformed `agentId`, or exceeding the cap, sets `valid = false`, as for Codex.

**`subagent_status_seq`.** For Claude rows, row-level `seq` starts at 0, overriding its Codex meaning (D8). It becomes the largest validated timestamp among accepted launch, resume and notification records, in microseconds, and is never assigned directly, because file order is not time order. While no child record has been accepted, the published stamp is the block's `coverage_seq` (D8), never the replay wall time. A thread with no children therefore shows 0 children with an honest source time.

**AGENTS.md amendment.** Change 2 amends AGENTS.md so that "typed native child lifecycle evidence" covers these structured Claude records. A start/stop hook ratio remains forbidden.

### D7. Turn timing

**Classification.** Records are classified in this order of precedence:

1. **Abort:** the interrupt marker text, a record carrying `interruptedMessageId`, or an assistant record with `isAbortedMidStream`.
2. **Turn trigger by origin:** a user record whose `origin.kind` is `human`, `task-notification`, `peer` or `coordinator`, regardless of `isMeta`; or a `queued_command` attachment with `commandMode: "task-notification"`.
3. **Ignored:** `isMeta`, `toolUseResult`, `isCompactSummary`, `queued_command` attachments with `commandMode: "prompt"` (queued human input, observed only inside turns), local-command output, bash-mode records, and `system/compact_boundary` and `microcompact_boundary`. These never start or end turns.
4. **Turn trigger by shape:** a remaining user text record without `origin`, or a slash-command echo. A user record with no `origin`, no rule-3 flag and no text (for example an image-only prompt) is an unrecognised shape and takes the unknown path.

Wrapper tags are matched on the leading tag of user text:

| Role | Tags |
|---|---|
| Slash-command echo (rule 4) | `command-name` |
| Local-command output (rule 3) | `local-command-stdout`, `local-command-stderr`, and `system` records with subtype `local_command` |
| Bash mode (rule 3) | `bash-input`, `bash-stdout`, `bash-stderr` |

`local-command-caveat` is ignored through `isMeta`. Any other leading tag falls to rule 4. Local-command output while a start is pending clears the pending start and sets `lost_idle`, because a custom command can run the model. Input taken (`dequeue` or `remove`) after a record lost while idle, a second record lost while idle, or a lost queue record makes the turn ambiguous. Change 2 adds a synthetic fixture for each tag.

**Turn triggers.**
- With no active turn, a trigger opens a pending start. A trigger that would replace an unconfirmed pending start is ambiguous instead, because the earlier prompt may have been killed or joined. It becomes the turn start only when an assistant record follows before the next trigger, including a `<synthetic>` error record, which confirms a start although D3 ignores its usage. Otherwise the newer trigger replaces it.
- The first classified trigger or turn end sets `Turns.supported`, so `complete` can be published.
- With an active turn, a trigger joins that turn only when a `queue-operation` record with operation `dequeue` or `remove` has appeared since the turn's trigger or the last join, while that turn was active or its start was pending [obs shape, inf semantics]. An `enqueue` alone never permits a join, so a killed turn whose queued input was never taken cannot absorb the idle gap. Every trigger consumes the evidence. With no active turn a trigger never joins; it replaces any pending start (joining a pending start added idle time in corpus replay). The joining trigger must also be stamped no later than that evidence, so input queued before a kill can never join a turn started after the restart.
- Without the evidence, accumulated coverage becomes unknown. The same holds at every point where a turn may still be running: an unjoined trigger during a turn, an unrecognised origin, a trigger with no stamp or key, a start that `Turns::begin` rejects (before the previous end, or a repeated key), and an unparseable, foreign or unclassifiable record while a turn is active or a start is pending. Because the input may instead have joined a still-running turn, no pending start opens and the current and last values stay unchanged (`current_known` false) until `system/turn_duration` or an abort proves an end; the next trigger after that opens a pending start normally. Silent ends do not clear this state; an abort clears it even when the start it confirms is rejected. A record lost while no turn is running (unparseable, unclassifiable, or a user record without `sessionId`) may itself have opened a turn: an assistant record that follows before any trigger makes coverage ambiguous until `turn_duration` or an abort; a trigger before that assistant record replaces the lost start as usual.
- An abort while a start is pending confirms that start and ends the turn as aborted at the abort timestamp. An `isAbortedMidStream` assistant record confirms a pending start before it is handled as an abort.
- An abort with no active turn and no pending start is ignored, apart from setting the abort-adjacency flag. The next trigger, assistant record or `turn_duration` clears that flag.
- The turn key is `sha256("anton-turn-v1:" + session + ":" + uuid)[..24]` of the record that opened the turn, which satisfies `Turns::validate`.

**Publication.** `native.rs` publishes Claude turn timing through `Row::published_turns()`, a masked copy; `Turns` and the block are unchanged. While a start is pending, or after local-command output cleared one (`lost_idle`), the published current turn is unknown. While `silent_end` is set, the current turn, accumulated total and `complete` are unknown.

**Ending a turn.**
- `system/turn_duration` completes the active turn at its own timestamp.
- A silent end clears `queued_since_start`. A silent end is `system/stop_hook_summary`, or an assistant line with `stop_reason: end_turn` and no pending tool use, while a turn is active. A later trigger with no `turn_duration` then takes the unknown path rather than joining the stale turn and absorbing the idle gap. In a reviewer's replay, the older-version file becomes honestly unknown, and the largest remaining joined gap fell from 66,701 s to 764 s. Change 2 adds a fixture for a silent end followed by queued input. Until a trigger, `turn_duration`, an abort or unknown coverage, a later `dequeue` or `remove` is no join evidence. While `silent_end` is set, the published current turn, accumulated total and `complete` are unknown; a later `turn_duration` restores them.
- An abort ends the active turn as aborted at the abort record's timestamp.
- A `turn_duration` with no active turn is ignored when it directly follows an abort. Otherwise it makes accumulated coverage unknown.

**Timestamps.**
- Timestamps convert to Unix seconds by floor, for both bounds. A trigger or queue operation whose timestamp floors to Unix second 0 is treated as missing its timestamp.
- A start earlier than the previous end, or an end earlier than its start, makes accumulated coverage unknown through `Turns::unknown`.
- The current or last valid interval stays available, as the spec allows.

**Durations.** `turn_duration.durationMs` is never substituted. It is used only as a test cross-check with an explicit tolerance, because the spec includes permission waits.

**State.** The finished intervals and total stay in the unchanged row-level `Turns` struct; a row shrunk at the byte bound keeps only its last finished interval (D8). Pending-start state lives in the `claude` block (D8), so `Turns` keeps its schema for older binaries.

### D8. Checkpoints

**Rows and keys.**
- Claude rows share the checkpoint file and its 32-session, 256 KiB and 24-hour bounds.
- They inherit the meaningful-progress throttle and installed-owner retirement unchanged.
- Keys are `sha256("anton-native-session-v1:claude:" + id)`, so no raw id is stored. The Codex key (`native.rs:693`) has no harness.
- Offsets are bytes, never timestamps.
- Codex rows are charged against the 32-row and 256 KiB bounds before Claude rows, in cursor validation and checkpoint eviction, so Claude rows never displace Codex rows; checkpoint eviction removes the oldest Claude row first.
- Checkpoint rows load individually, so a row this build cannot read never discards the others.
- A Claude row that would exceed the 256 KiB bound is first shrunk: `turns.finished` keeps only the last interval, `total` matches it, and accumulated coverage becomes unknown for the rest of the binding. Offset, file, fingerprint, block and children are kept, so replay resumes. The sample measured before the shrink still publishes complete coverage. Rows are taken in key order; a row that still does not fit is dropped with every row after it. Checkpoint eviction across hosts still removes whole rows.
- Each holder releases the checkpoint lock with `LOCK_UN` before closing it (`common::Unlock`), because a process spawned by another thread holds the lock's open file description until it calls exec. The lease and each write still wait up to 250 ms for genuine contention. The allowances receive lock and the reporter hook lock release the same way.

**Row-level fields.**
- `Cursor::children`, `valid`, `compactions_valid`, `turns`, `fingerprint`, `offset`, `file`, `at`, `caught_up` and `skipping` keep their existing meaning for Claude rows. `seq` follows D6.
- Row-level `envelope` stays `None`, and `compaction_markers` and `compaction_summaries` stay 0. Claude classifier state and the boundary count live only in the block. `turns` and `fingerprint` remain required, so checkpoint v1 is kept.

**The `claude` block.** All other Claude parser state lives in one required `claude: ClaudeCursor` block with `deny_unknown_fields`. `Cursor` itself does not deny unknown fields (`native.rs:88-110`), so an older binary would silently drop loose fields. A Claude-keyed row whose block is absent or invalid is discarded and replayed fresh, never resumed with zeroed sums.

Every block field is required, bounded and revalidated on reuse:

- the four cumulative sums (`input`, `output`, `cache_read`, `cache_creation`) as `u64` within 2^53, plus `totals_valid`;
- the last-response partition, `context` and `last_valid`;
- `model`: at most 64 characters, using the `telemetry_view_at` character allowlist;
- `usage_seq`;
- `coverage_seq`: the largest validated record timestamp replayed, in microseconds, at most 2^53 and at most now plus the 1 s skew;
- the open group's id hash, counted contribution and stop state;
- the ring of at most 32 closed group ids of 16 hex characters;
- the compaction boundary count and the compaction-iteration flag;
- the pending turn start: key hash and Unix second, or none;
- the abort-adjacency flag;
- `queued_since_start` (the evidence stamp, `Option<u64>`): set, from a stamp of at least 1 s, by a `dequeue` or `remove` queue-operation while a turn is active or a start is pending and no silent end has been seen; kept when the pending start is confirmed; consumed by every trigger; reset when a turn ends, on a silent end (D7) and when turn coverage becomes unknown; false unless a turn is active or a start is pending;
- `ambiguous`: set at every point where a turn may still be running (D7); while set there is no active turn or pending start and accumulated coverage is unknown;
- `silent_end`: set by a silent end (D7) while a turn is active; reset by a trigger, `turn_duration`, an abort and unknown turn coverage; only set while a turn is active and `queued_since_start` is false;
- `lost_idle`: set when a record is lost with no active turn, no pending start and no ambiguity; cleared by a pending start, `turn_duration`, an abort or ambiguity; only set while idle and not ambiguous;
- `foreign`: set by a record whose `sessionId` differs, never cleared on resume; requires invalid totals and last-response, row `valid`, `compactions_valid` and turn coverage false, and `current_known` false;
- the Claude envelope classifier state (D3), or none.

**Rediscovery.** D1 re-scans positive bindings too (D1 "Lookup").

**Resume and replacement.**
- A resume that appends to the same file keeps the binding. Dev/inode, header hash, tail hash and size or mtime still detect replacement.
- A resume that writes a new file elsewhere produces two matches at the next re-scan, so the session becomes unknown.

**Refinements after review round 4.**
- **D1 predecessor scan.**
  - A candidate that cannot be opened is an IO error, which is unknown and not cached, unless the candidate is a symlink, not a regular file, or owned by another user.
  - A repeated `session_id` is unknown and is cached.
  - A candidate line that serde rejects, or that repeats a key, is read only for its top-level `session_id`, once the classifier accepts it as well formed.
- **D2 header.**
  - A header line that serde rejects, or that repeats a key, is verified through the classifier, which must find `sessionId == id`. A repeated `sessionId` never matches.
  - A fresh pass applies such a header through the classifier.
- **D3 short lines.** A line of 64 KiB or less whose objects repeat a key goes through the classifier, so a repeated consumed key is lost at any line size.
- **D3 peers.**
  - When replay cannot open, verify or read the bound file, the incoming cursor row is kept unchanged and the all-null sample is published at its `coverage_seq`. Progress from an earlier pass of that probe is discarded.
  - Retention counts as bound only the Claude agents with a session generation among the first 32 agents of the response, which is the window a peer enriches. Panes sharing a generation count once.
  - The local cannot see the session kind or whether the id is safe, so a Claude pane of kind `path` or with an unsafe id is still counted. This fails closed.

### D9. Peers and compatibility

**Fields and envelope.**
- Claude fills only existing `Telemetry` and `TurnTiming` fields and adds `claude-transcript` to the `usage_source` allowlist (`telemetry.rs:179-183`). Without that, `telemetry_view_at` would null the source locally.
- The v1 envelope and the probe command are unchanged.

**Mixed versions.**

| Peer | Local | Result |
|---|---|---|
| old | new | Remote Claude threads stay status-only. |
| new | old | Values arrive in existing fields. The old local re-serialises cursors through its own `Cursor` type, which drops the `claude` block, so the new peer replays every Claude row from the header on each probe: up to 16 × `TAIL` per agent within the shared 750 ms probe deadline. Codex panes are enriched before Claude panes within that deadline, so Codex values are never lost to Claude replay; Claude threads whose combined replay exceeds it stay unknown. Measured with four 6 MB transcripts on one peer: every peer thread reached native telemetry in 150 ms, within the deadline, at 0.866 s CPU over 30 s against 0.550 s with a new local. The local must be upgraded before remote Claude values are relied on. |

**Redeployment.** SSH peer redeployment is therefore optional.

**Peer revalidation.** Change 2 also re-runs `telemetry_view` and `turn_timing_view` on peer samples, against the later of local now and `sampled_at`, plus the 1 s skew `sampled_at` allows, plus the 6 s Herdr snapshot timeout that bounds how long after `sampled_at` the peer stamps its values, for every harness, keeping original timestamps, in `State::sample` (`main.rs:191-281`, called from `accept_host`), as `harness-telemetry` already requires.

**Spec and AGENTS.md wording.** Change 2 must make the following harness-neutral, keeping Codex behaviour unchanged:

- `harness-telemetry` "Hook-time structured usage enrichment" (the native Codex reader);
- "Plugin-owned native enrichment" (Codex records; Codex collection without hooks);
- "Scoped cumulative harness metrics" (the native Codex sample retention sentence and the "Intermittent Codex usage read" scenario; the context sentence is assigned to change 3);
- "Bounded native outcome summaries";
- "Native turn wall-clock summaries" (Codex event names);
- "Private native replay checkpoints" (the Claude block and key, explicitly permitting the bounded allowlisted `model` string);
- AGENTS.md lines 18, 46 and 57.


## Fixture inventory

**Fixture inventory.** This change builds a named synthetic fixture for each case found during research and review:

- **Usage groups:**
  - identical-split groups (main) and streaming-partial groups (subagent style);
  - groups interleaved with user and attachment records;
  - a reopened closed group;
  - advisor iterations, a `compaction` iteration, `<synthetic>` records, and an aborted group (`stop_reason: null`);
  - a missing counter, and an unparseable line.
- **Classifier:** an oversized line that crosses `TAIL`, plus oversized assistant, user and attachment records.
- **Identity:**
  - a header mismatch;
  - a record whose `sessionId` differs;
  - records without `sessionId`;
  - a fork via `forkedFrom`;
  - the post-`/clear` predecessor, with the first `session_id` after more than 16 records and more than 64 KiB.
- **Discovery:** zero, several or truncated matches; a symlinked root; a second match appearing after binding; resume in place and resume to a new file.
- **Compaction:** `compact_boundary`, `microcompact_boundary` and `isCompactSummary`.
- **Children:**
  - async and synchronous launches, and a launch without `agentId`;
  - a resume via `resumedAgentId`, including for an unknown child;
  - notifications through `origin`, through `commandMode`, and without `origin`;
  - the statuses `completed`, `failed`, `killed`, `blocked` and an unknown status;
  - exceeding the cap.
- **Turns:**
  - human, task-notification, peer (`isMeta`), coordinator and command-echo triggers;
  - a synthetic record confirming a pending start, and an abort during a pending start;
  - an orphan abort;
  - queued input with and without a queue operation;
  - a silent end (stop-hook summary or `end_turn`) followed by queued input;
  - an orphan `turn_duration`;
  - non-monotonic and overlapping timestamps;
  - each wrapper tag.
- **Checkpoints:** a warm restart with the `claude` block, a missing or invalid block, an old binary round trip, and the 32-row bound.
- **Peers:**
  - a pass that does not catch up;
  - a caught-up pass with invalid totals, and one where everything is unknown;
  - a binding that becomes ambiguous after one caught-up sample;
  - no cursor row;
  - an old local with a new peer, and cost with several large transcripts.

**Added after review round 1:**

- **Usage:** a header record with usage, and a prompt header whose first turn is aborted.
- **Classifier:** a non-schema key under a consumed parent cut by a pass boundary, and an escaped schema key cut inside its escape.
- **Turns:**
  - an enqueue-only prompt after a killed turn;
  - a dequeue or remove before the first assistant line;
  - a dequeue after an unconfirmed command echo;
  - a second prompt after one join;
  - an unknown operation;
  - a trigger at Unix second 0;
  - an unjoined trigger during an active turn, followed by `turn_duration`, an abort and a later trigger.
- **Checkpoints:**
  - a full ring within the block size bound;
  - Codex rows charged before Claude rows;
  - one unreadable row among readable ones;
  - the lock under concurrent process spawns.
- **Peers and local:**
  - a request without the row followed by a peer restart that does not catch up;
  - a deadline skip of a current binding;
  - a truncated predecessor scan.

**Added after review round 2:**

- **Lost turns:** cases A to H:
  - an unknown origin;
  - a non-JSON line;
  - triggers without a stamp;
  - a rejected start, with and without dequeue;
  - a repeated uuid;
  - a pass boundary larger than `TAIL`.
- **Invariants:** a fuzz test of replay-state invariants.
- **Identity:** a mismatch followed by later groups.
- **Classifier:** presence-only strings over 1 KiB.
- **Usage:** a four-counter sum above 2^53.
- **Peers:**
  - peer skew at +0.5 s, kept;
  - peer skew at +2 s, rejected, which drops retention.
- **Checkpoints:** seven maximal Claude rows shrunk at the byte bound.
- **Test reliability:** executable fixtures under concurrent spawns.

**Added after review round 3:**

- **Queue and turn edge cases:**
  - a queue operation after a silent end;
  - a lost trigger while idle;
  - an image-only prompt.
- **Identity and classifier:**
  - a lost `sessionId` in oversized records of every kind;
  - unpaired surrogates at both line sizes.
- **Replay and timing:**
  - a file truncated during an oversized skip;
  - a deadline expiring inside rediscovery;
  - Claude replay listed before a Codex pane under a short deadline.
- **Peers:**
  - a foreign record in an incomplete peer pass;
  - peer skew at +1.05 s with `sampled_at` +0.95 s;
  - values just inside and just past the revalidation bound.
- **Locks:** each lock is free at once after release while siblings spawn.

**Ground-truth turn fuzzer.** `0ac8792` generates sessions whose true turn intervals are known, using the D7 model. It injects every shape above, plus kills, lost records, reordering, queues, silent ends, foreign records and byte-level pass boundaries with oversized lines.

After every record, and after every caught-up pass in file mode, it asserts (strict oracle, `950f360`) that each published current start and accumulated total is exactly the truth or unknown, and the last interval is the truth, unknown, or an unchanged earlier true interval while accumulated coverage is unknown (the spec allows the last valid interval to remain). An unchanged value is otherwise allowed only across a kill, which writes no record; both allowances are counted. The local-command and second-idle-loss shapes are guarded by their unit fixtures, not by the fuzzer.

Two model calibrations are explicit in the test:
- queued input is stamped when it was queued;
- a killed process writes nothing for at least 2 s.

**Added after review round 4:**
- **Turn timing:**
  - an epoch-stamped dequeue or remove, in memory and across a pass boundary;
  - published masking for a pending start, `lost_idle` and a silent end;
  - the strict oracle.
- **Compactions:** an unclassified assistant record with a compaction iteration. Variants: no timestamp, no `sessionId`, no message id, and an oversized line cut inside the type.
- **Peers and retention:**
  - a header or identity failure with an incoming row;
  - a Claude pane beyond agent 32;
  - two panes on one session.
- **Predecessor scan and header:**
  - an unreadable predecessor candidate;
  - a lone-surrogate or repeated-`session_id` candidate;
  - a lone-surrogate header;
  - repeated keys at both line sizes.
- **Fixtures and corpus:**
  - the State.js and shell Claude fixtures use the native shape: null starts, stops and `subagent_seq`, so children is null, with completion present;
  - the corpus example prints a counts-only `turns.joins`.

**Corpus check.** As a verification step, this change also runs a local counts-only replay of the real transcript corpus through the implementation. It prints aggregates only, and nothing from it is committed. Prose review could not converge on these rules; replay can.


## Risks / Trade-offs

- **[The context dial stays unknown until change 3]** This is accepted and recorded in the proposal.
- **[Unobserved shapes]** Compaction, synchronous children and resume to a new file come from binary strings. Unrecognised variants become unknown, and fixtures cover each.
- **[Peer cost with an old local]** An old local forces full replay on every probe, bounded by the 750 ms deadline. It is measured with `node tests/measure_anton_popover.mjs --claude-old-local <old binary>` (see evidence.md). Remote Claude values need the local upgraded first.
- **[Format drift]** Validation fails closed. The corpus check is re-runnable after Claude Code upgrades.

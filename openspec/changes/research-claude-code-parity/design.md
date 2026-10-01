# Design

## Context

Evidence for this design was gathered on 2026-10-01 with Claude Code 2.1.285,
Herdr 0.9.1 and herdr-observatory `main` at `2d2be90`. evidence.md lists the
sources and the method. Labels:

- **[obs]**: observed on this host, recorded as structure and counts only.
- **[bin]**: strings in the installed Claude Code binary. Internal and liable to
  change between versions.
- **[doc]**: official Claude Code documentation or CLI help, not verified by
  observation.
- **[repo]**: a `file:line` in this repository at `2d2be90`.
- **[inf]**: inferred.

The local corpus is live, so counts drift. Each count belongs to the first survey
pass unless marked otherwise. Change 2 tests use synthetic fixtures built from
the shapes below.

### Transcript layout and identity

- **Root** [bin]: `$CLAUDE_CONFIG_DIR/projects`, otherwise `~/.claude/projects`.
- **Main session** [obs, 18 files]: `<root>/<slug>/<sessionId>.jsonl`.
- **Children** [obs]:
  - Agent-tool children are `<root>/<slug>/<sessionId>/subagents/agent-<agentId>.jsonl`, each with a `.meta.json`.
  - Workflow workers live under `subagents/workflows/wf_<id>/`.
  - Spilled tool output lives under `<sessionId>/tool-results/`.
- **Slug and cwd**: neither is usable for binding.
  - The slug comes from the starting cwd by a lossy, undocumented rule [bin].
  - Per-record `cwd` drifts within a file (6 of 18 files) [obs].
- **Header** [obs 18/18]: the first line of every main file is `{"type":"mode","mode":…,"sessionId":<id>}`.
  - Mode records also recur mid-file.
  - The header carries identity only, with no version or cwd.
- **Record identity** [obs 18/18, 0 mismatches]: every main-file record that carries `sessionId` equals the file stem. `file-history-snapshot` and `file-history-delta` records carry none (16 of 18 files contain them).
- **Parent and child separation** [obs]: `isSidechain` is false on every main-file record and true on every subagent record, so children are never interleaved into the parent.
- **Session ids across resume, fork and clear**:
  - `--resume` and `--continue` reuse the original session id; only `--fork-session` creates a new one [doc, `claude --help`].
  - Whether a resume appends to the same file is unverified [inf].
  - `/clear` and fork produce a new id [doc, Herdr hook matcher].
- **Timestamps** [obs]: ISO-8601 UTC with milliseconds on every record that has one. They are not monotonic in file order (23 negative gaps).
- **Sizes** [obs]:
  - Main file p50 721 KB, max 5.4 MB.
  - Line p50 1.6 KB, p99 42.7 KB, max 832 KB; 5 lines are over 256 KiB.
  - The last assistant line always started within 73 KB of end of file.

### Usage records

- **Assistant records** [obs]: carry `requestId` and `message.{id, model, stop_reason, usage}`.
  - `usage` always has `input_tokens`, `output_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens`.
  - Complete records add `iterations[]`, `output_tokens_details`, `server_tool_use` and `speed`.
- **Split responses** [obs]: one API response is written as one line per content block, sharing `message.id` and `requestId`.
  - Main files: all 740 multi-line groups were contiguous among assistant records, with identical usage. User and attachment records often sit between lines of one group.
  - Subagent files: earlier lines are streaming partials, and 6 ids reappeared after another id had started.
  - Every multi-line group's last line had the maximum `output_tokens`.
  - Summing every main-file line overcounts input 2.5× and output 3×.
- **Advisor iterations** [obs, second pass]: 51 deduplicated responses across main and subagent files had `message, advisor_message, message`. In every one, top-level usage equalled the sum of the `message` iterations.
- **Claude Code's own context function** [bin]:
  - It takes the last iteration that is neither `advisor_message` nor `compaction`.
  - It uses that iteration only when its type is `message` or `fallback_message` and its four counters are numeric and sum above zero.
  - Otherwise it falls back to top-level usage.
  - Occupancy is `input + cache_creation + cache_read`.
- **Client errors** [obs]: `message.model == "<synthetic>"` marks client-generated error lines with zero usage (6 lines).
- **No context window**: no transcript record carries a window size, and `[1m]` never appears in a transcript model string (0 of about 5,800) [obs].
  - Claude Code computes the window client-side from the model id, beta headers, a capability table and environment overrides [bin].
- **`cost-state` records** [obs]: hold cumulative `modelUsage`, but appear only 1 to 9 times per file and exceed the deduplicated main-plus-subagent replay by up to 30%. Their scope is not the session transcript.

### Other transcript surfaces

- **Compaction** [bin, not observed]: `{type:"system", subtype:"compact_boundary", compactMetadata:{trigger, preTokens, …}}`, followed by a user record with `isCompactSummary: true`.
  - `microcompact_boundary` is not a compaction.
  - A usage iteration of type `compaction` also exists. It suggests server-side compaction inside one response, with no boundary record [inf].
- **Prompts** [obs]: human prompts are user records with text content, no `toolUseResult` and no `isMeta`. `peer`-origin records carry `isMeta: true` (6 of 6) and still open real turns.
  - Newer prompts carry `origin.kind` (`human`, `task-notification`, `peer`, `coordinator`), `promptId` and `turnPosition`.
  - The same predicate also matches records without `origin` that are not model turns [obs, second pass]: slash-command echoes (19), local-command output (12), bash-mode records (2) and interrupt markers (3). 15 plain-text records without `origin` were followed by an assistant record.
- **Turn ends** [obs]: `system/turn_duration` records (103 in the first pass).
- **Aborts** [obs]: the user text `[Request interrupted by user]` (plus a "for tool use" variant [bin]), `interruptedMessageId`, or `isAbortedMidStream`.
- **Async Agent children** [obs]:
  - There were 23 parent `tool_result` records with `toolUseResult.status: "async_launched"`.
  - 17 carry `agentId`, each with a matching child file. 6 carry none: these are workflow or teammate launches.
- **Completion** [obs]: arrives as a user record with `origin.kind == "task-notification"`, or inside a `queued_command` attachment.
  - Attachments identify it by `commandMode: "task-notification"`, and 4 of 13 had no `origin`.
  - The content is a provider-generated tag block with `task-id`, `status`, `summary`, `result` and `usage`.
  - Observed statuses: `completed` and `failed`. [bin] adds `killed` and `blocked`.
  - 20 notification task ids matched a known Agent child. 32 did not; they belong to shell tasks, workflows or teammates.
- **Resume of a child** [obs]: a `SendMessage` result whose `toolUseResult` has `resumedAgentId` and `success` (5 of 8).
- **Synchronous Agent result** [bin, not observed]: `toolUseResult.{agentId, totalDurationMs, totalTokens, …}`.

### Non-transcript surfaces

- **statusLine** [bin, doc]: a subprocess spawned by Claude Code that inherits its environment.
  - Input JSON includes `session_id`, `transcript_path`, `model` and `context_window.{context_window_size, used_percentage, current_usage, total_input_tokens, total_output_tokens}`. Both totals are last-response values [bin].
  - For Pro and Max, after the first response, it also includes `rate_limits.{five_hour, seven_day}.{used_percentage, resets_at}`.
  - It runs on each render.
  - It is a single settings slot. Plugins cannot provide one: the plugin customisation table disables `statusLine` [bin]. The slot is occupied on this host [obs].
- **Hooks** [bin]: 33 events.
  - SessionStart carries `source` (`startup|resume|clear|compact|fork`), `model` and, on resume or fork, `context_tokens`.
  - PostModelSwitch carries `from_model`, `to_model` and `context_tokens`.
  - SubagentStop carries `agent_transcript_path`.
  - PostCompact carries `trigger`.
  - **No hook payload carries a context window size or rate limits.**
  - Hooks can come from settings files or plugins.
- **Current hook ownership** [obs, repo]:
  - **Herdr integration**: Herdr's integration (`herdr integration status`) owns its SessionStart hook for Claude, Codex and Pi. The hook reports the session id.
  - **User badge hooks**: Claude `settings.json` and Codex `notify` also run a user-owned status badge script that no installer manages.
  - **Anton's install hooks**: Anton's `install.sh` runs `anton-runtime --install-hooks` (`hooks_install.rs`). It installs the marker-owned Pi extension `hooks/observatory.ts` and retires the old Codex hooks, keeping only an inert receipt-owned shim. Codex metrics now come entirely from native replay.
  - **Tracking**: `~/.claude/settings.json` is tracked in place by the user's mise dotfiles.
- **Herdr panes** [obs]:
  - `herdr api snapshot` reports Claude panes as `agent:"claude"` with `agent_session{agent:"claude", source:"herdr:claude", kind:"id", value:<uuid>}`.
  - Each live id resolved to exactly one `<root>/*/<id>.jsonl`.
  - `telemetry::session_binding` (`telemetry.rs:66-78`) is already harness-neutral [repo]. Only `native.rs:680` excludes Claude.
- **`~/.claude.json`** [obs, key names and types only]: provider-owned state, mode 0600, about 160 KB. It has no token, secret or credential keys.
  - `oauthAccount` holds the account profile: `accountUuid`, `emailAddress`, `organizationUuid`, `organizationName`, and billing and tier fields.
  - `cachedUsageUtilization` is `{accountUuid, fetchedAtMs, utilization:{five_hour, seven_day, limits[], …}}`. Windows carry `utilization` and `resets_at` but no duration. Its `accountUuid` equals `oauthAccount.accountUuid`.
  - The cache was about 41 hours old while several Claude sessions were active.
  - `projects.<path>.lastModelUsage` keys sometimes carry `[1m]`, per project rather than per session.
- **Fields that must never leave a parser** [obs]: all `message.content`, `toolUseResult` bodies, `cwd`, `gitBranch`, `attachment.session_context`, `credential_org`, `last-prompt`, `ai-title`, `queue-operation.content`, `pr-link`, and every `tool-results/*` file.

## Goals / Non-Goals

**Goals:**

- Fix every decision change 2 needs, so implementation does not improvise.
- Fix the direction, constraints, user gates and spec deltas for changes 3 and 4. Each of those changes still carries its own design and review.
- Give each Codex metric a Claude source, or a reasoned "unavailable".

**Non-Goals:**

- Implementing anything, or editing specs, tests, AGENTS.md, settings or the plugin.
- Running Claude Code to probe it. Spawning `claude` from a Herdr pane runs the Herdr SessionStart hook, which would rebind the live pane, and also runs user hooks. Unobserved shapes come from [bin] and are exercised with synthetic fixtures.

## Programme

Each change is merged before the next starts.

1. `research-claude-code-parity`: this change.
2. `add-claude-thread-telemetry`: Codex-equivalent native transcript replay for local and peer Claude threads (D1 to D9).
3. `add-claude-status-reporter`: Pi-equivalent installer-owned Claude reporter for the context window and live rate limits (D10).
4. `add-claude-allowances-identity`: Claude allowance rows and verified identity (D11).

## Metric mapping

Anton fields are `technical.telemetry.*` and `technical.turn_timing.*` (`model.rs:52-98`, `State.js:77-168`). "Group" means one deduplicated response (D3).

| Anton field | Codex source | Claude source | Change |
|---|---|---|---|
| `context` | `last_token_usage.total_tokens` | occupancy of the last completed counted group (D4) | 2 |
| `window`, `context_percent` | `model_context_window` with a 12,000-token reserve | statusLine `context_window_size` via the reporter (D10); unknown without it | 3 |
| `total_input` | cumulative input | Σ `input + cache_read + cache_creation` | 2 |
| `total_cache_read` | cumulative cached input | Σ `cache_read_input_tokens` | 2 |
| `total_cache_write` | not reported | Σ `cache_creation_input_tokens` | 2 |
| `total_uncached_input` | `total_input − cache_read` | Σ `input_tokens` | 2 |
| `total_output` | cumulative output | Σ `output_tokens` | 2 |
| last-response `input`, `output_tokens`, `cache_read`, `cache_write` | `last_token_usage` | `input_tokens + cache_read + cache_creation`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens` of the D4 usage object | 2 |
| `model` | turn context | `message.model` of the last counted group with a non-null `stop_reason` | 2 |
| `compactions` | `compacted` and `context_compacted` | `compact_boundary` count (D5) | 2 |
| children and outcomes | typed `SubAgentActivity` | launch, resume and notification records (D6) | 2 |
| `turn_timing.*` | `task_started`, `task_complete`, `turn_aborted` | prompt, `turn_duration` and abort timestamps (D7) | 2 |
| `usage_seq`, `usage_source` | rollout timestamp, `codex-rollout` | D3, `claude-transcript` | 2 |
| 5-hour and weekly allowances | account RPC | reporter `rate_limits` (D10, D11) | 4 |
| identity | account RPC | `~/.claude.json` `oauthAccount` (D11) | 4 |
| pacing | weekly window by duration | the seven-day window | 4 |

## Decisions

### D1. Binding: Herdr session id to exact file, verified by content

**Acceptance.** Accept a Claude pane only when Herdr reports `agent == "claude"` and `agent_session` has `source == "herdr:claude"`, `kind == "id"` and a value that passes `safe_id(value, 128)`.

**Lookup.**
- Look for `<root>/<entry>/<id>.jsonl` at depth exactly two, within the Codex discovery entry and time budgets.
- Require exactly one match. Zero or several matches leave telemetry unknown, and so does a scan truncated by the entry or time budget (`native.rs:651-670`).
- Positive bindings are re-scanned every 60 seconds too, unlike today's cache, which re-discovers only missing paths (`native.rs:698-711`). A second match reverts the session to unknown.
- Never derive the slug, and never use `cwd` or pids.

**Root.**
- The local root is the optional plugin config key `claude_projects_root`. It must be an absolute path, owned by the user, and contain no symlink component.
- Otherwise the local root is `$CLAUDE_CONFIG_DIR/projects` from the collector's own environment, and otherwise `~/.claude/projects`.
- Peers have no plugin config and the probe request carries no root, so a peer uses its own environment default.
- The collector never reads another process's environment. A mismatched config directory leaves the thread unknown.

**Rejected alternatives.**
- **pid or cwd to newest file.** Sibling sessions share slug directories, cwd drifts, `/clear` races the old file, and nothing proves identity.
- **A binding file written by a hook.** It duplicates Herdr's id.

### D2. Identity and confinement

- Open with `common::open_owned`: per-component no-follow, regular file, owner uid. A symlink anywhere fails closed.
- The header is the first line: at most 64 KiB, newline-terminated, with `sessionId == id`. Any first-record type is tolerated.
- Any later record that carries a `sessionId` different from the bound id makes the session's telemetry unknown.
- Fork and branch paths copy the parent's records into the new file and rewrite `sessionId`, adding `forkedFrom` [bin]. A record carrying `forkedFrom` is inherited history and feeds no total, last-response value, turn, child or compaction. Only its presence is read, never the nested id. Change 2 adds a synthetic fork fixture. Records without `sessionId` (`file-history-snapshot`, `file-history-delta`) are ignored for identity and feed no metric. A fork that copied history therefore never counts a response in two sessions.
- Thread replay never reads `<id>/subagents/**`, `tool-results/**`, `memory/**` or `~/.claude.json`.

### D3. Deduplicated usage replay

**Groups.**
- A group is the run of assistant records sharing one `message.id`. Only an assistant record with a different `message.id` closes the open group. User, attachment, system, queue and other records never close or reset it, and skipped `<synthetic>` lines are neutral. (Tool results routinely sit between lines of one response: 142 interleavings across 8 of 18 files, 0 reopenings among assistant records.)
- The cursor keeps the open group's id hash and its counted contribution. A later line of the same group replaces that contribution; it is never added. This holds across replay passes.
- The cursor also keeps a ring of the last 32 closed group id hashes. An assistant record whose id matches one of them makes totals unknown. Main files never did this. Subagent files did, which is one reason they stay out of totals.

**Counted groups.**
- `<synthetic>` records are skipped.
- A group whose last line has `stop_reason: null` (aborted) still counts towards totals, because the request was billed. It is never the last-response or context source.
- A missing or non-numeric counter in a counted group makes totals unknown. It never becomes zero.
- Each condition that makes totals unknown also nulls the last-response values, `context` and `model` (the block's `last_valid` flag) until the next complete counted group. These conditions are the D2 mismatch, a ring match, an unclassifiable assistant record, an unparseable line and a missing counter.
- Totals use the top-level usage, which equals the sum of the `message` iterations. Advisor iterations are excluded, matching Claude Code's own accounting.

**Replay and coverage.**
- Totals come from the replay cursor, not a tail read. Codex reads provider-cumulative totals from a 512 KiB tail (`native.rs:504-580`), but Claude has no cumulative record.
- The whole Claude numeric sample is published only when the cursor has `caught_up && !skipping`. That sample is the totals, last-response values, `context`, `model` and `usage_seq`. This follows the gating Codex applies to turns, children and compactions (`native.rs:764, 783, 815`).
- Each pass reads at most `TAIL` bytes and resumes from the checkpoint, so a cold multi-megabyte transcript takes several passes before values are known.
- `NativeTelemetry` keeps the last published Claude sample in memory per key. A pass that is not caught up re-emits it unchanged, with its original `usage_seq`. A binding change or file replacement (dev/inode, header or tail mismatch) drops it. Growth does not. This is the retention the spec requires for an intermittent read.
- An unparseable line of 64 KiB or less calls `invalid()`, which for Claude rows also clears `totals_valid`.

**`usage_seq`.**
- `usage_seq` is the largest validated group timestamp seen so far, in microseconds.
- It is non-decreasing despite non-monotonic file order, so the `usage_seq >= previous` merge (`native.rs:772-779`) never rejects newer totals.

**Oversized lines.**
- An oversized line (over 64 KiB) goes through a new bounded Claude envelope classifier. The Codex `Envelope` (`envelope.rs`) cannot extract these fields.
- The classifier extracts only the fields D3, D5, D6 and D7 consume: `type`, `subtype`, `sessionId`, `uuid`, `timestamp`, `isMeta`, `origin.kind`, `commandMode`, `message.id`, `stop_reason`, `model`, `usage`, `toolUseResult.{status, agentId, resumedAgentId, success, totalDurationMs}`, `interruptedMessageId`, `isAbortedMidStream`, the presence of `forkedFrom`, the bounded `task-id` and `status` tags, and the leading wrapper tag of user text.
- Its state persists across pass boundaries in the `claude` block, because lines larger than `TAIL` exist.
- An oversized record of a relevant type that cannot be classified makes the dependent coverage unknown:

  | Record type | Coverage made unknown |
  |---|---|
  | assistant | totals |
  | system | compactions and turns |
  | user or attachment | children and turns |

  Large `prompt_snapshot` attachments and tool results are common (10 of 18 files) and are classified, not dropped.

### D4. Context occupancy

- `context` is the occupancy of the last counted group with a non-null `stop_reason`, by Claude Code's rule: `input + cache_creation + cache_read` of the selected iteration or the top level.
- The last-response fields use that same selected usage object, so last-response input never exceeds `context`. Top-level usage feeds only the totals.
- In change 2, `window` and `context_percent` stay null. Change 3 supplies them from the reporter (D10).

**Rejected sources for the window.**
- **A model-to-window table.** A 1M-context session is indistinguishable in transcripts, and Claude Code's own rule depends on beta headers and environment overrides.
- **`~/.claude.json` `lastModelUsage` `[1m]` keys.** They are per project rather than per session, are written outside the session, and sit inside the D11-gated file.

### D5. Compactions

- Count `system/compact_boundary` records, excluding `microcompact_boundary`. Only count with complete coverage from the header and a known `usage_seq`, as for Codex.
- If any counted group has a usage iteration of type `compaction`, `compactions` becomes unknown. That path has no boundary record, and its meaning is unverified.

### D6. Children and completion

**Source and keys.**
- Children come from the main file only.
- Associations use the existing row-level `children` map and `valid` flag, keyed by `sha256(agentId)` and capped at 128, as for Codex.
- `Cursor::validate` additionally accepts the status `unknown`. An older binary rejects such a row and replays it fresh, which is the same cost as D9.

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

**`subagent_status_seq`.** For Claude rows, row-level `seq` starts at 0, overriding its Codex meaning (D8). It becomes the largest validated timestamp among accepted launch, resume and notification records, in microseconds, and is never assigned directly, because file order is not time order. While no child record has been accepted, the published stamp is the largest validated record timestamp replayed (coverage time), never the replay wall time. A thread with no children therefore shows 0 children with an honest source time.

**AGENTS.md amendment.** Change 2 amends AGENTS.md so that "typed native child lifecycle evidence" covers these structured Claude records. A start/stop hook ratio remains forbidden.

### D7. Turn timing

**Classification.** Records are classified in this order of precedence:

1. **Abort:** the interrupt marker text, a record carrying `interruptedMessageId`, or an assistant record with `isAbortedMidStream`.
2. **Turn trigger by origin:** a user record whose `origin.kind` is `human`, `task-notification`, `peer` or `coordinator`, regardless of `isMeta`; or a `queued_command` attachment with `commandMode: "task-notification"`.
3. **Ignored:** `isMeta`, `toolUseResult`, `isCompactSummary`, local-command output, bash-mode records, and `system/compact_boundary` and `microcompact_boundary`. These never start or end turns.
4. **Turn trigger by shape:** a remaining user text record without `origin`, or a slash-command echo.

**Turn triggers.**
- With no active turn, a trigger opens a pending start. It becomes the turn start only when an assistant record follows before the next trigger, including a `<synthetic>` error record, which confirms a start although D3 ignores its usage. Otherwise the newer trigger replaces it.
- The first classified trigger or turn end sets `Turns.supported`, so `complete` can be published.
- With an active turn, a trigger joins that turn only when a `queue-operation` record of any operation (enqueue, dequeue or remove) has appeared since the turn started. That record shows Claude Code queued the input into the running turn [obs shape, inf semantics]. Without it, accumulated coverage becomes unknown, because a turn that ended without a record (for example, a killed process) must not absorb the idle gap.
- An abort while a start is pending confirms that start and ends the turn as aborted at the abort timestamp. An `isAbortedMidStream` assistant record confirms a pending start before it is handled as an abort.
- An abort with no active turn and no pending start is ignored, apart from setting the abort-adjacency flag. The next trigger, assistant record or `turn_duration` clears that flag.
- The turn key is `sha256("anton-turn-v1:" + session + ":" + uuid)[..24]` of the record that opened the turn, which satisfies `Turns::validate`.

**Ending a turn.**
- `system/turn_duration` completes the active turn at its own timestamp.
- An abort ends the active turn as aborted at the abort record's timestamp.
- A `turn_duration` with no active turn is ignored when it directly follows an abort. Otherwise it makes accumulated coverage unknown.

**Timestamps.**
- Timestamps convert to Unix seconds by floor, for both bounds.
- A start earlier than the previous end, or an end earlier than its start, makes accumulated coverage unknown through `Turns::unknown`.
- The current or last valid interval stays available, as the spec allows.

**Durations.** `turn_duration.durationMs` is never substituted. It is used only as a test cross-check.

**State.** The finished intervals and total stay in the unchanged row-level `Turns` struct. Pending-start state lives in the `claude` block (D8), so `Turns` keeps its schema for older binaries.

### D8. Checkpoints

**Rows and keys.**
- Claude rows share the checkpoint file and its 32-session, 256 KiB and 24-hour bounds.
- They inherit the meaningful-progress throttle and installed-owner retirement unchanged.
- Keys are `sha256("anton-native-session-v1:claude:" + id)`, so no raw id is stored. The Codex key (`native.rs:693`) has no harness.
- Offsets are bytes, never timestamps.

**Row-level fields.**
- `Cursor::children`, `valid`, `compactions_valid`, `turns`, `fingerprint` and `offset` keep their existing meaning for Claude rows. `seq` follows D6. `turns` and `fingerprint` remain required, so checkpoint v1 is kept.

**The `claude` block.** All other Claude parser state lives in one required `claude: ClaudeCursor` block with `deny_unknown_fields`. `Cursor` itself does not deny unknown fields (`native.rs:88-110`), so an older binary would silently drop loose fields. A Claude-keyed row whose block is absent or invalid is discarded and replayed fresh, never resumed with zeroed sums.

Every block field is required, bounded and revalidated on reuse:

- the four cumulative sums (`input`, `output`, `cache_read`, `cache_creation`) as `u64` within 2^53, plus `totals_valid`;
- the last-response partition, `context` and `last_valid`;
- `model`: at most 64 characters, using the `telemetry_view_at` character allowlist;
- `usage_seq`;
- the open group's id hash, counted contribution and stop state;
- the ring of at most 32 closed group id hashes;
- the compaction boundary count and the compaction-iteration flag;
- the pending turn start: key hash and Unix second, or none;
- the abort-adjacency flag;
- `queued_since_start`: set by any `queue-operation` while a turn is active, reset when a turn starts or ends, and false whenever `Turns.active` is none;
- the Claude envelope classifier state (D3), or none.

**Rediscovery.** D1 re-scans positive bindings too (D1 "Lookup").

**Resume and replacement.**
- A resume that appends to the same file keeps the binding. Dev/inode, header hash, tail hash and size or mtime still detect replacement.
- A resume that writes a new file elsewhere produces two matches at the next re-scan, so the session becomes unknown.

### D9. Peers and compatibility

**Fields and envelope.**
- Claude fills only existing `Telemetry` and `TurnTiming` fields and adds `claude-transcript` to the `usage_source` allowlist (`telemetry.rs:179-183`). Without that, `telemetry_view_at` would null the source locally.
- The v1 envelope and the probe command are unchanged.

**Mixed versions.**

| Peer | Local | Result |
|---|---|---|
| old | new | Remote Claude threads stay status-only. |
| new | old | Values arrive in existing fields. The old local re-serialises cursors through its own `Cursor` type, which drops the `claude` block, so the new peer discards those rows and replays fresh on every probe. The cost is bounded by `TAIL` per probe and is measured in change 2. |

**Redeployment.** SSH peer redeployment is therefore optional.

**Peer revalidation.** Change 2 also re-runs `telemetry_view` and `turn_timing_view` on peer samples in `State::sample` (`main.rs:191-281`, called from `accept_host`), as `harness-telemetry` already requires.

**Spec and AGENTS.md wording.** Change 2 must make the following harness-neutral, keeping Codex behaviour unchanged:

- `harness-telemetry` "Hook-time structured usage enrichment" (the native Codex reader);
- "Plugin-owned native enrichment" (Codex records; Codex collection without hooks);
- "Scoped cumulative harness metrics" (the native Codex sample retention sentence and the "Intermittent Codex usage read" scenario; the context sentence is assigned to change 3);
- "Bounded native outcome summaries";
- "Native turn wall-clock summaries" (Codex event names);
- "Private native replay checkpoints" (the Claude block and key, explicitly permitting the bounded allowlisted `model` string);
- AGENTS.md lines 18, 46 and 57.

### D10. Claude reporter for window and rate limits (change 3)

**Why a reporter.**
- Codex needs no hook because its transcript carries the window. Pi has the installer-owned extension.
- For Claude, the only surface with `context_window_size` and live `rate_limits` is the statusLine input.
- Plugins cannot provide a statusLine.
- So change 3 adds an installer-owned reporter, run by `anton-runtime --install-hooks` beside the Pi extension.
- The statusLine command is a subprocess that inherits Claude's environment, including `HERDR_*` and `CLAUDE_CONFIG_DIR`.

**Settings value.** The `statusLine.command` in `~/.claude/settings.json` becomes a self-contained, marker-owned shell wrapper that embeds the user's original command, quoted with the existing `shell_quote`. The wrapper:

1. captures stdin once, byte for byte;
2. pipes it to the original command;
3. writes the original command's output unchanged and exits with its exit status. Claude Code hides the status line on a non-zero exit [bin];
4. only then, if the runtime path exists and is executable, starts `anton-runtime --report claude` detached (for example with `setsid`). The reporter gets the saved bytes on its own stdin, has stdout and stderr redirected to `/dev/null`, runs under a total deadline, and ignores all errors. It never holds Claude's pipes.

Two behaviours are unverified and become change 3 risks with tests: whether Claude waits for stdout EOF, and whether aborting a superseded render kills the process group.

A machine without the plugin, including another host receiving the dotfile, therefore still shows the user's statusLine.

**Installer and receipt.**
- The receipt records the original command and the exact wrapper value.
- Uninstall restores the original only when the current value equals the recorded wrapper, and refuses otherwise.
- Reinstall over Anton's own wrapper leaves it alone and never wraps twice.
- A marker wrapper with no local receipt (for example, one arriving by dotfiles) is a conflict: the installer refuses and does not adopt it.
- With no existing statusLine, the installer adds no wrapper.
- Other keys and hooks are preserved byte for byte. The edit targets only the `statusLine.command` value. Re-serialising through `serde_json::Value` would sort keys, because `preserve_order` is unavailable offline. A fixture compares every byte outside that value across install and uninstall.

**Reporter.**
- The reporter runs only with `HERDR_ENV` and a pane.
- A private per-pane state file lets it exit without any RPC when nothing changed.
- Otherwise it sends at most one `pane.report_metadata` RPC per 30 seconds per pane. A change of window size or model bypasses the throttle.
- The reporter exits unless the local receipt records the user's consent and the exact wrapper value. A wrapper that arrived by dotfiles on a host where the user never consented therefore reports nothing.

**Window channel.**
- The window uses v2 pane metadata through the existing `window` slot in `telemetry.rs` `GROUPS`.
- The report is bound by kind `id`, comparing the statusLine `session_id` with Herdr's `agent_session.value`. `reporter.rs` currently requires kind `path` and `harness == "pi"`.
- Wire values: `event:"session"` and `phase:"ready"` (the values the collector already fills in, so nothing visible changes). `seq` is render time in microseconds, kept strictly increasing per pane in the reporter state. `obs_model` is the statusLine model id canonicalised: a trailing `[1m]` or `[2m]` is stripped and the id lower-cased. The `[1m]` suffix that selects a 1M window never reaches transcripts, and the telemetry sanitiser rejects brackets.
- It sends no `usage_seq`, totals or `display_agent`.
- statusLine runs are event-driven: new messages, token usage, model or mode changes, rate-limit reset timers, and `refreshInterval` only when configured. Nothing runs while a session is idle.
- The collector takes `window` only from a bound reporter sample whose canonical model equals replay's last model, compared case-insensitively. A model alias rather than a full id is a change 3 risk, with a fixture, and `context` only from replay (D4). It computes `context_percent` once, without the Codex reserve. `context_window.used_percentage` is not used.

**Rate limits are not part of change 3.** Rate limits belong to an account, not a pane, and v2 pane metadata has no free keys (16 of 16 used, `reporter.rs`). Change 4 extends the reporter with a private account channel (D11), together with the AGENTS.md and spec amendments that make an account read lawful. Change 3 reads no account data.

**Scope limits.**
- Remote hosts have no plugin state. Remote Claude window and allowances are unavailable in this programme.
- These sessions stay without a window: a project or local `statusLine` override, a `CLAUDE_CONFIG_DIR` that differs from the installed settings, an untrusted workspace, or `disableAllHooks`.

**Change 3 gate.** `~/.claude/settings.json` is tracked in place by the user's dotfiles, and AGENTS.md says installers refuse managed configuration. `managed()` detects only chezmoi.

Change 3 starts by asking the user whether Anton may edit this tracked file, given that the wrapper then propagates to other hosts and degrades to the original command there. The answer is enforced in code by an explicit recorded consent flag, not left as a procedural step. If the user declines, change 3 records a blocker and the dial stays unknown.

**Change 3 deltas.**
- `harness-telemetry` "Supplementary harness reports" (Pi-only and local-only wording).
- "Scoped cumulative harness metrics": the Pi context API and Codex reserve sentences; context wording assigned here from D9.
- "Minimal adapter lifecycle": one owned integration per harness.
- AGENTS.md: the "Required Pi reporters" line, line 53, and the "refuse managed configuration" line, scoped to the user-approved exception.

### D11. Allowances and identity (change 4)

The user accepted `~/.claude.json` as provider-owned state, not an authentication file, for identity and usage data. Change 4 amends AGENTS.md accordingly.

**Rate-limit channel.** Change 4 extends the change 3 reporter. On a changed sample, the reporter writes rate limits to a private, owner-checked, atomic, bounded state file under the plugin state directory, keyed by the hashed account. The write is not throttled, because renders are event-driven and a throttled final sample would be lost.

**Account binding at report time.**
- The reporter reads `oauthAccount.accountUuid` from the global config file that Claude Code itself resolves: `($CLAUDE_CONFIG_DIR or $HOME)/.claude.json` [bin]. The read is bounded, owner-checked and no-follow.
- It refuses to attribute when a legacy `.config.json` exists in the config directory, or when `CLAUDE_CODE_CUSTOM_OAUTH_URL` is set. Either changes the file name.
- The collector resolves the identity file by the same rule.
- It hashes the id with the prefix `observatory-claude-account-v1:`, so Claude keys cannot collide with Codex keys (`allowances.rs:63-66`), and stores only the hash.
- The reporter refuses to attribute rate limits when its own environment names a non-subscription auth mode. It checks variable names only: `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`, `CLAUDE_CODE_CUSTOM_OAUTH_URL`, and every provider switch in the binary (`CLAUDE_CODE_USE_BEDROCK`, `_VERTEX`, `_FOUNDRY`, `_ANTHROPIC_AWS`, `_ANTHROPIC_GOOGLE_CLOUD`, `_MANTLE`, `_GATEWAY`).
- It also refuses when the payload contains `rate_limits.spend_limit`, which only gateway mode emits. A gateway sign-in without `spend_limit` is a residual change 4 risk.
- The reporter keeps, per session, the account hash each sample was attributed under. If the account read for a session changes (a `/login` elsewhere), it drops that session's samples until `rate_limits` or `current_usage` change afterwards. In-process limits carry no account id, so a residual risk remains and is recorded in change 4.
- The collector accepts an observation only when its hash matches an explicit Claude account mapping. The mapping gains a provider field, and the user obtains the key from diagnostics, as for Codex.

**Identity.**
- Collection reads `oauthAccount.{accountUuid, emailAddress}` only. `organizationUuid` is not needed.
- The email is shown only after the existing hashed mapping check.

**Source time.**
- The statusLine payload has no sample time.
- The reporter stamps each window separately. A window's `sampled_at` becomes render time only when that window gains a new `(used_percentage, resets_at)` value, or when `current_usage` changed on this render. A window that disappears because it expired is not a new sample: Claude Code schedules a render at each reset, with no API response behind it. Otherwise the previous stamp is kept. Change 4 adds a fixture for the reset-timer render.
- A session's first report after state loss is not stamped fresh until `current_usage` changes on a later render.
- In the per-account file, the newest `sampled_at` per window wins. A write with an older stamp is ignored.
- So the existing ten-minute rule ages out idle accounts honestly.
- `cachedUsageUtilization` is a fallback only when its `accountUuid` matches, with `fetchedAtMs` as its source time. It usually fails the ten-minute rule.

**Durations and pacing.**
- Neither source carries a duration. Change 4 maps the provider window names `five_hour` (18,000 s) and `seven_day` (604,800 s) to durations.
- The seven-day window is the single pacing window. A row with only `five_hour` is available but has no pacing window, with a fixture.
- Scoped weekly limits come only from the cache fallback, and are never pacing.

**Change 4 deltas.** `account-allowances`:
- "Account-bound allowance observation" (source type: reporter observation or provider state file, not only an RPC; fallback freshness);
- "Fleet-bound account sources" (Claude is local-only);
- "Provider-neutral allowance rows" (duration by window name for Claude, single pacing window);
- any Codex-specific pacing wording, and the Codex-only Purpose;
- `omarchy-companion`: "Native read-only identity RPCs SHALL be matched against existing account mappings" gains the Claude provider-state source.

**Forbidden sources.**
- The credential file and the OAuth usage endpoint.
- `claude auth status` and headless `/usage`.
- Transcript `credential_org` and `session_context`.

## Risks / Trade-offs

- **[Unobserved shapes: compaction, synchronous children, resume-to-new-file]** These come from [bin] or [doc]. Change 2 makes unrecognised variants unknown, not zero, and its fixtures cover both resume outcomes.
- **[Claude Code changes its formats]** Validation fails closed to unknown. Each record carries `version`, so later changes can gate on it.
- **[Reporter edits user settings]** It is reversible and receipt-checked, and a missing runtime must not break the user's statusLine. If change 3 cannot meet that, the context dial stays unknown and change 4 relies on the stale cache.
- **[Peer cost]** A cold replay of large transcripts takes several bounded passes, and an old local forces fresh replay on each probe. Change 2 measures both.

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
- **Record identity** [obs 18/18, 0 mismatches]: every record's `sessionId` in a main file equals the file stem.
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
  - Main files: all 740 multi-line groups were contiguous with identical usage.
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
- **Prompts** [obs]: human prompts are user records with text content, no `toolUseResult` and no `isMeta`.
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

- **statusLine** [bin, doc]:
  - Input JSON includes `session_id`, `transcript_path`, `model` and `context_window.{context_window_size, used_percentage, current_usage, total_input_tokens, total_output_tokens}`. Both totals are last-response values [bin].
  - For Pro and Max, after the first response, it also includes `rate_limits.{five_hour, seven_day}.{used_percentage, resets_at}`.
  - It runs inside the Claude process on each render.
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

- Fix every decision that changes 2 to 4 need, so implementation does not improvise.
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
| `context` | `last_token_usage.total_tokens` | occupancy of the last counted group (D4) | 2 |
| `window`, `context_percent` | `model_context_window` with a 12,000-token reserve | statusLine `context_window_size` via the reporter (D10); unknown without it | 3 |
| `total_input` | cumulative input | Σ `input + cache_read + cache_creation` | 2 |
| `total_cache_read` | cumulative cached input | Σ `cache_read_input_tokens` | 2 |
| `total_cache_write` | not reported | Σ `cache_creation_input_tokens` | 2 |
| `total_uncached_input` | `total_input − cache_read` | Σ `input_tokens` | 2 |
| `total_output` | cumulative output | Σ `output_tokens` | 2 |
| last-response `input`, `output_tokens`, `cache_read`, `cache_write` | `last_token_usage` | `input_tokens + cache_read + cache_creation`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens` of the last counted group | 2 |
| `model` | turn context | `message.model` of the last counted group | 2 |
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
- Require exactly one match. Zero or several matches leave telemetry unknown.
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
- Any later record whose `sessionId` differs from the bound id makes the session's telemetry unknown. A fork that copied history therefore never counts a response in two sessions.
- Thread replay never reads `<id>/subagents/**`, `tool-results/**`, `memory/**` or `~/.claude.json`.

### D3. Deduplicated usage replay

**Groups.**
- A group is the contiguous run of assistant records with the same `message.id`.
- The cursor keeps the open group's id hash and its counted contribution. A later line of the same group replaces that contribution; it is never added. This holds across replay passes.
- A record whose `message.id` matches an already closed group makes totals unknown. Main files never did this. Subagent files did, which is one reason subagent files stay out of totals.

**Counted groups.**
- `<synthetic>` records are skipped.
- A group whose last line has `stop_reason: null` (aborted) still counts towards totals, because the request was billed. It is never the last-response or context source.
- A missing or non-numeric counter in a counted group makes totals unknown. It never becomes zero.
- Totals use the top-level usage, which equals the sum of the `message` iterations. Advisor iterations are excluded, matching Claude Code's own accounting.

**Replay and coverage.**
- Totals come from the replay cursor, not from a tail read. Codex reads provider-cumulative totals from a 512 KiB tail (`native.rs:504-580`); Claude has no cumulative record. So Claude totals follow the `caught_up` gating that Codex applies to turns, children and compactions (`native.rs:764, 783, 815`).
- Each pass reads at most `TAIL` bytes and resumes from the checkpoint, so a cold multi-megabyte transcript takes several passes before totals are known.

**`usage_seq`.**
- `usage_seq` is the largest validated group timestamp seen so far, in microseconds.
- It is non-decreasing despite non-monotonic file order. This keeps the `usage_seq >= previous` merge (`native.rs:782-791`) from rejecting newer totals.

**Oversized lines.**
- An oversized line (over 64 KiB) goes through a bounded envelope classifier. The classifier extracts only the fields D3, D5, D6 and D7 consume: `type`, `subtype`, `sessionId`, `uuid`, `timestamp`, `isMeta`, `origin.kind`, `commandMode`, `message.id`, `stop_reason`, `model`, `usage`, `toolUseResult.{status, agentId, resumedAgentId, success, totalDurationMs}`, `interruptedMessageId`, `isAbortedMidStream`, and the bounded `task-id` and `status` tags.
- An oversized record of a relevant type that cannot be classified makes the dependent coverage unknown:
  - assistant: totals;
  - system: compactions and turns;
  - user or attachment: children and turns.

### D4. Context occupancy

- `context` is the occupancy of the last counted group, by Claude Code's rule: `input + cache_creation + cache_read` of the selected iteration or the top level.
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
- Associations are keyed by `sha256(agentId)` and capped at 128, as for Codex.

**Records.**
- **Launch.** A `toolUseResult` with `status: "async_launched"` and an `agentId` starts a child as running. A synchronous Agent result with `agentId` and `totalDurationMs` records it as completed.
- **Launch without `agentId`.** An `async_launched` result with no `agentId` is a workflow or teammate launch. It is ignored.
- **Resume.** A `toolUseResult` with `resumedAgentId` naming a known child, and `success: true`, returns that child to running. This is the Claude form of "resumed work invalidates old completion". A repeat launch of a known child does the same.
- **Completion.** Completion is a user record whose `origin.kind` is `task-notification`, or a `queued_command` attachment with `commandMode: "task-notification"`.
  - A bounded tag grammar extracts only `task-id` and `status`.
  - Nothing else in the block is read.
  - Notifications whose task id matches no known Agent child are ignored. These come from shell tasks, workflows and teammates.

**Status mapping.**

| status | outcome |
|---|---|
| `completed` | completed |
| `failed` | failed (the `errored` bucket) |
| `killed` | interrupted |
| `blocked` or any unrecognised status | `subagent_unknown` |

`subagent_unknown` keeps the partition summing to `subagent_total`.

**Invalidation.** Any of the following makes children unknown, as Codex's `valid = false` does:
- a recognised record with a malformed `agentId`;
- a notification that names a known child before its launch;
- exceeding the cap.

**AGENTS.md amendment.** Change 2 amends AGENTS.md so that "typed native child lifecycle evidence" covers these structured Claude records. A start/stop hook ratio remains forbidden.

### D7. Turn timing

**Classification.** Classify records in this order of precedence:

1. **Abort:** the interrupt marker text, a record carrying `interruptedMessageId`, or an assistant record with `isAbortedMidStream`.
2. **Non-turn:** `isMeta`, `toolUseResult`, `origin.kind` other than `human`, a `commandMode`, or text wrapped as a slash-command echo, local-command output or bash-mode record.
3. **Human prompt:** text content with `origin.kind` absent or `human`.

**Starting a turn.**
- A human prompt opens a pending turn. The start becomes final only when an assistant record follows before the next prompt candidate.
- The turn key is `sha256("anton-turn-v1:" + session + ":" + uuid)[..24]`, satisfying `Turns::validate`.

**Ending a turn.**
- `system/turn_duration` completes the active turn at its own timestamp.
- An abort ends the active turn as aborted at the abort record's timestamp.

**Unknown coverage.** Each of these makes accumulated coverage unknown through `Turns::unknown`, so it never fabricates or merges intervals:
- a human prompt while a turn is active, unless a `queue-operation` record proves it was queued into that turn;
- a `turn_duration` with no active turn, unless it directly follows an abort of the same turn;
- turns opened by task notifications, peers or coordinators;
- a start earlier than the previous end, or any end earlier than its start.

The current or last valid interval stays available, as the spec allows.

**Durations.** `turn_duration.durationMs` is never substituted. It is used only as a test cross-check.

### D8. Checkpoints

**Rows and keys.**
- Claude rows share the checkpoint file and its 32-session, 256 KiB and 24-hour bounds.
- They inherit the meaningful-progress throttle and installed-owner retirement unchanged.
- Keys are `sha256("anton-native-session-v1:claude:" + id)`, so no raw id is stored. The Codex key (`native.rs:693`) has no harness.
- Offsets are bytes, never timestamps.

**The `claude` block.**
- Claude parser state lives in one required `claude: ClaudeCursor` block with `deny_unknown_fields`.
- `Cursor` itself does not deny unknown fields (`native.rs:88-110`), so an older binary would silently drop loose fields.
- A Claude-keyed row whose block is absent or invalid is discarded and replayed fresh. It is never resumed with zeroed sums.

**Block contents.** Every field below is required and bounded, and is revalidated on reuse:

- the four cumulative sums (`input`, `output`, `cache_read`, `cache_creation`) as `u64` within 2^53;
- the last-response partition and `context`;
- `model`, as an allowlisted string of at most 80 characters;
- `usage_seq`;
- the open group's id hash, counted contribution and stop state;
- the validity flags `totals_valid`, `children_valid` and `compactions_valid`;
- the compaction-iteration flag;
- the boundary count.

`turns` and `fingerprint` remain required at row level, so checkpoint v1 is kept.

**Resume and replacement.**
- A resume that appends to the same file keeps the binding, and dev/inode, header hash, tail hash and size or mtime still detect replacement.
- A resume that writes a new file elsewhere produces two matches (D1), so the session is unknown.

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

**Peer revalidation.** Change 2 also re-runs `telemetry_view` and `turn_timing_view` on peer samples in `accept_host` (`main.rs:579-592`), as `harness-telemetry` already requires.

**Spec and AGENTS.md wording.** Change 2 must make the following harness-neutral, keeping Codex behaviour unchanged:

- `harness-telemetry` "Hook-time structured usage enrichment" (the native Codex reader);
- "Plugin-owned native enrichment" (Codex records; Codex collection without hooks);
- "Bounded native outcome summaries";
- "Native turn wall-clock summaries" (Codex event names);
- "Private native replay checkpoints" (Claude block and key);
- AGENTS.md lines 18, 46 and 57.

### D10. Claude reporter for window and rate limits (change 3)

Codex needs no hook because its transcript carries the window. Pi has the installer-owned extension. For Claude, the only surface with `context_window_size`, `used_percentage` and live `rate_limits` is the statusLine input. Plugins cannot provide a statusLine. So change 3 adds an installer-owned reporter.

**Installation.**
- `anton-runtime --install-hooks` replaces the `statusLine` command in `~/.claude/settings.json` with a marker-owned wrapper.
- A receipt records the user's original command verbatim.
- The installer refuses symlinks and conflicts, and keeps every other key, hook and file.
- Uninstall restores the original command only when the current value is still the marker-owned wrapper.

**Wrapper behaviour.**
- The wrapper passes stdin to the original command unchanged and prints its output unchanged.
- It extracts only `context_window.{context_window_size, used_percentage}`, `rate_limits.{five_hour, seven_day}.{used_percentage, resets_at}` and `session_id`.
- It reports them through `anton-runtime --report claude <pane>` and `pane.report_metadata`, as Pi does.
- It reports only when a value changes, with a bounded rate.
- Without `HERDR_ENV` and a pane it only runs the original command.

**Collection.**
- The collector accepts the reported window only when its `session_id` equals the bound Herdr id.
- It then computes `context_percent` without the Codex reserve.

**Change 3 must also decide:**
- the owned-installation contract for a mise-tracked file. Live files are authoritative, so the installer edits in place. It refuses only chezmoi-managed paths, as today.
- the metadata v2 key budget;
- the failure mode if the runtime is missing, where the original command must still run.

### D11. Allowances and identity (change 4)

The user accepted `~/.claude.json` as provider-owned state, not an authentication file, for identity and usage data. AGENTS.md is amended accordingly.

**Identity.**
- Read only `oauthAccount.{accountUuid, emailAddress, organizationUuid}`, bounded, with owner and no-follow checks.
- Show the email only after the existing hashed mapping check.
- The account key is derived from a hash of `accountUuid`.

**Windows.**
- Use live `rate_limits` from the reporter (D10), bound to the account of the reporting process's config directory.
- `cachedUsageUtilization` is a fallback only when its `accountUuid` matches. Its `fetchedAtMs` is the observation time, so the existing ten-minute freshness rule usually rejects it. That is honest, but means rows mostly come from the reporter.
- Neither source carries a duration. Change 4 adds an `account-allowances` delta mapping the provider window names `five_hour` (18,000 s) and `seven_day` (604,800 s) to durations. The seven-day window is the single pacing window.
- Scoped weekly limits are optional and are never pacing.

**Forbidden sources.**
- the credential file and the OAuth usage endpoint;
- `claude auth status`;
- headless `/usage`;
- transcript `credential_org` and `session_context`.

## Risks / Trade-offs

- **[Unobserved shapes: compaction, synchronous children, resume-to-new-file]** These come from [bin] or [doc]. Change 2 makes unrecognised variants unknown, not zero, and its fixtures cover both resume outcomes.
- **[Claude Code changes its formats]** Validation fails closed to unknown. Each record carries `version`, so later changes can gate on it.
- **[Reporter edits user settings]** It is reversible and receipt-checked, and a missing runtime must not break the user's statusLine. If change 3 cannot meet that, the context dial stays unknown and change 4 relies on the stale cache.
- **[Peer cost]** A cold replay of large transcripts takes several bounded passes, and an old local forces fresh replay on each probe. Change 2 measures both.

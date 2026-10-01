# Design

## Context

Evidence for this design was gathered on 2026-10-01 with Claude Code 2.1.285,
Herdr 0.9.1 and herdr-observatory `main` at `2d2be90`. evidence.md lists the
sources and the method. Labels:

- **[obs]**: observed on this host, recorded as structure and counts only.
- **[bin]**: found in strings of the installed Claude Code binary. Internal and
  liable to change between versions.
- **[doc]**: official Claude Code documentation, not verified locally.
- **[repo]**: a `file:line` in this repository at `2d2be90`.
- **[inf]**: inferred.

The local corpus is live, so counts drift. They describe the survey date, not a
fixed oracle. Change 2 tests use synthetic fixtures built from the shapes below.

### Transcript layout and identity

- Root [bin]: `$CLAUDE_CONFIG_DIR/projects`, otherwise `~/.claude/projects`.
- One main session is `<root>/<slug>/<sessionId>.jsonl` [obs, 18 files].
- Agent-tool children are `<root>/<slug>/<sessionId>/subagents/agent-<agentId>.jsonl`
  with a `.meta.json`. Workflow workers live under `subagents/workflows/wf_<id>/`.
  Spilled tool output lives under `<sessionId>/tool-results/`. [obs]
- The slug is derived from the starting cwd with a lossy, undocumented rule
  [bin]. Per-record `cwd` drifts within a file (6 of 18 files) [obs]. Neither is
  usable for binding.
- The first line of every main file is `{"type":"mode","mode":…,"sessionId":<id>}`
  [obs 18/18]. Mode records also recur mid-file, so the header carries identity
  only, with no version or cwd.
- Every record's `sessionId` in a main file equals the file stem [obs 18/18,
  0 mismatches]. `isSidechain` is false on every main-file record and true on
  every subagent record [obs], so children are never interleaved into the parent.
- `--resume` and `--continue` reuse the original session id. Only
  `--fork-session` creates a new one [obs, `claude --help`]. `/clear` and fork
  produce a new id [doc, Herdr hook matcher].
- Timestamps are ISO-8601 UTC with milliseconds on every record that has one, but
  are not monotonic in file order (23 negative gaps) [obs].
- Sizes [obs]: main file p50 721 KB, max 5.4 MB. Line p50 1.6 KB, p99 42.7 KB,
  max 832 KB, with 5 lines over 256 KiB. The last assistant line always started
  within 73 KB of end of file.

### Usage records

- Assistant records carry `requestId` and `message.{id, model, stop_reason, usage}`
  [obs].
- `usage` always has `input_tokens`, `output_tokens`, `cache_read_input_tokens`
  and `cache_creation_input_tokens`. Complete records add `iterations[]`,
  `output_tokens_details`, `server_tool_use` and `speed` [obs].
- One API response is written as one line per content block, sharing
  `message.id` and `requestId` [obs]. In main files the 740 multi-line groups
  were contiguous with identical usage. In subagent files earlier lines are
  streaming partials, and 6 ids reappeared after another id had started. In
  every multi-line group the last line had the maximum `output_tokens`. Summing
  every main-file line overcounts input 2.5× and output 3×.
- 86 responses had iterations `message, advisor_message, message`. In every case
  the top-level usage equalled the sum of the `message` iterations [obs]. Claude
  Code's own context function takes the last iteration that is neither
  `advisor_message` nor `compaction`, uses it only when its type is `message` or
  `fallback_message` with four numeric counters summing above zero, and
  otherwise falls back to top-level usage [bin].
- `message.model == "<synthetic>"` marks client-generated error lines with zero
  usage (6) [obs].
- No transcript record carries a context window size. `[1m]` never appears in a
  model string (0 of about 5,800) [obs].
- `cost-state` records hold cumulative `modelUsage` but appear only 1 to 9 times
  per file and exceed the deduplicated main-plus-subagent replay by up to 30%
  [obs]. Their scope is not the session transcript.

### Other surfaces

- **Compaction** [bin, not observed]: `{type:"system", subtype:"compact_boundary",
  compactMetadata:{trigger, preTokens, …}}`, followed by a user record with
  `isCompactSummary: true`. `microcompact_boundary` also exists and is not a
  compaction. A usage iteration of type `compaction` also exists, which suggests
  server-side compaction inside one response with no boundary record [bin, inf].
- **Turns** [obs]: human prompts are user records with text content, no
  `toolUseResult` and no `isMeta`. Newer ones carry `origin.kind` (`human`,
  `task-notification`, `peer`, `coordinator`), `promptId` and `turnPosition`, but
  51 non-meta prompts had no `origin`. Turn ends are `system/turn_duration`
  records (104). Aborts are a user text `[Request interrupted by user]` (and a
  "for tool use" variant [bin]), `interruptedMessageId` or `isAbortedMidStream`.
- **Children** [obs]: an async Agent launch writes a parent `tool_result` whose
  `toolUseResult` has `status: "async_launched"` and `agentId` (17, each with a
  matching child file). Completion arrives as a user record with
  `origin.kind == "task-notification"` (37) or inside a `queued_command`
  attachment (12). Its content is a provider-generated tag block with
  `task-id`, `status`, `summary`, `result` and `usage`. Observed statuses are
  `completed` and `failed`. [bin] adds `killed` and `blocked`. A synchronous
  Agent result carries `toolUseResult.{agentId, totalDurationMs, totalTokens, …}`
  [bin, not observed]. Workflow launches and teammates have no `agentId` child
  file. Notifications for background shell tasks and workflows carry task ids
  that match no Agent child.
- **statusLine** [bin, doc]: input JSON includes `session_id`,
  `transcript_path`, `model`, `context_window.{context_window_size,
  used_percentage, current_usage, total_input_tokens, total_output_tokens}`
  (the two totals are last-response values [bin]) and, for Pro and Max after the
  first response, `rate_limits.{five_hour, seven_day}.{used_percentage,
  resets_at}`. It runs inside the Claude process on every render, in a single
  slot that the user already occupies [obs]. It carries no account identifier.
- **Hooks** [doc, bin]: SessionStart (`startup|resume|clear|compact|fork`),
  Stop, SubagentStart, SubagentStop (with `agent_transcript_path` [bin]),
  PreCompact, SessionEnd and others. Plugins can ship hooks [bin].
- **Herdr** [obs]: `herdr api snapshot` reports Claude panes as `agent:"claude"`
  with `agent_session{agent:"claude", source:"herdr:claude", kind:"id",
  value:<uuid>}`. A Herdr-managed SessionStart hook reports the id through
  `pane.report_agent_session`, skipping subagents. Each live id resolved to
  exactly one `<root>/*/<id>.jsonl`. `telemetry::session_binding`
  (`telemetry.rs:66-78`) is already harness-neutral [repo]. Only `native.rs:680`
  excludes Claude.
- **`~/.claude.json`** [obs, key names and types only]: provider-owned state
  file, mode 0600, about 160 KB. It contains no token, secret or credential keys.
  `oauthAccount` holds the account profile (`accountUuid`, `emailAddress`,
  `organizationUuid`, `organizationName`, billing and tier fields).
  `cachedUsageUtilization` holds `{accountUuid, fetchedAtMs,
  utilization:{five_hour, seven_day, limits[], …}}`, where windows have
  `utilization` and `resets_at`. Its `accountUuid` equals
  `oauthAccount.accountUuid`. On the survey date the cache was about 41 hours
  old while several Claude sessions were active, so normal use does not keep it
  fresh.
- **Fields that must never leave a parser** [obs]: all `message.content`,
  `toolUseResult` bodies, `cwd`, `gitBranch`, `attachment.session_context`,
  `credential_org`, `last-prompt`, `ai-title`, `queue-operation.content`,
  `pr-link`, and every `tool-results/*` file.

## Goals / Non-Goals

**Goals:**

- Fix every decision change 2 needs, so implementation does not improvise.
- Give each Codex metric a Claude source or a reasoned "unavailable".
- Define the change 3 source gate.

**Non-Goals:**

- Implementing anything, or editing specs, tests, AGENTS.md or the plugin.
- Running Claude Code to probe it. Spawning `claude` from a Herdr pane runs the
  Herdr SessionStart hook, which would rebind the live pane, and runs user hooks.
  Shapes that were not observed come from [bin] and are exercised with synthetic
  fixtures.
- Hook or statusLine collection (D10).

## Metric mapping

Anton fields are `technical.telemetry.*` and `technical.turn_timing.*`
(`model.rs:52-98`, `State.js:77-168`). "Group" means one deduplicated response
(D3).

| Anton field | Codex source | Claude source | Status |
|---|---|---|---|
| `context` | `last_token_usage.total_tokens` | Occupancy of the last counted group, by Claude's rule (D4) | available |
| `window`, `context_percent` | `model_context_window` with a 12,000-token reserve | No compliant source | **unavailable** (D4) |
| `total_input` | cumulative input | Σ `input + cache_read + cache_creation` | available |
| `total_cache_read` | cumulative cached input | Σ `cache_read_input_tokens` | available |
| `total_cache_write` | not reported | Σ `cache_creation_input_tokens` | available, new for a native source |
| `total_uncached_input` | `total_input − cache_read` | Σ `input_tokens` | available |
| `total_output` | cumulative output | Σ `output_tokens` | available |
| last-response fields | `last_token_usage` | the same partition from the last counted group | available |
| `model` | turn context | `message.model` of the last non-synthetic group | available |
| `compactions` | `compacted` and `context_compacted` | `compact_boundary` count (D5) | available, unobserved shape |
| `subagent_total`, `subagent_done`, outcomes | typed `SubAgentActivity` | launch results and task notifications (D6) | available |
| `turn_timing.*` | `task_started`, `task_complete`, `turn_aborted` | prompt, `turn_duration` and interrupt timestamps (D7) | available |
| `usage_seq`, `usage_source` | rollout timestamp, `codex-rollout` | group timestamp, `claude-transcript` | available |
| allowances: 5-hour and weekly | account RPC | `~/.claude.json` `cachedUsageUtilization`, if accepted (D11) | gated |
| identity | account RPC | `~/.claude.json` `oauthAccount`, if accepted (D11) | gated |
| pacing | weekly window by duration | weekly window by duration | contract unchanged |

## Decisions

### D1. Binding: Herdr session id to exact file, verified by content

Accept a Claude pane only when Herdr reports `agent == "claude"` and
`agent_session` has `source == "herdr:claude"`, `kind == "id"` and a value that
passes `safe_id(value, 128)`. Look for `<root>/<entry>/<id>.jsonl` at depth
exactly two, within the Codex discovery entry and time budgets. Require exactly
one match. Zero or several matches leave telemetry unknown. Never derive the slug
and never use `cwd` or pids.

The root is a new optional plugin config value, the Claude projects root,
defaulting to `$CLAUDE_CONFIG_DIR/projects` from the collector's own environment
and otherwise `~/.claude/projects`. The collector never reads another process's
environment. A Claude process with a different config directory is therefore
unknown unless the user configures that root. This satisfies the existing
"configured session roots" requirement.

Rejected: pid or cwd to newest file (siblings share slug directories, cwd drifts,
`/clear` races the old file, no identity proof); a plugin hook or statusLine
writing a binding file (duplicates Herdr, conflicts with the user's slot and with
collection-time enrichment).

### D2. Identity and confinement

- Open with `common::open_owned`: per-component no-follow, regular file, owner
  uid. A symlink anywhere fails closed.
- The header is the first line, at most 64 KiB, newline-terminated, with
  `sessionId == id`. Tolerate any first-record type.
- Any later record carrying a `sessionId` other than the bound id makes the
  session's telemetry unknown. This turns a fork that copied history into
  unknown, so no response is counted in two sessions.
- Never read `<id>/subagents/**`, `tool-results/**`, `memory/**` or
  `~/.claude.json` during thread replay.

### D3. Deduplicated usage replay

- A group is the contiguous run of assistant records with the same
  `message.id`. The cursor keeps the open group's id hash and its counted usage.
  When the next line of the same group arrives, its usage replaces the open
  group's contribution; it is never added. This holds across replay passes.
- A record whose `message.id` matches an already closed group makes totals
  unknown. Main files never did this. Subagent files did, which is one reason
  they stay out of totals.
- `<synthetic>` records are skipped. Groups that end with `stop_reason: null`
  (aborted) still count their usage, because the request was billed, but are
  never the last-response or context source.
- Totals use the top-level usage, which equals the sum of `message` iterations.
  Advisor iterations are excluded, matching Claude Code's own accounting.
- Totals are complete only after replay from the header has caught up, as for
  Codex (`native.rs:561-569`). A partial or tail-only read leaves totals unknown.
- An oversized line (over 64 KiB) goes through a bounded envelope classifier
  that extracts only `type`, `subtype`, `sessionId`, `message.id`,
  `stop_reason`, `model` and `usage`. If an oversized assistant or system record
  cannot be classified, totals and compaction coverage become unknown, as
  `finish_envelope` does for Codex.

### D4. Context occupancy without a window

`context` uses Claude Code's rule from the Usage records section. `window` and
`context_percent` stay null. The popover therefore shows the Claude context dial
as unknown, which is a visible parity gap.

Rejected:

- A model-to-window table. 1M-context sessions are indistinguishable in
  transcripts, so a table would invent a denominator.
- A statusLine bridge. It runs inside the Claude process, needs the user's single
  slot and a writable bridge file, and installers refuse managed configuration.

Revisit only if a provider-owned, collection-time window source appears.

### D5. Compactions

Count `system/compact_boundary` records, excluding `microcompact_boundary`, only
with complete coverage from the header. If any counted group has a usage
iteration of type `compaction`, `compactions` becomes unknown, because that path
has no boundary record and its meaning is unverified. Resume keeps the session
id, so compactions accumulate across resumes of the same file.

### D6. Children and completion

- Children come from the main file only. A launch is a parent `tool_result` with
  `toolUseResult.agentId` and `status: "async_launched"` (running), or a
  synchronous Agent result with `agentId` and `totalDurationMs` (completed).
  Associations are keyed by `sha256(agentId)`, capped at 128 as for Codex.
- Completion is a user record whose structured `origin.kind` is
  `task-notification`, or a `queued_command` attachment of that origin. A bounded
  tag grammar extracts only `task-id` and `status`. Nothing else in the block is
  read.
- Status mapping: `completed` to completed; `failed` to errored; `killed` to
  interrupted; anything else to unknown, which invalidates outcomes.
- Notifications whose task id matches no known Agent child are ignored. These are
  shell tasks and workflows.
- A launch record for an already-known child returns it to running. This is the
  Claude form of "resumed work invalidates old completion".
- Change 2 amends AGENTS.md so that "typed native child lifecycle evidence"
  includes this structured-origin notification. The status is provider-generated
  and typed by `origin.kind`. A start/stop hook ratio is still forbidden.

### D7. Turn timing

- A turn starts at a human prompt record: a user record with text content, no
  `toolUseResult`, no `isMeta`, and an `origin.kind` that is absent or `human`.
  The turn key is `sha256("anton-turn-v1:" + session + ":" + uuid)` of that
  record. `promptId` is not required.
- A prompt that arrives while a turn is active joins that turn, so queued prompts
  are never counted twice.
- A `system/turn_duration` record completes the active turn at its own
  timestamp. An interrupt marker, `interruptedMessageId` or `isAbortedMidStream`
  aborts it at that record's timestamp.
- `turn_duration.durationMs` is never substituted. It is used only as a test
  cross-check.
- An end earlier than its start, a missing timestamp or an unparseable one makes
  turn timing unknown, through the existing `Turns::unknown`.

### D8. Checkpoints

Claude cursors share the checkpoint file and its 32-session, 256 KiB and 24-hour
bounds. Keys are namespaced (`anton-native-session-v1:claude:<id>`), because the
Codex key at `native.rs:693` has no harness. Cursor offsets are bytes, never
timestamps. Claude rows supply `turns` and `fingerprint`, so checkpoint v1 is
kept. The open group state and the compaction-iteration flag are allowlisted
cursor fields. Resume appends to the same file, so the dev/inode, header hash,
tail hash and size or mtime fingerprint continue to detect replacement, and a
new Herdr id creates a new binding.

### D9. Peers and compatibility

Claude fills only existing `Telemetry` and `TurnTiming` fields and adds
`claude-transcript` to the `usage_source` allowlist (`telemetry.rs:179-183`).
The v1 envelope and probe command are unchanged.

- An old peer with a new local: remote Claude threads stay status-only.
- A new peer with an old local: values arrive in existing fields. The unknown
  `usage_source` is only carried.
- So SSH peer redeployment is optional, not a protocol requirement.

Change 2 also re-runs `telemetry_view` and `turn_timing_view` on peer samples in
`accept_host` (`main.rs:579-592`), as `harness-telemetry` already requires.

### D10. Hooks and statusLine are not telemetry sources

AGENTS.md and `harness-telemetry` place enrichment in collection, not in
synchronous callbacks. Every metric in D3 to D7 is available from the transcript.

### D11. Allowances and identity gate for change 3

| Candidate | Verdict |
|---|---|
| Credential file plus OAuth usage endpoint (the Omarchy collector's path) | Forbidden. It parses an authentication file. |
| statusLine `rate_limits` | Not compliant. It needs an in-process callback, the user's slot and a bridge file, and has no identity. |
| Transcript `quotaLimits` | Present only on rejection. No live windows. |
| `claude auth status --json`, headless `/usage` | Not run. They may refresh tokens, write session files or call the network. A headless run from a Herdr pane rebinds the pane and runs user hooks. |
| `~/.claude.json` `cachedUsageUtilization` and `oauthAccount` | Leading candidate. It is provider-owned, read-only for Anton and holds no credentials, and its `accountUuid` binds windows to identity. Its freshness depends on Claude Code refreshing the cache, which normal use did not do for about 41 hours. |

Change 3 starts by asking the user whether `~/.claude.json` counts as an
authentication file under AGENTS.md. If it is accepted, change 3 reads only the
`oauthAccount` identity fields and `cachedUsageUtilization`:

- bounded, owner and no-follow checked;
- windows selected by duration;
- original `fetchedAtMs` and `resets_at` kept, with past resets invalidating
  balances;
- email shown only after the existing hashed mapping check.

If it is declined, change 3 records a source blocker with this evidence, as
`add-notion-allowance` did, and implements nothing.

## Risks / Trade-offs

- **[The Claude context dial shows unknown]** Accepted per D4, because inventing
  a window is worse.
- **[Compaction and synchronous child shapes are unobserved]** They come from
  [bin]. Change 2 makes unrecognised variants unknown rather than zero.
- **[Claude Code changes its transcript format]** Validation fails closed to
  unknown. `version` is recorded per line, so later changes can gate on it.
- **[The allowance cache is stale]** Staleness is shown honestly from
  `fetchedAtMs`. A 5-hour window whose reset has passed is invalid, not zero.
- **[Peer cost]** Each probe repeats Claude discovery and the tail read. Change 2
  measures this with the harness.

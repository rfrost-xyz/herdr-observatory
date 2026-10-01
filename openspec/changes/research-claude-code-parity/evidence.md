# Evidence

All evidence was gathered on 2026-10-01. It records structure, counts and command
outcomes only. No transcript content, session ids, account values, project names
or private paths appear here.

## Sources and method

| Source | Method |
|---|---|
| Local Claude Code transcripts | Read-only scripts that printed only key unions, record type counts, ratios and size percentiles. Corpus: 18 main transcripts (7,797 lines) and 61 subagent transcripts (8,433 lines), 0 malformed lines. The corpus is live, so later counts differ slightly (for example, 63 subagent files and 104 `turn_duration` records in the critique pass). |
| Claude Code 2.1.285 binary | `strings` with context extraction for the statusLine payload builder, `rate_limits`, `compact_boundary`, the context function, hook schemas, plugin hooks and the projects root. Internal names are not recorded. |
| `claude --help`, `claude auth --help`, `claude auth status --help` | Help text only. `claude auth status` and headless `/usage` were not run (design D11). |
| code.claude.com documentation | statusLine, hooks, Claude directory, authentication and costs pages. |
| Herdr 0.9.1 | `herdr --help`, `agent list`, `api snapshot`, `api schema --json`, `integration status`, reduced to field shapes and agent kinds. The Herdr-managed Claude hook script was read. |
| User `settings.json` | Key names only. The statusLine slot is occupied. |
| `~/.claude.json` | `jq` over key paths and value types only, plus two boolean or numeric checks: cache `accountUuid` equals profile `accountUuid` (true), and cache age in seconds (about 150,000). |
| `/usr/share/omarchy/bin/omarchy-agent-usage-claude` | Read for format knowledge only. It reads the credential file and calls the OAuth usage endpoint, which AGENTS.md forbids for Anton. |
| This repository at `2d2be90` | Source and specs, with `file:line` references in design.md re-checked by the critique pass. |

Research ran as a read-only workflow: five researchers, a synthesis and an
adversarial critique. The critique re-verified the load-bearing claims. Its 13
numbered points held 10 corrections, all applied in design.md:

- the context rule excludes `compaction` iterations and applies a validity fallback;
- a second compaction path exists;
- `--resume` keeps the session id;
- fork duplication and non-contiguous groups;
- Codex-only wording in AGENTS.md and the spec;
- a configured projects root;
- the `usage_source` allowlist consequence;
- headless `/usage` mutation risks;
- the SDK child status enum;
- the date of the counts.

The critique found no private data in the synthesis.

## Review rounds

Round 1, an independent review of `0bfce30` through two lenses (factual accuracy,
and decision completeness against AGENTS.md and the canonical specs), found 7
blocking and 13 other findings. Each is addressed in design.md:

- D3: coverage citation, totals from the cursor, `usage_seq`, the oversized-field allowlist and missing counters.
- D6: `commandMode`, launches without `agentId`, resume via `resumedAgentId`, the unknown bucket, `blocked`, and notification before launch.
- D7: classification precedence, pending starts, queued prompts, orphan ends and overlap.
- D8: a required `claude` block that denies unknown fields, the full allowlist, and hashed keys.
- D9: the old-local cost, and the list of spec requirements to modify.
- D1: the configured root, which is local only.
- D4: explicit formulas, and the `lastModelUsage` rejection.
- D11: freshness, and duration by window name.
- Evidence counts and labels.

Round 2, on `e05f901`, found 9 blocking and 17 other findings. It added:

- D3: group closing by assistant records only, sample gating and the classifier state;
- D2: records without `sessionId`;
- D8: the full block allowlist, with Claude turn state kept out of `Turns`;
- D1: positive-binding re-scans and truncated scans;
- D6: `unknown` status, resumes and notifications for unknown children, `subagent_status_seq`;
- D7: triggers, queued input, aborts while pending, and floor conversion;
- D9: the spec list;
- D10: wire channels, a self-contained wrapper, the managed-file gate and deltas;
- D11: account binding at report time, source time, key prefix and deltas.

Line references were also corrected.

Round 3, on `edd2383`, found 4 blocking and 17 other findings, all addressed:

- D7: classify by origin before `isMeta` (peer turns), let synthetic records confirm a pending start, carry the `queued_since_start` state, accept any queue operation, ignore compaction records, and set `supported`;
- D6: take the maximum `subagent_status_seq`;
- D3: add `last_valid` and in-memory retention of the last sample;
- D4: source last-response values from the selected usage object;
- D9: cite `State::sample`;
- D10: define the wire values, wrapper exit and detachment, event-driven cadence, model match, exact-match ownership and consent flag;
- D11: resolve the config file, handle account switches, define the merge order, and handle a missing `seven_day` window.

A structure-only simulation of D7 with the round 3 fixes reported valid accumulated coverage in 18 of 18 main files: 109 turns, 0 orphan ends and 0 overlaps (reviewer's measurement). Round 7 showed that figure was too generous: silent turn ends let stale turns absorb idle gaps.

Round 4, on `6e01c99`, found 6 blocking and 9 other findings, all addressed:

- D2: fork records copy history under the new id and are now inherited history;
- D6: Claude `seq` starts at 0, with coverage time when there are no children;
- D7: aborts confirm a pending start, plus the abort-adjacency reset;
- D10: canonical `[1m]` model comparison, throttle on the RPC only, receipt consent at report time, and byte-preserving settings edits;
- D10 and D11: the rate-limit channel moved to change 4 with its account amendments, per-window stamping that ignores expiry renders, the full provider switch list, `spend_limit` refusal, and `omarchy-companion` deltas.

Round 5, on `4f2b02d`, found 3 blocking and 9 other findings, all addressed:

- D3: local retention of peer samples, `usage_seq` taken from counted lines, the compaction-summary classifier, and the assistant coverage row;
- D8: `coverage_seq`;
- D7: the trigger after unknown, and queued prompt attachments;
- D9: the old-local peer bound;
- D10: the change key with binding, persist after success, no throttle, uninstall that tolerates drift, and conflicts that skip only the Claude reporter;
- D11: the no-session behaviour with thread-independence deltas, and refusal under `CLAUDE_CONFIG_DIR`.

The reviewer's D7 replay reported valid coverage in 18 of 18 files (114 turns, 0 orphans, 0 overlaps, 37 queued triggers joined). This was later corrected in round 7. Discovery used 84 entries of the 8,192 budget.

Round 6, on `8b5c5dc`, found 1 blocking and 5 other findings, all addressed:

- **Plugin function hooks.** The blocking finding was that plugin function hooks (`session.measure`) are a second surface carrying the window and rate limits. It is verified in the binary: `context` is `{tokens, window, percent}` and `rateLimits` lists `{kind, percentUsed, resetsAt}`. It is recorded, and the A/B choice moves to the change 3 gate.
- **D3:** a precise peer re-emit trigger and subset.
- **D7:** the wrapper tag table.
- **D10:** process identity and expiry in the change key, plus mode preservation and compare-before-rename for the settings edit.

Round 7, on `07966f9`, found 3 blocking and 8 other findings, all addressed:

- D7: silent turn ends (stop-hook summary or `end_turn`) clear the queued state. With that change the older-version file becomes unknown, and the largest joined gap fell from 66,701 s to 764 s;
- D1 and D2: the snake-case `session_id` after `/clear`, with a fail-closed predecessor check;
- D3: every caught-up pass publishes, so the peer re-emit trigger is exact;
- D8: row-level field ownership;
- D10: the Claude pid is passed by the wrapper, and stdin is captured with a sentinel;
- D11: a pattern rule for credential variables and `apiKeyHelper`, the `~/.claude.json` credential caveat with allowlisted extraction, source normalisation, and the row sample time.

Round 8, on `4ea41ee`, found 3 blocking and 5 other findings, all addressed:

- D1: the predecessor scan is bounded by bytes, with three outcomes. The 16-record window never reached `session_id` in 4 of 4 successors;
- D3: explicit retention drop triggers, a peer all-null sample on binding failure, and the no-cursor case;
- D11: stamping only on changed window values, because rewinds change `current_usage`;
- D4: replay omits `window` rather than writing nulls;
- D10: a POSIX sh wrapper, a mandatory new session, the shell-prefix and managed-policy limits, and the render triggers;
- D11: deltas for reporter account reads and inferred account authority.

The final bounded review, on `bb5ba89`, checked fact labels, privacy, consistency and compliance only, following the scope-of-authority pivot. It found 2 minor wording findings and 2 change 2 inputs, all applied. It verified every repository reference, the binary strings, the corpus structure and the privacy scan. A re-run on `bb34de9` found 2 wording contradictions (rate limits placed in change 3; PostModelSwitch fields) and 3 change 2 inputs. All were applied.

## User decisions (2026-10-01)

- `~/.claude.json` is accepted as provider-owned state, not an authentication file, for Claude identity and usage data.
- The context window needs hook equivalence with Codex and Pi. After reviewing how hooks are created today, the equivalent is an installer-owned Claude reporter, the counterpart of Anton's Pi extension. It becomes programme change 3 (design D10). Round 6 found a second surface, plugin function hooks, so the choice of surface (statusLine A or function hook B) moved to the change 3 gate.

Herdr environment variables are present in agent shells, so any Claude Code probe from a pane would rebind that pane through the Herdr SessionStart hook. No such probe was run.

## Key measurements behind decisions

| Claim | Evidence |
|---|---|
| Session id equals file stem and every main-file `sessionId` | 18 of 18 files, 0 mismatching records among records that carry `sessionId`. `file-history-snapshot` and `file-history-delta` carry none |
| `isSidechain` separates parent and child files | All main-file records false, all subagent records true |
| Split responses share `message.id` | 740 main-file multi-line groups, all contiguous among assistant records (user and attachment records interleave 142 times), with identical usage. Subagent files: 1,311 multi-line groups (first pass), 1,287 with streaming partials, 6 non-contiguous ids |
| Summing every line overcounts | Main files: input 2.53×, output 3.04× |
| Last line of a group is the maximum | All multi-line groups in both passes (2,190 of 2,190 in the second pass) |
| Advisor iterations | Second pass: 51 deduplicated responses (94 lines) across main and subagent files. Top-level usage equals the sum of `message` iterations in 51 of 51 |
| No window size in transcripts | 0 window fields. `[1m]` in 0 of about 5,800 model strings |
| Compaction records | 0 observed. Shape from the binary |
| Turn ends | 103 `turn_duration` records. `durationMs` matches the prompt-to-record gap (median difference 0.011 s, 98 turns), but with outliers: in round 7, 96 of 115 turns were within ±2 s. One older-version file had 22 stop-hook summaries and only 2 `turn_duration` records |
| Non-monotonic timestamps | Common in file order. The first pass counted 23 negative gaps, method unrecorded. Round 7, comparing every consecutive timestamped record, counted 274 (assistant, attachment and user records) |
| Async children | 23 `async_launched` results: 17 with `agentId` (17 matching child files), 6 without. 49 task notifications, statuses `completed` 47 and `failed` 2. 20 notification task ids matched a known child, 32 did not. 4 of 13 `queued_command` notifications had no `origin` but all had `commandMode: "task-notification"` |
| Child resume | 8 `SendMessage` uses. 5 results carry `resumedAgentId` and `success` |
| Prompt-shaped non-turns (second pass) | Records without `origin` that match a naive prompt predicate: 19 slash-command echoes, 12 local-command outputs, 2 bash-mode records, 3 interrupt markers |
| Hook payloads | 33 hook event schemas in the binary. No command-hook payload carries a context window size or rate limits. SessionStart carries `model` and, on resume or fork, `context_tokens`. PostModelSwitch carries `from_model`, `to_model` and `context_tokens` |
| Plugin statusLine | The plugin customisation table disables `statusLine` [bin] |
| Hook ownership on this host | Herdr integration current for Claude (v10), Codex (v8) and Pi (v9). The user badge script is wired into Claude settings and Codex `notify`. Anton's installer owns only the Pi extension and the retired Codex shim |
| Herdr Claude binding | Every live Claude pane had `agent_session` with keys `{agent, kind, source, value}`, kind `id`. Each id resolved to exactly one transcript |
| Line sizes | p50 1.6 KB, p99 42.7 KB, max 832 KB. 5 lines over 256 KiB. Last assistant line within 73 KB of end of file |

## Baseline (unchanged `2d2be90`)

Release binary: `cargo build --release --locked --offline`, sha256
`74f50d695c89e4cc01198c77df9d31ab9b1230e8e5948a6a73dceb569d76931d`.

`node tests/measure_anton_popover.mjs --binary <bin> --source-root . --repeat 3 --json`:

| Metric | Median |
|---|---|
| Runtime CPU over 30 s (2 synthetic hosts, 32 agents each) | 0.031 s |
| Peak RSS | 4,100 KiB |
| Mean snapshot bytes | 27,674.6 |
| Snapshots | 10 |
| colors.toml opens (inotify) | 0 |
| Herdr RPCs, local and remote | 15 and 6 |
| Connected | 12 ms |
| Provider coupling in presentation files | 0 |

Scope: synthetic fixtures only. Runtime CPU is user plus sys of the runtime and
its reaped descendants, including fake-SSH peer probes. Remote hosts, Qt and GPU
are not measured.

Suites:

| Suite | Result |
|---|---|
| `cargo fmt --check`, `cargo clippy --locked --all-targets -D warnings` | pass |
| `cargo test --locked`, three consecutive runs | 105 passed each run (65 lib, 8, 6, 26), no intermittent failure seen |
| `node --test` (Pi hooks, State, distribution) | 87 of 87 |
| `tests/run-qml.sh` | 95 passed, 0 failed |
| `tests/run-qmllint.sh` | no warnings outside Panel.qml |
| `tests/run-shell-harness.sh` (installed Omarchy modules) | failures 0, runtime invocations 6, `--open-thread` 0 |

Installed plugin: every QML file and `State.js` is byte-identical to the
`2d2be90` source, and the installed `anton-runtime` equals the baseline build.

## Change-specific gates

- **Tests that fail on the old behaviour:** not applicable. This change edits
  only planning artefacts, so it has no behaviour to test. Change 2 owns the
  failing-first Claude fixtures.
- **Live check:** not applicable. The installed plugin is unchanged and was
  verified equal to `2d2be90` above. No install or peer change is part of this
  change.

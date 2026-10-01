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
adversarial critique. The critique re-verified the load-bearing claims and
produced 13 corrections, all applied in design.md:

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

The critique found no private data in the synthesis. Herdr environment variables
are present in agent shells, so any Claude Code probe from a pane would rebind
that pane through the Herdr SessionStart hook. No such probe was run.

## Key measurements behind decisions

| Claim | Evidence |
|---|---|
| Session id equals file stem and every main-file `sessionId` | 18 of 18 files, 0 mismatching records |
| `isSidechain` separates parent and child files | All main-file records false, all subagent records true |
| Split responses share `message.id` | 740 main-file groups, all contiguous with identical usage. Subagent files: 1,311 groups, 1,287 with streaming partials, 6 non-contiguous ids |
| Summing every line overcounts | Main files: input 2.53×, output 3.04× |
| Last line of a group is the maximum | 2,059 of 2,059 multi-line groups |
| Advisor iterations | 86 responses. Top-level usage equals the sum of `message` iterations in 86 of 86 |
| No window size in transcripts | 0 window fields. `[1m]` in 0 of about 5,800 model strings |
| Compaction records | 0 observed. Shape from the binary |
| Turn ends | 103 `turn_duration` records. `durationMs` matches the prompt-to-record gap (median difference 0.011 s, 98 turns) |
| Non-monotonic timestamps | 23 negative gaps in file order |
| Async children | 17 launches, 17 matching child files. 49 task notifications, statuses `completed` 47 and `failed` 2 |
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

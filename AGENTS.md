# Anton: native Omarchy plugin

This repository supports the Anton menubar/popover plugin and its on-demand
native SSH peer. The web dashboard, HTTP service, Docker deployment and
Work/music forwarding are retired. Historical OpenSpec archives describe former
versions; they do not authorise restoring retired services. See README.md and
omarchy/herdr.observatory/README.md for current operation.

## Boundaries

- Collection is read-only. Never send agent input, alter Herdr lifecycle/session
  authority, attach terminals or expose terminal/transcript content.
- Only explicit user navigation may call `herdr agent focus` for the exact
  configured host/session/pane and raise/open its matching terminal. One attempt,
  no fallback to another host. Preserve most-recent matching window behaviour.
- Required Pi reporters and the Claude Code mod's reporter may read `pane.get`
  and write bounded owned presentation metadata through `pane.report_metadata`.
  Native Herdr remains authoritative. The Claude Code mod only observes events,
  passes each on unchanged, never waits on its reporter and sends only the pane,
  sequence, session id, window and, from a measurement with fresh evidence (a
  rate-limit window that moved or appeared, or a grown session cost total), at
  most two `five_hour`/`seven_day` windows as argv, never the cost itself; its
  reporter writes one bound window report for agent `claude` and, when
  attribution is allowed, those windows to the private Claude account state
  file, and nothing else. Codex and Claude Code transcript enrichment belong to
  collection, not synchronous Observatory callbacks.
- No Python or Docker subprocess, web listener, independent daemon or autostart
  service is part of the supported plugin. QML owns the Rust collector lifetime.
  Peers are invoked over existing authenticated SSH and exit after bounded work.
  The reporter child the Claude Code mod starts is bounded and short-lived, and
  the mod skips a sample while a recent run is in flight; it is not a daemon.
- Never parse authentication files. `~/.claude.json` is provider-owned state,
  not an authentication file: it is read only by the Claude reporter, the
  collector, identity refresh, `--claude-account-key` and
  `--claude-attribution-check`, which read only `oauthAccount.accountUuid`,
  `oauthAccount.emailAddress` and `cachedUsageUtilization`, never log, persist
  or retain any other value, store the email only in the private identity file,
  and skip the whole file when `primaryApiKey` is present; the Claude reporter
  and attribution check may also read whether `~/.claude/settings.json` sets
  `apiKeyHelper`. Account observations use native read-only Codex account RPCs
  and, for Claude, mod rate limits attributed to the account `~/.claude.json`
  names at report time, refused whenever the environment, configuration location
  or settings could select another credential or API endpoint, or Claude Code's
  usage cache when it names the same account and is fresh. No
  login/reset/redemption mutation. Verify hashed mapping before displaying an
  email; private email identity never enters normal snapshots.
- Preserve unknown, stale and zero distinctly. Never invent tokens, allowances,
  child completion, timing, or measurement freshness from transport heartbeats.
- No private configuration, credentials, live snapshots, project names, session
  paths or account identity in source/tests/review evidence. Fixtures are synthetic.
- Build from a locked offline dependency graph. No runtime package installation,
  assembly or intrinsics without a measured justified hotspot.

## Source ownership

`omarchy/anton-runtime` contains the native library, collector and CLI. Its modules
own bounded file/process/socket operations, config, collection, telemetry,
validated native replay/checkpoints, allowances/identity, navigation, Pi reporting
and integration/peer receipts. `omarchy/herdr.observatory` contains QML/JS, manifest
and shell build/install/uninstall. `hooks/observatory.ts` is the Pi extension and
`hooks/claude/anton-observatory` is the local-only Claude Code mod.

During an active native migration, old Python files are temporary development
oracles only. Remove them after parity fixtures preserve plugin guarantees. Do
not add features or compatibility work to the retired web application.

## Native data contracts

- Validate exact native session identity/header, session-root confinement,
  ownership and every symlink boundary before reading Codex or Claude Code
  records. Bound discovery, file reads, line/envelope parsing, total work and
  caches. Claude Code binds Herdr's session id to exactly one transcript;
  ambiguous, truncated, forked or predecessor bindings stay unknown, and split
  responses count once.
- Cumulative input/output/cache/uncached counters have session scope; last-response
  values have response scope. Preserve original source timestamps. Pi input
  excludes cache buckets at source: include read/write in total input while
  retaining source input as uncached; Claude Code follows the same partition.
  Missing complete totals remain unknown.
- Codex context uses its verified baseline reserve; Pi uses supported context
  API. Claude Code takes its window only from a bound local Claude Code mod
  report for the pane's current session; its percentage is replay context over
  that window, rounded half up, without the Codex reserve. Without such a report,
  or with a window smaller than the context, the window and percentage stay
  unknown; peer Claude Code threads carry neither, and a peer collects them
  without reading reporter metadata. Never invent a window from a
  model table. Compactions require complete bounded coverage. Repeated or reset counters
  cannot create activity. Reported old values remain last-known for the same
  bound live session; session replacement invalidates them.
- Codex and Claude Code completion is based on typed native child lifecycle
  evidence (Claude Code: structured launch, resume and task-notification
  records) and bounded hashed associations, never start/stop hook ratios. Resumed work invalidates old
  completion. Outcome partitions must sum coherently; older total/done-only
  metadata remains usable without inventing running children. No raw child IDs,
  names, prompts or results enter popover/peer telemetry.
- Turn timing uses validated saved Unix-second starts/completions/aborts, includes
  waits inside turns, excludes idle gaps and counts each interval once. Never
  substitute `duration_ms` or `durationMs`; Claude Code `durationMs` may only
  reject an interval whose saved bounds disagree by over 2 s or that lacks it.
  Claude Code joins queued input to a turn only with dequeue or remove evidence;
  silent turn ends and aborts make coverage unknown, and its current turn is
  shown only from the session start or after a checked end, with no background
  agent pending.
  Incomplete accumulated coverage stays unknown.
- Native metadata v2 uses immutable four numeric groups plus named provenance,
  child/completion/outcome fields within 16 report keys and 80 characters/value.
  Never split a coherent report across equal sequence writes or mix wire versions.
- Checkpoints are private, atomic, versioned, bounded to 32 sessions, 256 KiB and
  24 hours. Store only hashed associations/allowlisted cursor state. Revalidate
  file/tail identity on reuse. Checkpoint load is never a fresh measurement.
  Throttle meaningful writes and use installed-owner retirement to exclude late
  writes, delayed startup and final flush. Uninstall must fail safely if busy.
- Allowances use explicit account mappings and source identity, independent of
  thread activity, except Claude: its rows are local-only and need an active
  local reporting session or Claude Code's fresh account-matched cache. Weekly
  windows are selected by duration (Codex) or by window name (Claude
  `seven_day`), never field order. Past resets invalidate balances; pass expiry
  invalidates count without a refill. Use native availableCount, not pass-list
  length or credits. Cache bounds and original source times remain enforced
  locally and on peer responses.

## Installation and peer contract

The local plugin owns `~/.config/omarchy/plugins/herdr.observatory` and private
state under `~/.local/state/herdr.observatory`. The peer owns only
`~/.local/share/herdr.observatory-peer` and
`~/.local/state/herdr.observatory-peer`. Never use the unrelated existing
`~/.local/share/anton`. No remote container/service is managed by this plugin.

Keep private config/accounts, concealment preferences and owner marker inode on
updates. Explicit migration removes retired publication/music and known container
export fields. Installers refuse conflicts/symlinks/managed configuration and
preserve unrelated hooks. For the Claude Code mod, managed configuration also
covers mise dotfiles (history entries and `[dotfiles]` declarations in every
mode; the declarations are read with `mise config get -f`, which renders no
template-mode dotfile source, and mise loads the user's configuration, including
`[env]`, as any mise command does)
and Git repositories with a real `.git` marker; it installs
only locally under `~/.claude/skills/anton-observatory/`, never on a peer, and
removal refuses a changed recorded file and keeps the receipt; a recorded
directory is kept only when it still holds entries, and any other failure
to remove one fails the removal with the receipt kept. Its reporter needs a
local host configured with `socket_path` and does nothing without one. Hook receipt
writers hold one exclusive lock and refuse as busy after a bounded wait. Remove old callbacks/files only with proven ownership.
Peer provision has a marked receipt; local `.peers.json` records provisioned
SSH targets. Uninstall completes recorded peer removal first. On unreachable or
conflicting peers, retain local installation and remaining receipt for retry.
Never delete unknown files or unrelated applications. Parent performs installed
or fleet changes only after reviewed source acceptance.

During Codex hook migration, retain the receipt-owned inert shell compatibility
helper for already-running callers. It drains stdin and exits successfully
without registration or collection, and explicit receipt-checked uninstall
removes it. An empty current hooks list does not prove cached callers are gone.

## Presentation

Preserve current QML visuals: monochrome Agents glyph with static priority mark,
compact square single-column popover, themed status accents, collapsible sections
and machines, icon filters, four equally weighted metric slots, concise tooltips,
local email concealment and exact thread navigation. Account balance and pacing
stay distinct; positive hover particles occupy only the surplus strip, deficits
use a static hatch. New visible thread entrance is about 300 ms; hydration,
reconnect, filtering, expand/collapse and sorting must not replay it. Working
sheen only while visible; reduced motion/hidden state suppress effects.

## Verification and lifecycle

Use change-lifecycle, git/worktrunk and repository OpenSpec skills. Explicit local
scope overrides Git/forge defaults; preserve pre-existing dirty work. Delegate
bounded ownership when requested, then independently review frozen source.

Run locked/offline Rust tests, format and Clippy with warnings denied. Run native
process/install/retirement/peer fixtures, Pi/State JS suites, QML production
fixtures and manifest validation. Audit shipped dependencies for Python/Docker/
web paths. Tests must exercise malformed/oversized/stale/zero values, session
replacement, owner EOF, deadlines, restart and clean uninstall. Compare native
outputs to captured synthetic oracle fixtures, not live private snapshots.

Record CPU/RSS scope and duration including local child processes. Distinguish
controlled fixtures from live workload, exclude remote CPU and Qt/GPU explicitly
where unmeasured, and do not claim wake-ups from context switches. Verify the
actual installed popover, both hosts, account refresh, navigation, owner restart,
source hashes and private-state preservation before canonical sync/archive.
Clean only owned temporary artefacts. Historical archives remain as evidence.

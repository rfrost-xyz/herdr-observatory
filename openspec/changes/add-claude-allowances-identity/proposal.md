# Proposal

## Why

After change 3 (`add-claude-status-reporter`), local Claude Code threads show a
context window and percentage, but Claude accounts have no allowance row or
verified email. Codex rows come from a native read-only account RPC; Claude
Code has no such RPC Anton may use. The credential file, the OAuth usage
endpoint, `claude auth status` and `/usage` are forbidden sources (research
D11). The change 3 mod already receives `session.measure`, which carries the
rate-limit windows of the last API response, and Claude Code keeps its account
profile and a usage cache in `~/.claude.json`, which the user accepted as
provider-owned state rather than an authentication file.

This is change 4 of 4, the last change of the Claude Code parity programme.

## What Changes

- **Mod.** On `session.measure` only, the mod appends the rate-limit windows it
  has fresh evidence for to the change 3 argv: at most two windows, kind
  `five_hour` or `seven_day`, `percentUsed` (0 to 100, at most one decimal) and
  `resetsAt` as epoch seconds. A sample is fresh only when it is not the first
  measurement since the mod loaded or the session ended, nor for another
  session id, and either
  `rateLimits` is in `changed` with a window's used value moved or a window
  appeared, or `cost` is in `changed` and the session's cost total strictly
  grew over the previous measurement (a real API response; plan-gate answer
  G4, 2026-10-04). The cost total stays in the mod's memory and is never
  sent. Rewinds and compactions without cost growth, reset expiry, a reset
  time moving alone and start or `/resume` readings are never fresh. Any
  `spend_limit` window means no rate limits are sent for that sample or any
  later one until the session ends. The mod stays argv-only, never awaited
  and never blocking.
- **Reporter.** `anton-runtime --report claude` accepts the optional rate-limit
  tail with strict validation; four values keep their change 3 meaning. After
  the pane binding and sequence checks and the unchanged window report, and
  only when the window report did not fail, still under the hook lock, it
  decides attribution: it refuses when any environment
  variable name matches the credential refusal patterns (with a closed
  exemption list naming only `CLAUDE_CODE_MESSAGING_TOKEN`, G3),
  `ANTHROPIC_BASE_URL` is set (G1), `CLAUDE_CONFIG_DIR` is set, a legacy
  `~/.claude/.config.json` exists, user settings set `apiKeyHelper`, or
  `~/.claude.json` holds `primaryApiKey`. Otherwise it hashes
  `oauthAccount.accountUuid` with the prefix `observatory-claude-account-v1:`
  and writes the windows to a private, atomic, bounded per-account state file,
  newest sample per window winning. A session whose account changes is never
  attributed again. The pane metadata wire is unchanged: still the window only.
- **Collector.** Account mappings gain an optional `provider` field (`codex` by
  default, or `claude`). For a Claude mapping the collector builds a row from
  the per-account file, with `cachedUsageUtilization` from `~/.claude.json` as
  a fallback only when its account matches and it is itself fresh. Windows map
  by name to durations (`five_hour` 18,000 s, `seven_day` 604,800 s);
  `seven_day` is the single pacing window. The existing ten-minute rule applies,
  so with no active Claude session the row turns unavailable. Claude rows are
  local-only: peers never report them.
- **Identity.** `--refresh-identities` adds the local Claude email for a
  mapped Claude account, matched by the hashed `accountUuid`, and matches
  Codex RPC and peer identity rows to Codex mappings only. A new private
  command, `--claude-account-key`, prints only the hashed key of the current
  Claude account so the user can write the mapping. A private dry run,
  `--claude-attribution-check`, prints only the first refusing step's name and
  matching variable names, so a refused live session can be diagnosed. These
  and identity refresh widen decision 1's readers of `~/.claude.json` (reporter
  and collector) to the same three allowlisted fields, as the user accepted
  at the plan gate (G7).
- **Contracts.** Amend `account-allowances`, `omarchy-companion`,
  `harness-telemetry`, AGENTS.md and the READMEs to allow the three allowlisted
  `~/.claude.json` paths, the presence-only `apiKeyHelper` read, the inferred
  Claude attribution and the Claude exception to thread-independent refresh.
- Measure before and after with `tests/measure_anton_popover.mjs`, adding a
  Claude allowance section without changing existing metrics.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `account-allowances`: account observation admits Claude provider state and
  reporter observations with inferred attribution; Claude is local-only; Claude
  windows are selected by name, with `seven_day` pacing; a Claude row depends
  on an active reporting session or a fresh matched cache.
- `omarchy-companion`: identity display admits the Claude provider-state email
  after the hashed mapping check; account refresh independence gains the Claude
  exception.
- `harness-telemetry`: the Claude Code mod and reporter may carry rate limits
  through argv to the private account state file, and the reporter may read the
  allowlisted `~/.claude.json` paths and `apiKeyHelper` presence; pane metadata
  still carries the window only.

## Impact

- **Runtime:** `reporter.rs` (rate-limit tail, attribution, account file write),
  a new Claude account module (typed `~/.claude.json` reader, account state
  file, row builder), `allowances.rs` (`provider` mapping, Claude rows in
  `snapshot_at`), `identity.rs` (Claude email), `main.rs`
  (`--claude-account-key`, `--claude-attribution-check`, periodic
  provider-state read).
- **Mod:** `hooks/claude/anton-observatory/hooks/register.js` (rate-limit tail,
  freshness baseline with the cost total, no `Date`). Installed mods refresh
  through `--install-claude-mod`; the receipt version is unchanged.
- **Shell:** `uninstall.sh` removes the new state file. No new installed file.
- **Tests:** Rust unit and process fixtures, `tests/test_claude_mod.mjs`,
  `tests/test_omarchy_state.cjs`, QML provider-neutral checks and an additive
  measurement section. Synthetic `~/.claude.json` fixtures in temporary homes
  only.
- **Docs:** AGENTS.md, README.md, the plugin README and three specs.
- **Compatibility:** a configuration holding a `provider` field is rejected by
  older runtimes, so rollback means removing Claude mappings first. A new mod
  with an old runtime loses the window report until the runtime is updated.
- **User configuration:** the user adds a Claude mapping to private
  configuration. Nothing writes under `~/.claude` apart from the change 3 mod
  refresh, and `~/.claude.json` and `~/.claude/settings.json` are only read.
- **Out of scope:** peer Claude allowances and identity, scoped (per-model)
  limits from the cache, reset passes for Claude, the credential file, the OAuth
  usage endpoint and `$.session.authorize()`.

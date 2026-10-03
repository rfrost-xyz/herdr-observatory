# Evidence

Synthetic fixtures only. No private transcripts, session ids, account ids,
emails, paths, provider-state values, settings values or snapshots are recorded
here. Host facts are recorded as key names, types, counts, kinds, booleans and
equality results only.

## Baseline

To be recorded by task 1.2 before any runtime change, from the unchanged
`7a9fefb` binary:

- release binary sha256: _pending_;
- `tests/measure_anton_popover.mjs --repeat 3`: runtime CPU, peak RSS and mean
  snapshot size: _pending_;
- `claude_allowance` section (task 1.1): reporter wall time and CPU for four-
  and ten-value runs, collector snapshot time with a Claude mapping, Claude row
  and available counts, snapshot bytes: _pending_. Expected on the old binary:
  ten-value runs exit 2, the Claude mapping makes the configuration invalid, so
  the Claude metrics are null;
- suites green: _pending_.

## Claude Code types (Claude Code 2.1.287) [types]

Read-only extraction of the installed build's mods types (14,914 lines), made
during change 3 and re-read on 2026-10-04. Line numbers refer to that copy.

- `'session.measure'` (4111-4122): "Fires when the engine measures the session
  and a unit moved: after each main-thread turn, and when a rate-limit window
  moves a whole point." "One at a time, a burst folding into one more."
- `SessionMeasureInput` (10378-10404): `context`, `rateLimits:
  SessionRateLimit[]` ("The rate-limit windows the last response reported ...
  empty off a subscription or before the first reading"), `cost?`, `changed:
  UsageUnit[]` ("never empty; the first measurement names every unit it has a
  figure for"; "`rateLimits`: a window moved a whole point, appeared or left,
  or the account's limit status changed; `cost`: the total grew").
- `SessionRateLimit` (10565-10580): `kind: string` ("`five_hour`,
  `seven_day`, or a Claude gateway's `spend_limit`"), `percentUsed: number`
  ("0 to 100 with at most one decimal ... past 100 on an exceeded spend
  limit"), `resetsAt?: string` ("an ISO 8601 timestamp").
- `SessionUsage` (10983-11010) and `$.session.usage` (2603-2624): the same
  `rateLimits`, plus `startedAt` (session start, not a sample time).
- `UsageUnit` (13562): `'context' | 'rateLimits' | 'cost'`.
- `SessionContextUsage` (10225-10258): `tokens` and `percent` are absent until
  the first response of the live window, including just after compaction.
- `$.env` (3354-3372): "The environment of this process, the one every Bash
  child, MCP server and `$.process.run` command started after inherits";
  `get` takes a string-literal name.
- `ProcessRunInit.env` (7503-7515): "Variables set over the host process's own
  environment."
- `$.settings` (3329-3352): settings cross unfiltered, `env` and helper
  commands included; "The OAuth session and the global config (~/.claude.json)
  are not settings and are never read."
- `$.session.authorize()` (2680-2691, 10018-10021): an opaque handle and kind
  `bearer` or `api-key`, or null for a third-party provider, a gateway or no
  login. Not used (design G8).
- `session.end` reasons (4124, 4500) include `logout`. No login or account
  event exists.
- Absent everywhere: account id, email, organisation, subscription type,
  sample or response time, window duration, limit status field.

## Provider state [provider state]

Probed on 2026-10-04 from a private temporary directory, deleted afterwards.
Key names, types and format classes only.

- `~/.claude.json`: regular file, owned by the user, mode 0600, about 165 KB.
  Top level is an object. `primaryApiKey` absent.
- `oauthAccount.accountUuid`: string, uuid-like, 36 characters.
  `oauthAccount.emailAddress`: string containing `@`.
- `cachedUsageUtilization`: exactly `accountUuid` (equal to the profile's),
  `fetchedAtMs` (integer, epoch milliseconds, over one day old at probe time)
  and `utilization` (26 keys).
  - `five_hour`, `seven_day`: `{utilization: integer, resets_at: ISO string
    with numeric offset and fractional seconds, three dollar fields (null),
    locked_reason (null)}`. Both resets were in the past at probe time.
  - `limits`: 3 entries `{kind, group, percent, resets_at, scope, severity,
    is_active}`; no `kind` equals `five_hour` or `seven_day`; one is
    model-scoped. Not used.
  - Every utilisation and percent value was 0, so the scale (0 to 100 or 0 to
    1) is unconfirmed (design D7, open question 1).
- Legacy `~/.claude/.config.json`: absent.

## Environment and settings [environment]

- Clean login shells, the systemd user environment and `mise env` in the home
  directory: no variable named `ANTHROPIC_*`, `CLAUDE_CODE_*`, `CCR_*` or
  `CLAUDE_CONFIG_DIR`.
- A running Claude Code session's environment (inherited by its Bash
  children, also for a shell-launched session with agent teams enabled): nine
  `CLAUDE_CODE_*` names that Claude Code sets itself; no `ANTHROPIC_*`,
  `CLAUDE_CONFIG_DIR`, `CCR_*`, `CLAUDE_CODE_USE_*` or
  `CLAUDE_CODE_CUSTOM_OAUTH_URL`. The only name matching a D11 refusal pattern
  is `CLAUDE_CODE_MESSAGING_TOKEN` (design D4, G3).
- `~/.claude/settings.json`: `apiKeyHelper` absent; its `env` block sets two
  names, neither matching a refusal pattern. No `settings.local.json` and no
  managed settings file.

## Repository facts (at `7a9fefb`) [repo]

- `register.js` `sample()` guard order and the `$.process.run` argv as listed
  in design Context; the mod uses no `Date`.
- `reporter.rs` `ClaudeReport::parse` destructures exactly four values;
  `digits()` accepts ASCII integers only; `report_claude` returns before any
  write at the `obs_seq` refusal and at the no-change check.
- Reporter state: explicit `--state`, else absolute `XDG_STATE_HOME`, else
  `<home>/.local/state`; collector state: `XDG_STATE_HOME` as given, else
  `~/.local/state`.
- `allowances.rs`: `account_key` uses only `observatory-codex-account-v1:`;
  `mapping()` rejects unknown keys and forces `window_seconds` 604800; the
  four-account cap covers all mappings; `public_row` hard-codes Codex.
- `State.js` `diagnostics()` emits only label and availability per allowance;
  `--allowances-probe` and `--identity-probe` return only mapped accounts.
- `native_process.rs` collector and peer fixtures set `CLAUDE_CONFIG_DIR`;
  the Claude report fixture at `:2267` expects exit 2 for five values and the
  one at `:2344` asserts no "account" or "rate" in the wire.
- `uninstall.sh` removes only the listed state file names.

## Implementation

_Pending: one section per task group, with commits, test counts and the
new-behaviour tests shown failing on `7a9fefb`._

## After

_Pending (task 5.1)._

## Review rounds

_Pending (task 5.2)._

## Live installed check

_Pending (task 5.3)._

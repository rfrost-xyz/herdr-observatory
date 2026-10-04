# Design

## Context

This change implements research decision D11 of the archived change
`2026-10-01-research-claude-code-parity` (Claude allowances and identity), as
revised by the user's change 4 decisions and by what change 3 shipped:

- **Decision 1 (accepted).** The Claude reporter and collector may read exactly
  three paths from `~/.claude.json`: `oauthAccount.accountUuid`,
  `oauthAccount.emailAddress` and `cachedUsageUtilization`. Nothing else is
  read, logged, persisted or retained. The whole file is skipped, attributing
  nothing, when `primaryApiKey` is present. `~/.claude.json` is provider-owned
  state, not an authentication file.
- **Decision 2 (accepted, confirmed at the 2026-10-04 plan gate).** Rate
  limits carry no account id, so the reporter attributes them to the account
  `~/.claude.json` names at report time, refusing whenever the environment or
  settings could select a different credential or API endpoint.
- **Decision 3 (accepted, confirmed at the 2026-10-04 plan gate).** The mod
  passes rate limits to its reporter as extra argv values: at most two windows, kind `five_hour`
  or `seven_day`, `percentUsed` (0 to 100, at most one decimal) and `resetsAt`
  as epoch seconds. Any `spend_limit` window means no attribution for that
  sample (strengthened in D2: no rate limits for that sample or any later one
  until `session.end`). Still argv-only, never awaited, never blocking.

D11 assumed the statusLine payload (research surface A). Change 3 shipped the
function-hook mod (surface B) instead, so the source here is the mod's
`session.measure` event. The installed Claude Code 2.1.287 types
(evidence.md [types]) give:

- `SessionMeasureInput` is `{context, rateLimits: SessionRateLimit[], cost?,
  changed: UsageUnit[]}`. `changed` is never empty; the first measurement names
  every unit it has a figure for. `rateLimits` is in `changed` when "a window
  moved a whole point, appeared or left, or the account's limit status changed".
- `SessionRateLimit` is `{kind, percentUsed, resetsAt?}`. `kind` is
  `five_hour`, `seven_day` or a gateway's `spend_limit`; `percentUsed` is 0 to
  100 with at most one decimal (past 100 only on an exceeded spend limit);
  `resetsAt` is an optional ISO 8601 string. `rateLimits` holds "the rate-limit
  windows the last API response reported".
- `cost?` is a `SessionCost`, `{usd: number}`: "US dollars, summed over every
  priced API response this session", absent only where the host keeps no
  ledger (the CLI always has one). `cost` is in `changed` when "the total
  grew".
- `session.measure` fires after each main-thread turn and when a rate-limit
  window moves a whole point, one at a time, with bursts folding into one.
- No measure or usage field carries an account id, email, sample time, window
  duration or limit status. There is no login event; `session.end` has a
  `logout` reason.

What change 3 left in place (code map, evidence.md [repo]):

- **Mod** `hooks/claude/anton-observatory/hooks/register.js`: `sample()` guards
  in order (runtime, `HERDR_ENV`, pane, session id, classic id match, window,
  start and classic dedupe, epoch clock, in-flight skip, `seq`), then
  `$.process.run([runtime, '--report', 'claude', pane, seq, id, window],
  {timeoutMs: 2000})`. No `Date`, no Node APIs.
- **Reporter** `reporter.rs`: `ClaudeReport::parse` takes exactly four values;
  `report_claude` runs the owner, mod receipt, single local socket host,
  `hook.lock`, `pane.get`, binding and `obs_seq` guards, then the no-change
  check and one 16-key v2 metadata write. Exit 0 means the bound window is in
  the pane's metadata; the mod confirms its key only on exit 0.
- **Collector**: Claude context percentage from replay over a bound window;
  peers ignore reporter metadata. Allowance rows come from
  `allowances::snapshot_at` every 2 s; mappings reject unknown keys;
  `public_row` hard-codes Codex; the four-account cap and `allowances.json`
  belong to the Codex shape that "Legacy allowance source compatibility" pins.
- **Identity**: `identity::refresh` (`--refresh-identities`) writes
  `.accounts.json` keyed by mapping id, which `PopupContent.qml` reads.

v2 pane metadata has no free keys (16 of 16), so rate limits cannot travel as
pane metadata. They go to a private per-account state file (D6).

## Goals / Non-Goals

**Goals:**

- A mapped local Claude account shows an allowance row with a `seven_day`
  pacing window while a local Claude Code session in a Herdr pane reports fresh
  rate limits, and its verified email after `--refresh-identities`.
- Every refusal, unknown or stale case leaves the row unavailable or a value
  unknown, never zero and never attributed to the wrong account knowingly.
- The window report from change 3 is unaffected by anything this change adds:
  same wire, same exit status meaning, same timing.

**Non-Goals:**

- Peer Claude allowances or identity. Claude is local-only.
- Scoped (per-model) weekly limits from the cache's `limits[]`. Their `kind`
  naming is unconfirmed (evidence.md [provider state]).
- Reset passes for Claude, plan names, organisation, spend or credit fields.
- `$.session.authorize()`, the credential file, the OAuth usage endpoint,
  `claude auth status`, headless `/usage`, transcript `credential_org` and
  `session_context`.
- Any presentation code change. Claude rows go through the provider-neutral
  card unchanged (D9).

## Architecture

- **`hooks/claude/anton-observatory/hooks/register.js`:** a freshness baseline
  (used values and the cost total) and the rate-limit tail on
  `session.measure` only (D2). Refreshed through the
  existing `--install-claude-mod`; receipt version 1 unchanged.
- **`src/reporter.rs`:** `ClaudeReport::parse` accepts 4, 7 or 10 values (D3).
  After the binding and `obs_seq` guards, the change 3 no-change check and
  metadata write run unchanged; then, when a tail is present and the window
  outcome is exit 0, it calls the attribution path (D4, D5) and the account
  state write (D6), still under `hook.lock`.
- **`src/claude_account.rs` (new):** the environment name rule, the
  `apiKeyHelper` presence read, three typed `~/.claude.json` extractions (one
  per consumer), the ISO 8601 parser, the account state file and the Claude row
  builder.
- **`src/allowances.rs`:** `mapping()` accepts `provider`; `snapshot_at` routes
  Claude mappings to the Claude row builder and keeps Codex cache and peer rows
  for Codex mappings only (D7).
- **`src/identity.rs`:** `refresh` adds the local Claude email for a mapped
  Claude key (D8).
- **`src/main.rs`:** `--claude-account-key` and `--claude-attribution-check`
  (D8); the local allowance worker
  reads the provider-state cache at most once per 60 s when a Claude mapping
  exists (D7).
- **`omarchy/herdr.observatory/uninstall.sh`:** removes `claude-allowances.json`
  with the other owned state files.
- **Tests and measurement:** D11.

## Decisions

### D1. Source and scope

- The only source of fresh Claude rate limits is the mod's `session.measure`
  event (`e.rateLimits`, `e.changed` and, for the cost path, `e.cost`). `session.start` and
  `classic.SessionStart` read `$.session.usage()`, which has no change signal
  and after a reload or `/resume` returns an old reading; they never send rate
  limits and never touch the freshness baseline.
- The second source is Claude Code's own `cachedUsageUtilization` in
  `~/.claude.json`, used by the collector only as an account-matched fallback
  with `fetchedAtMs` as its stamp (D7). Research and the change 4 probe found
  it over a day old while sessions were active, so it rarely passes the
  ten-minute rule.
- Only local panes report, as in change 3. Peers never install the mod, their
  `--allowances-probe` and `--identity-probe` carry no Claude data, and the
  local collector builds Claude rows only from local state (D7).

### D2. Mod: freshness and the rate-limit tail

**Source-time rule (re-derived for `session.measure`).** D11 stamped a window
when the statusLine showed a new `(used_percentage, resets_at)` value. Here the
engine already reports change through `changed`, but with three traps: the
first measurement names every unit (so an old reading after a reload or
`/resume` looks changed); `rateLimits` is also flagged when a window leaves
(which can be a reset with no API response behind it) or when the limit status
changes with the same values; and a context change proves nothing (rewinds and
compactions). The engine's `cost` unit, by contrast, grows only when a priced
API response arrives, and `rateLimits` is what the last response reported. The
user chose (G4, 2026-10-04) to stamp on either signal: a whole-point window
movement or a strict growth of the session's cost total.

**Cost tracking.** The mod reads `e.cost` only on `session.measure`. The
measurement's cost total `c` is `e.cost.usd` when `e.cost` is a non-null object
whose `usd` is a number with `Number.isFinite(usd)` and `usd >= 0`; in every
other case (`e.cost` absent, `null`, not an object, `usd` missing, not a
number, `NaN`, infinite or negative) `c` is `null`. Reading `e.cost` or `usd`
never throws into the hook: the read sits inside the same guarded block as the
rest of `sample()`. The cost total is held only in the mod's memory as part of
the baseline. It is never passed as argv, logged or written anywhere, so the
reporter requirement that the mod sends no costs still holds.

**Per-session baseline.** The baseline is one record per module load:
`{session, used: {five_hour, seven_day}, costUsd}`, where `session` is the
session id of the measurement that set it, each `used` value is that
measurement's `percentUsed` for the kind or absent, and `costUsd` is that
measurement's `c` (possibly `null`). An empty baseline means no measurement
has been seen since the module loaded or since the last `session.end`.

A measurement is **fresh** when rules 1 and 2 hold and at least one of the
paths A or B holds:

1. it is not the first measurement this module load has seen since it loaded
   or since the last `session.end` (the baseline is empty), and its session id
   equals the baseline's session id (a measurement for another id is treated as
   the first measurement);
2. `e.rateLimits` lists no window of kind `spend_limit`, and no earlier
   measurement in this module load since the last `session.end` listed one
   (once seen, `spend_limit` blocks every tail until `session.end`);

- **A. Window path (unchanged):** `e.changed` is an array that includes
  `'rateLimits'`, and at least one `five_hour` or `seven_day` window in
  `e.rateLimits` has a `percentUsed` different from the baseline's value for
  that kind, or is absent from the baseline (it appeared).
- **B. Cost path (G4):** `e.changed` is an array that includes `'cost'`, the
  measurement's `c` is not `null`, the baseline's `costUsd` is not `null`, and
  `c > costUsd` (strictly). Both conditions are required: `'cost'` in
  `changed` without a strictly larger total (equal, lower or unknown) is not
  fresh, and a larger total without `'cost'` in `changed` is not fresh either.

A fresh measurement sends a tail only when the tail construction below keeps at
least one window; a fresh measurement with no `five_hour` or `seven_day`
window sends the four-value run. Under path B the windows sent are the ones
`e.rateLimits` holds at that measurement, with their current used values and
reset times, even when none of them moved.

Not fresh: the first measurement after the module loads, after `session.end`
or for another session id, whatever `changed` names (including `cost` and
`rateLimits`); a rewind or compaction whose measurement shows no cost growth
(a rewind itself makes no priced call, so normally `cost` is not in `changed`
and the total is unchanged); a window that only left; a limit status
change with unchanged used values; a change of `resetsAt` alone; a measurement
whose only changed unit is `context`; and every start or classic reading. A
compaction that does make a priced call, so the total grows and `cost` is in
`changed`, is fresh under path B, because that response is a real API
response. More generally, path B looks only at strict cost growth with `cost`
in `changed`, whatever triggered the measurement: if a side or background
priced call lands between measurements and folds into a rewind's measurement,
that measurement is fresh too, and re-stamps the windows `e.rateLimits` holds
(Risks, [Cost without rate limits]). A mod test pins this accepted behaviour.
Path A compares the used value only: a reset time that moves without
a used value moving proves no new API response. A fresh tail still carries
each window's current `resetsAt`.

**Staleness under the combined rule.** `session.measure` fires after each
main-thread turn, and each turn makes at least one priced response, so path B
stamps once per turn while a session is used, independent of how fast the
windows move. Rows therefore stay fresh while a session completes a turn at
least every ten minutes, and turn stale after ten minutes without a turn or a
whole-point movement. Path A still stamps a whole-point movement that arrives
between turns. The residual risk (Risks, [Cost without rate limits]) is that a
priced response whose answer carried no rate-limit reading re-stamps the
previous response's values.

**Baseline ordering.** Freshness is evaluated, and the baseline replaced with
this measurement's session id, `five_hour` and `seven_day` used values and
cost total `c`, immediately after the session-id step and before the window
guard, the dedupe, the clock check and the in-flight skip. A skipped
measurement therefore still moves the baseline, so its evidence is lost rather
than replayed with a later value. `session.end` clears the baseline (session,
used values and `costUsd`), the `spend_limit` flag and `confirmed`. A
measurement for another session id replaces the whole baseline, cost total
included. Only `session.measure` reads or writes the baseline and sets the
flag; `session.start` and classic events never touch it.

**Evidence lost while a run is in flight.** Change 3 skips a sample while a
run started within 3 s is in flight and holds nothing. A fresh measurement
skipped this way is not raised again; the next measurement with cost growth or
a moved window stamps again. This change keeps that behaviour (decision 3
lists no further argv value) and records the loss, as the user accepted (G2).

**Tail construction**, when the measurement is fresh, for each of
`five_hour` and `seven_day` present in `e.rateLimits` (at most one of each; a
repeated kind drops the tail):

- `percentUsed` must be a finite number with `0 <= p <= 100`. Let
  `t = Math.round(p * 10)`; if `Math.abs(p * 10 - t) > 1e-6` the window is
  dropped. The text is built from the integer `t`: `String(t / 10)` when
  `t % 10 === 0`, otherwise `String((t - t % 10) / 10) + '.' + String(t % 10)`.
  So 23.5 gives `23.5`, 7 gives `7`, 100 gives `100`, and float noise never
  reaches argv.
- `resetsAt` must be a string matching
  `^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$`
  with month 1 to 12, a day valid for that month (leap years included), hour
  0 to 23, minute and second 0 to 59, offset hours 0 to 23 and minutes 0 to 59.
  It is converted with integer days-from-civil arithmetic, the offset
  subtracted and the fraction truncated, so the mod still has no `Date`
  dependence (change 3 D2). An absent, malformed or out-of-range value drops
  the window.
- The tail is built after `seq` is computed, and the reset bounds use that
  same `seq`, exactly as the reporter does (D3), in exact integers: with
  `S = (seq - seq % 1000000) / 1000000` (the whole seconds of `seq`, exact
  because `seq` is a safe integer), the epoch-second reset `r` is kept only
  when `S < r <= S + D + 3600`, where `D` is the window's duration (so
  `S + 21600` for `five_hour`, `S + 608400` for `seven_day`). Because `r` is an
  integer, `S < r` is the same as `r > seq / 1e6`; no product such as
  `r * 1e6` is formed, so nothing exceeds 2^53. Otherwise the window is
  dropped. Using `seq` rather than the clock matters
  when `lastSeq + 1` dominates: the mod never sends a tail the reporter would
  reject, and a node test pins the boundary.
- The tail is `kind used resets` for each kept window, `five_hour` first. If
  no window is kept, no tail is sent and the run is the change 3 four-value
  run.

The tail is appended to the change 3 argv in the same synchronous
`$.process.run` call, so the run is still started only inside the hook, never
awaited, and every promise still ends in `.catch`. The window guard still
applies first: a measurement without a valid window sends nothing at all. The
static source scan in `tests/test_claude_mod.mjs` keeps rejecting `Date`,
`import`, timer globals and Node APIs.

### D3. Reporter argv grammar and validation

```
anton-runtime [--root R] [--state S] --report claude <pane> <seq> <session-id> <window> [<kind> <used> <resets-at> [<kind> <used> <resets-at>]]
```

- The values after `claude` are still taken verbatim, with no option parsing.
  Their count must be 4, 7 or 10; any other count exits 2.
- The first four keep their change 3 rules exactly.
- `<kind>` is `five_hour` or `seven_day`; the two triples must have different
  kinds. Order is free, because windows are selected by name.
- `<used>` matches `^(100|(0|[1-9][0-9]?)(\.[1-9])?)$`: 0 to 100, at most one
  decimal, no sign, exponent, leading zero, trailing `.0` or space. It is
  stored as an `f64` with that one decimal.
- `<resets-at>` is 1 to 11 ASCII digits parsed as `u64` `r`, kept only when
  `S < r <= S + D + 3600` with `S = seq / 1_000_000` in integer division and
  `D` the kind's duration: the same exact rule as the mod (D2).
- Any failure exits 2 before any file or socket access, as for the change 3
  values. A malformed tail therefore also loses that run's window report; the
  mod never sends one (D2), and the next run reports the window.

**Exit statuses** keep their change 3 meaning: 0 when the bound window is in
the pane's metadata, 2 for invalid arguments, 3 when the window report does
not apply, 1 for any other error. Attribution refusal and account state
outcomes never change the status (D6), so the mod's `confirmed` key logic is
unchanged.

**Compatibility.** A four-value run is byte-for-byte the change 3 behaviour.
The change 3 regression fixtures that pass five values
(`claude_report_rejects_invalid_arguments_without_socket_access` in
`native_process.rs`, the `reporter.rs` five-value parse case) change
intentionally: five and six values still exit 2, seven valid values now
succeed. See D10 for mixed mod and runtime versions.

### D4. Attribution and refusal

Attribution runs only when a tail is present, and only after guards 1 to 6 of
change 3 (owner, mod receipt, single local socket host, `hook.lock`,
`pane.get`, binding to this Claude session id, `obs_seq` older than `seq`).
The account step runs after the window outcome is decided: after a successful
metadata write or the no-change return, still under `hook.lock`. It cannot
change a status already decided, and an unchanged window still allows an
account write. When the metadata write fails (exit 1) the account step does
not run. Its lock-hold time is measured with the 4 MiB fixture (tasks 1.1 and
5.1); the budget is an absolute `hook.lock` hold time of at most 100 ms for a
ten-value run in the release build (the whole hold, not the extra over a
four-value run), well under the 400 ms that sibling reporters wait for the
lock. A miss returns the plan to the gate. The steps, in order, each refusing
with no account write:

1. **Environment names.** The reporter is a child of Claude Code: the mod
   calls `$.process.run` without `env`, and the types say a run "inherits" the
   environment of the Claude Code process, the same one every Bash child and
   MCP server gets (evidence.md [types]). So the reporter's own
   `std::env::vars_os()` is the session's environment. Only names are read. It
   refuses when any name is not valid UTF-8, or matches:
   - `ANTHROPIC_*KEY*`, `ANTHROPIC_*TOKEN*`, `ANTHROPIC_CUSTOM_HEADERS`;
   - `ANTHROPIC_BASE_URL` (exact name, G1);
   - `CLAUDE_CODE_*TOKEN*`, `CLAUDE_CODE_*_FILE_DESCRIPTOR`, `CLAUDE_CODE_HOST_*`;
   - `CCR_OAUTH_TOKEN_FILE`, `CLAUDE_CODE_CUSTOM_OAUTH_URL`;
   - `CLAUDE_CODE_USE_*`;

   unless the name is on the closed exemption list, which holds exactly
   `CLAUDE_CODE_MESSAGING_TOKEN`. Claude Code sets that name for its own
   children (seen in Bash children of a shell-launched session with agent teams
   enabled, evidence.md [environment]); a literal D11 rule would refuse every
   sample in such a session. The user consented to this single exemption on
   2026-10-04 (G3); no other name can be added to the list without a new
   decision. The patterns are matched as ASCII globs where `*` matches zero or
   more characters, case-sensitively, so `ANTHROPIC_KEY` matches
   `ANTHROPIC_*KEY*`. `ANTHROPIC_BASE_URL` has no wildcard: the name being
   set refuses, with any value, including an empty one, because a proxy or
   gateway at another endpoint would report windows that are not the profile
   account's (G1, 2026-10-04). Like every step 1 rule it reads the name only,
   never the value.
2. **Configuration directory.** `CLAUDE_CONFIG_DIR` set to any value, even
   empty or naming `~/.claude`, refuses. This is stricter than the change 3
   installer rule on purpose: the collector reads a fixed `$HOME/.claude.json`
   with the shell's environment and cannot follow a session's config directory.
3. **Legacy configuration.** `<home>/.claude/.config.json` existing in any form
   (checked with `symlink_metadata`, no read) refuses, because Claude Code then
   uses it instead of `~/.claude.json`.
4. **`apiKeyHelper`.** `<home>/.claude/settings.json`: absent passes. Present,
   it is read with `read_owned` (no-follow, owner-checked, not required to be
   private, at most 1 MiB) and parsed into a struct whose only field is the
   presence of `apiKeyHelper`; every other key is skipped with `IgnoredAny`.
   Present (any value) refuses; unreadable, unsafe, oversized or malformed
   refuses. Nothing else in the file is extracted, including `env`. Project,
   local and managed settings are not read (Risks; not built, G8).
5. **Provider state.** `<home>/.claude.json` through the reporter extraction
   (D5). `primaryApiKey` present refuses; an invalid or missing account id
   refuses.
6. **Session memory** (D6): a session already attributed to another account,
   or marked refused, refuses.

`<home>` is the reporter's resolved home from change 3 (absolute `HOME`, else
the password database).

`CLAUDE_CODE_CUSTOM_OAUTH_URL` and the legacy file both change which file
Claude Code uses, the variable patterns cover credentials selected from the
environment, and `ANTHROPIC_BASE_URL` covers an API endpoint selected from the
environment. The collector, identity refresh and `--claude-account-key` apply
only steps 2 and 3 (D7, D8); G1 concerns attribution, so `ANTHROPIC_BASE_URL`
does not stop them. A token read from a well-known path with no variable, a gateway
session without `spend_limit`, and `apiKeyHelper` in project, local or managed
settings remain residual risks (Risks).

### D5. Reading `~/.claude.json`

- `read_owned(path, 4 MiB, private = true)`: opened without following a final
  symlink, owned by the effective user, no group or other mode bits, at most
  4 MiB. The probed file is a regular 0600 file of about 165 KB
  (evidence.md [provider state]). Any failure means "no account".
- It is parsed with `serde_json::from_slice` into a typed struct per consumer.
  Unlisted keys are skipped with `IgnoredAny`, so their values are never held
  in a `serde_json::Value`, logged or stored. `primaryApiKey` is detected by
  key presence alone, with any value, including `null`: presence is a
  `#[serde(default)]` newtype whose `Deserialize` calls
  `IgnoredAny::deserialize` and returns `true`. `Option<IgnoredAny>` is not
  used, because it maps `null` to absent.
- Every parse or validation error from `~/.claude.json` or `settings.json` is
  replaced by a fixed message before it leaves `claude_account.rs`; serde error
  text, which can embed the offending value, is never formatted.
  - **Reporter:** `oauthAccount.accountUuid` and `primaryApiKey` presence.
  - **Collector:** `oauthAccount.accountUuid`, `primaryApiKey` presence and
    `cachedUsageUtilization.{accountUuid, fetchedAtMs,
    utilization.five_hour.{utilization, resets_at},
    utilization.seven_day.{utilization, resets_at}}`. `limits[]`, `spend`,
    `extra_usage`, dollar fields and every other key are skipped.
  - **Identity refresh, key command and attribution check:**
    `oauthAccount.accountUuid`, `oauthAccount.emailAddress` (identity refresh
    only) and `primaryApiKey` presence.
- The readers are exactly the Claude reporter, the collector, identity
  refresh, `--claude-account-key` and `--claude-attribution-check`. Decision 1
  names only the reporter and collector; the user consented on 2026-10-04 (G7)
  to the last three reading the same three allowlisted paths.
- The account id must be a string of 1 to 256 characters with no control
  character. The key is `sha256("observatory-claude-account-v1:" + id)` as 64
  lowercase hex characters. The id itself is dropped once hashed.
- The reporter parses the file only when a tail is present, which happens only
  on fresh evidence (D2). Under the G4 cost path that is about once per
  main-thread turn while a session is used, so the ten-value run is the
  per-turn path and the D4 absolute lock-hold budget applies to every such
  run. Its cost is measured (D11).

### D6. Per-account state file

- **Path:** `<state>/claude-allowances.json`, beside `allowances.json`, where
  `<state>` is the reporter's resolved state directory (change 3 D3).
  `allowances.json` and the `--allowances-probe` shape stay Codex-only.
- **Format (version 1):**

  ```json
  {"version":1,
   "accounts":[{"account_key":"<64 hex>","windows":{
     "five_hour":{"used_percent":12.5,"resets_at":1800003600,"sampled_at":1800000000.123456},
     "seven_day":{"used_percent":40,"resets_at":1800400000,"sampled_at":1800000000.123456}}}],
   "sessions":[{"session":"<64 hex>","account_key":"<64 hex>","at":1800000000.123456}]}
  ```

  `sampled_at` is the report's `seq / 1e6`, the mod's clock at the fresh
  measurement. `session` is `sha256("observatory-claude-session-v1:" + id)`;
  the raw session id is never stored. A session entry with `"account_key":null`
  is refused for good. `at` is the latest report time for that session,
  renewed on every report that reaches the account step.
- **Bounds:** at most 4 accounts (the least recently stamped is evicted), at
  most 32 session entries (entries whose `at` is older than 24 hours dropped;
  then attributed entries evicted oldest `at` first, refused entries only
  after every attributed one), at most 16,384 bytes serialised. A write that
  would exceed the byte bound is refused.
- **Merge, newest stamp wins:** for each window in the tail, the stored window
  for that account and kind is replaced only when the new `sampled_at` is
  greater. A window absent from the tail keeps its stored value and stamp.
  Every window in a fresh tail gets the same stamp, because all come from the
  same last response; this departs from D11's per-window value-change rule, as
  the user accepted (G5).
- **Session attribution:** the first attributed report for a session records
  its account. A later report for that session that resolves another account
  sets the entry's account to `null` and writes nothing else; every later
  report for that session is refused. D11 instead resumed after the next
  change, but with this source a sample after a `/login` elsewhere cannot be
  told apart from one under the new account, so the stricter rule is chosen,
  as the user accepted (G6). New sessions attribute normally.
- **Write:** under the `hook.lock` that `report_claude` already holds, so Claude
  and Pi reporters on the same state directory are serialised and no second
  lock is needed. The state directory's owner is checked as `receive()` does.
  The existing file is read with `read_owned(LIMIT, private = true)`; a missing
  file starts empty; an owned file that is malformed or the wrong version is
  replaced; an unsafe one (symlink, other owner, loose mode) refuses the write.
  The result is written with `atomic_owned_write`. The collector only reads
  it, without a lock, relying on the atomic rename (D7).
- **Errors** in attribution or the write are silent (no output) and never
  change the exit status or stop the window report. A lost sample is
  recovered by the next fresh one.

### D7. Collector: Claude rows beside Codex rows

- **Mappings.** `mapping()` accepts an optional `provider`, `codex` or
  `claude`; absent means `codex`, so every existing configuration is
  unchanged. The bare `"Personal"`/`"Work"` forms are Codex. A Claude mapping
  takes the same `id`, `label` and `category` rules; `window_seconds` must be
  absent or 604800 (it is ignored for Claude). The four-account cap stays
  shared across providers, and ids stay unique across providers, so the
  id-keyed `.accounts.json` cannot collide. Unknown providers are rejected.
- **Provider state read.** When at least one Claude mapping exists, the local
  allowance worker reads `$HOME/.claude.json` with the collector extraction
  (D5) at start and then at most once per 60 s, keeping only the hashed key
  and the two cache windows in memory; the raw account id is dropped once
  hashed. `$HOME` is the collector's own. The collector does not follow
  `CLAUDE_CONFIG_DIR`: it applies D4 steps 2 and 3 against its own environment
  and home, so `CLAUDE_CONFIG_DIR` set or a legacy `.config.json` present
  means no cache and no key. `primaryApiKey` present means the same.
- **Cache fallback.** Used only when `cachedUsageUtilization.accountUuid`
  equals `oauthAccount.accountUuid`, the hashed key is a Claude mapping, and
  `fetchedAtMs / 1000` is within the existing `[-1, 600]` second window. Each
  window's `utilization` is read as a percentage when it is 0 or in (1, 100],
  rounded half up to one decimal (so the D3 one-decimal grammar applies only
  to reporter-supplied values); a value in (0, 1] is ambiguous between a
  fraction and a percentage (the probe saw only zeros) and leaves that window
  unknown. `resets_at` is parsed by the same ISO 8601 grammar as D2,
  implemented in Rust; a malformed value drops the window.
- **Reading the state file.** On each snapshot (every 2 s) the collector reads
  `claude-allowances.json` from its own state directory with
  `read_owned(16 KiB, private = true)` (no-follow, owned by the effective user,
  no group or other mode bits). Any read failure, a version other than 1, an
  unknown key, more than 4 accounts or 32 sessions, or a malformed structure
  rejects the whole file and is treated as no state (the row is unavailable
  unless the cache supplies it). Within an accepted file, a window whose
  `used_percent` breaks the D3 grammar (0 to 100, at most one decimal), whose
  `resets_at` is not an integer in the D3 digit range or whose `sampled_at` is
  not finite and positive is dropped. Session entries are not read by the
  collector.
- **Row.** Per window, the newer of the reporter state and the cache wins by
  stamp, whichever source has it. `five_hour` becomes `{kind:"five_hour", label:"5-hour",
  duration_s:18000, pacing:false}` and `seven_day` becomes
  `{kind:"seven_day", label:"7-day", duration_s:604800, pacing:true}`. A window
  whose reset has passed keeps its stamp with `used_percent` and `resets_at`
  null. The row's `sampled_at` is the oldest stamp among its windows; the row
  is available only when that is within `[-1, 600]` seconds of now, otherwise
  it is unavailable with no windows (D11's rule, so one stale window makes the
  row stale). `provider` is `claude`, `provider_label` `Claude`; `plan`,
  `status_text`, `reset_count` and `reset_expires_at` are null.
- **Merge with Codex.** `snapshot_at` keeps one row per configured account in
  key order. Codex cache rows and peer rows are matched to Codex mappings only,
  so a peer row can never fill a Claude row. Claude rows come only from the
  local state file and the in-memory cache.
- **State directory.** If the session's `XDG_STATE_HOME` or `HOME` differs
  from the collector's, the reporter writes elsewhere and the row stays
  unavailable: honest, recorded under Risks, with a fixture.

### D8. Identity and the account key

- **Email.** `identity::refresh` (`--refresh-identities`) reads
  `$HOME/.claude.json` with the identity extraction (D5) only when at least
  one Claude mapping exists, and only after D4 steps 2 and 3 pass against its
  own environment and home. When the hashed id is a Claude mapping and the
  email passes the existing `email()` check, it is added to `.accounts.json`
  under the mapping id. No peer is asked for Claude identity. A file with
  `primaryApiKey`, an unmapped key or an invalid email adds nothing. The email
  never enters snapshots, diagnostics or the account state file.
- **Provider check on identity rows.** The Codex RPC row and peer
  `--identity-probe` rows are matched to Codex mappings only (`identity::mapped`
  today checks no provider). A Claude email comes only from the local provider
  state, so a peer or Codex RPC row carrying a Claude mapping's key stores
  nothing and cannot overwrite the local Claude email.
- **Key discovery.** D11 said the user obtains the key "from diagnostics, as
  for Codex", but the diagnostics JSON carries only labels and availability
  and Codex keys are derived by hand. A new command,
  `anton-runtime --claude-account-key`, reads `$HOME/.claude.json` with the
  identity extraction minus the email and prints only the 64-character key and
  a newline, exit 0. It first applies D4 steps 2 and 3 against its own
  environment and home. With either refusing, `primaryApiKey`, no file or no
  valid id it prints nothing and exits 3. It never prints the id or email,
  needs no owner guard or configuration, and is documented in the plugin README
  with the mapping format. The popover diagnostics are unchanged. The user
  accepted this command and its reads (G7).
- **Attribution check.** Refusals are silent in the reporter, so the live check
  could not otherwise find why attribution fails. A private dry run,
  `anton-runtime --claude-attribution-check`, runs D4 steps 1 to 5 against its
  own environment and home and prints only the first refusing step's name
  (`environment`, `config-dir`, `legacy-config`, `api-key-helper`,
  `provider-state`) or `ok`, then a newline; exit 0 for `ok`, 3 otherwise. For
  the environment step it also prints each matching variable name on its own
  line, never a value, and it prints exempt matches too, marked `exempt`, so
  the live check can record whether `CLAUDE_CODE_MESSAGING_TOKEN` is present.
  `ANTHROPIC_BASE_URL`, when set, is printed under `environment` like any other
  matching name. The names follow the first line whatever the outcome
  (so an `ok` run still shows an exempt name), sorted, one per line, an
  exempt one as `<name> exempt`; a name that is not valid UTF-8 adds one
  line `non-utf8-name` instead of itself. The home is resolved as the
  reporter's (an absolute `HOME`, else the password database); without one
  the provider state cannot be located, so after steps 1 and 2 the step is
  `provider-state`. It prints no id, key, email or value, writes nothing and
  needs no owner guard. Run from a Bash tool call inside a Claude Code
  session, it inherits that session's environment, as the reporter does. It
  reads the same paths as the reporter, as the user accepted (G7).

### D9. Presentation

- No QML or `State.js` change is planned. Claude rows go through
  `allowanceView`, which knows no provider, so `providerCoupling()` in the
  measurement harness stays unchanged.
- Provider group "Claude" appears only when a Claude mapping exists. The card's
  balance and pace come from the single pacing window, `seven_day`; a
  `five_hour`-only row shows no balance or pace. The non-pacing `five_hour`
  window is carried in the row but no current card element shows it.
- The verified email replaces the label after `--refresh-identities`;
  concealment aliases cover Claude rows as they cover Codex rows.
- `AllowanceCard.qml` shows its unknown placeholder before "resets" when the
  reset count is null. Claude has no reset passes, so a Claude card shows that
  placeholder, the existing unknown rendering. Changing it would need either a provider branch (forbidden) or a
  new provider-neutral row field; that is left as open question 4.

### D10. Compatibility

| Case | Result |
|---|---|
| New runtime, change 3 mod | Four values: window reports as before; no rate limits, so Claude rows stay unavailable until `--install-claude-mod` refreshes the mod. |
| New mod, change 3 runtime | Seven or ten values exit 2 on the old parser, so fresh runs lose their window report; four-value runs still report. Fixed by updating the runtime. The installer updates both together; this arises only from a manual runtime downgrade. Within this branch, the mod change (task 3.1) is committed only after the parser change (task 2.1), so no commit pairs a seven- or ten-value mod with the four-value parser. |
| Configuration with a Claude mapping, older runtime | `mapping()` rejects the `provider` key, so the allowances configuration is invalid. Rollback means removing Claude mappings first (Migration plan). |
| Older peer, new local | Peers carry no Claude data either way. Codex rows unchanged. |
| New peer, older local | The peer's probes are unchanged in shape. |
| Codex and Pi | Codex rows, `allowances.json`, the probe and Pi reports unchanged; existing fixtures stay green. |
| Pane metadata | The 16-key wire is unchanged; the guard in `claude_report_writes_the_bound_window_once_and_repeats_read_only` (`native_process.rs`) that no "account" or "rate" key appears stays. |

### D11. Tests and measurement

All fixtures are synthetic: temporary homes holding a synthetic
`.claude.json` (fake uuid `00000000-0000-4000-8000-000000000001`, email
`fixture@example.invalid`), synthetic settings files, synthetic Herdr sockets
and session ids. No test reads or writes the real `~/.claude`, `~/.claude.json`
or `~/.claude/settings.json`. Process fixtures use `env_clear()` with an
explicit `HOME`, `PATH` and the variables under test. The default
`native_process.rs` fixture environment sets `CLAUDE_CONFIG_DIR` (lines 149
and 204); Claude row, identity and key-command fixtures drop it, and one
fixture keeps it and asserts no cache, no key and no email. No fixture calls
`std::env::set_var`: the environment rule takes the name list as a parameter.

**New behaviour** tests must fail on `7a9fefb` (the change 3 merge). A unit
test in the new `claude_account.rs` cannot compile on `7a9fefb`; that counts
as failing and is recorded by test name, and every collector, reporter and
command case also runs as a process fixture, which runs and fails on
`7a9fefb`. **Regression guards** are existing tests only; they must pass on
both `7a9fefb` and the change. No new test is a regression guard: a new
no-change assertion (Codex output, four-value runs, probe shape) sits in a
test that also asserts new behaviour, as the mod tests do below.

On `7a9fefb` any configuration holding a `provider` key is invalid, so a
negative assertion over a configuration with a Claude mapping ("no Claude
data", "nothing stored", "rejected", "no cache", "unchanged output") passes
there trivially. Every such fixture also asserts something that needs the new
configuration to be accepted: exit 0, the Codex row or email present, or the
Claude row present with its identity. Every rejection case is paired, in the
same test, with an accepted neighbour that differs only in the rejected
property.

New behaviour:

- **Mod (`tests/test_claude_mod.mjs`):** the old mod never sends a tail, so a
  test that only asserts "no tail" would pass on `7a9fefb`. Every test that
  asserts no tail, a dropped window or the four-value run, whether the
  measurement is fresh or not, also asserts, in the same test and session, the
  exact tail of another measurement, which fails on `7a9fefb` and, for a
  not-fresh case, proves it moved the baseline as specified. The check that
  the cost total never appears in argv sits inside a test that asserts a
  tail. Cases:
  fresh on a moved or appeared window (path A); **cost growth with unchanged
  percentages is fresh**: a measurement with `changed: ['context','cost']`,
  `cost.usd` strictly above the previous measurement's and both windows at
  the previous used values sends both windows with their current used values
  and reset times; **rewind is not fresh**: `changed: ['context']` with the
  same `cost.usd` and windows sends no tail, then a cost-growth measurement
  sends one; **a rewind-shaped measurement with cost growth is fresh**:
  `changed: ['context','cost']`, a context drop and a strictly larger
  `cost.usd` (a side priced call folded into the rewind's measurement) sends
  the tail, pinning the accepted behaviour of D2 path B; **compaction without
  cost growth is not fresh**: a context drop
  with `cost.usd` unchanged sends no tail, also with `'cost'` in `changed` but
  an equal or lower total, then a cost-growth measurement sends one; **first
  measurement after load or `session.end` is not fresh**, even with
  `changed: ['context','rateLimits','cost']` and a non-zero total, for a fresh
  load and for a measurement right after `session.end`, then a second
  measurement with cost growth sends a tail; a larger total without `'cost'`
  in `changed` not fresh; `e.cost` absent, `null`, `usd` missing, a string,
  `NaN`, infinite or negative on the current or the baseline measurement not
  fresh by path B (then a measurement with valid growth over a valid baseline
  sends a tail); cost growth with a `spend_limit` window sending no tail, and
  no tail after it until `session.end`, then a tail after a new session's
  second measurement; cost growth with no `five_hour` or `seven_day` window
  sending the four-value run; cost growth in a measurement for another
  session id treated as the first; the cost total never appearing in argv;
  not fresh on `changed` without `rateLimits` or `cost`, on a window that only
  left, on a status-only change, on `spend_limit` present (tail absent, window
  run still made), on start and classic events (and the baseline untouched by
  them); a skipped in-flight measurement with cost growth moves the baseline
  (used values and total) and is not replayed; percent text
  for 0, 7, 23.5, 99.9, 100, 0.7 and noisy values; ISO conversion against a
  `Date.parse` oracle in the test for `Z`, numeric offsets, fractions, leap
  days, and rejection of invalid dates, hours, offsets and shapes; past and
  too-distant resets dropped, with the bounds taken from the sent `seq` (the
  first whole second after `seq / 1e6` kept, any earlier second dropped, the
  last second inside `S + D + 3600` kept and the next dropped, also when
  `lastSeq + 1` exceeds the clock); a change of `resetsAt` alone not fresh; a
  measurement for another session id treated as the first; `spend_limit` in
  one measurement blocking the tail of a later measurement without it until
  `session.end`; repeated kinds drop the tail; argv order and count; the source
  scan still finds no `Date`. The fixture's measure builder
  uses an array for `rateLimits`, a non-empty `changed` and a `cost` of the
  `SessionCost` shape `{usd}` unless a case removes or corrupts it.
- **Reporter (`reporter.rs`, `tests/native_process.rs`):** parse of 4, 7 and 10
  values in one test (the four-value result asserted unchanged beside the
  seven- and ten-value results) and rejection of 5, 6, 8, 9 and 11 values, unknown and repeated kinds,
  used values `-1`, `100.1`, `1.25`, `01`, `5.0`, `1e1`, `+5`, resets in the
  past, at `seq`, beyond the bound and with 12 digits, all with no socket
  access; the environment rule over each pattern, the exemption, a non-exempt
  `CLAUDE_CODE_*TOKEN*` beside the exempt one, and a non-UTF-8 name;
  `ANTHROPIC_BASE_URL` set to a URL carrying the `PRIVATE` marker and set
  empty, each refusing (unit test on the name rule, and a process fixture
  showing exit 0, the window bound, no `claude-allowances.json` written and
  the marker absent from stdout, stderr and the state directory), and a name
  that only contains it, such as `ANTHROPIC_BASE_URL_X`, not matching that
  exact rule;
  `CLAUDE_CONFIG_DIR` set empty, to `~/.claude` and elsewhere; legacy config
  as a file and as a dangling symlink; `apiKeyHelper` present, `null`, absent,
  settings missing, symlinked, oversized and malformed; `.claude.json` with
  `primaryApiKey` (string and `null`), missing account, invalid id, symlink,
  other owner (where the test can arrange it), mode 0644 and over 4 MiB; a
  mistyped `accountUuid` and `fetchedAtMs` and a mistyped `apiKeyHelper`
  value carrying the `PRIVATE` marker, with stderr and stdout searched for it;
  a success writing the state file with the expected stamps; newest-stamp-wins;
  a late older report using two panes bound to two sessions on one account,
  the older `seq` arriving second (one pane would stop at the `obs_seq`
  guard); a window absent from a tail kept; four-account and 32-session
  eviction with refused entries evicted last, `at` renewed on each report,
  24-hour expiry and the byte bound; the account switch marking the session
  refused for later reports; an unchanged window still writing account state;
  a failed metadata write (exit 1) writing no account state; every refusal
  still exiting 0 with the window bound; the pane wire byte-identical to
  change 3 with a tail present; a concurrent Pi report on the same state
  directory while a ten-value Claude run with the 4 MiB fixture holds
  `hook.lock`, both completing within their lock waits.
- **Collector (`claude_account.rs`, `allowances.rs`, `identity.rs`, `main.rs`):**
  mapping with and without `provider` in one configuration, the Codex
  mapping's output asserted unchanged beside the accepted Claude mapping;
  an unknown provider rejected beside the same configuration with `claude`
  accepted; the four-account cap shared across providers (two Codex and two
  Claude mappings accepted, a fifth mapping of either provider rejected); ids
  unique across providers (distinct ids accepted, a Claude mapping reusing a
  Codex id rejected); a Claude mapping's `window_seconds` absent or 604800
  accepted and any other value rejected; a state file holding a mapped and an
  unmapped Claude account key giving the mapped row available and no row,
  label or key for the unmapped one; rows for both windows, five_hour only, one fresh and one
  stale window, past reset, no state (unavailable), cache fallback matched and
  fresh, cache for another account, stale and future cache, ambiguous cache
  scale (0.5 and 1 unknown, 0 and 1.5 read), cache `resets_at` with offset and
  fraction, `primaryApiKey` skipping the cache; newer reporter window over
  older cache and the reverse; a peer row carrying a Claude mapping's key
  ignored while the Claude row is available from local state and the peer's
  Codex row is present; **peer side carries no Claude data**: a process fixture
  runs `--allowances-probe` and `--identity-probe` on a runtime whose
  configuration holds a Claude mapping beside a Codex mapping, with a
  `claude-allowances.json` holding that Claude account and a synthetic
  `.claude.json` in its home, and asserts exit 0, the Codex row or identity
  present and no Claude key, row, window, uuid or email in either output;
  **Codex cache untouched**: with a Claude row available in the snapshot,
  `allowances.json` and the `--allowances-probe` output hold only Codex rows in
  their existing shape (both fail on `7a9fefb`, where the Claude mapping makes
  the configuration invalid); a mismatched state directory leaving the row unavailable; the
  state file malformed, oversized, symlinked, at mode 0644, of another
  version, with an unknown key, or holding a used value `12.25`, each giving
  no state or a dropped window; `CLAUDE_CONFIG_DIR` set or a legacy
  `.config.json` present in the collector's environment giving no cache
  while the Codex rows stay available;
  Claude email added only for a mapped key and never with `primaryApiKey`;
  identity refresh without a Claude mapping never calling the provider-state
  reader (a unit test on that gate); a peer or Codex RPC identity
  row carrying a Claude mapping's key storing nothing for that key while the
  Codex email and the local Claude email are stored;
  `--claude-account-key` printing only 65 bytes and exit 0, and nothing with
  exit 3 for `primaryApiKey`, a missing file, an invalid id,
  `CLAUDE_CONFIG_DIR` or a legacy file; `--claude-attribution-check` printing
  each step name and `ok`, matching variable names with exempt ones marked,
  `environment` and the name `ANTHROPIC_BASE_URL` (never its value) when it
  is set, and never a value, uuid, key or email; `ANTHROPIC_BASE_URL` set in
  the collector's, identity refresh's and key command's environment leaving
  the Claude row available, the Claude email stored and the key printed; no uuid or email
  bytes in any snapshot, state file, probe output or stderr (a literal
  `PRIVATE` marker in unlisted keys and the fixture uuid and email searched
  for).
Regression guards (existing tests only):

- Change 3 window fixtures with four values, the wire constants, the no
  "account"/"rate" wire check, Pi report fixtures, Codex allowance contract
  tests, legacy cache and peer fixtures, `providerCoupling()` unchanged,
  `tst_popup.qml` provider-neutral rows and the diagnostics oracle.
- **State.js and QML.** D9 plans no presentation change, so a Claude-shaped
  row behaves the same on `7a9fefb`, and a new test for it could not fail
  there. These cases are therefore added as further assertions inside the
  existing provider-neutral tests (`providers group any configured accounts
  in configured order` and `mapped weekly allowance preserves zero and rejects
  expired reset` in `tests/test_omarchy_state.cjs`, and the existing allowance
  cases in `tst_popup.qml`), with no new test function, and count as
  regression guards: a Claude row with `seven_day` pacing shows a balance; a
  `five_hour`-only Claude row shows none; an unavailable Claude row; email
  lookup by mapping id and concealment covering it.

**Measurement.** `tests/measure_anton_popover.mjs` gains an additive
`claude_allowance` section, existing sections unchanged: reporter wall time,
CPU and `hook.lock` hold time for a four-value run and a ten-value run with a
synthetic 165 KB and a 4 MiB `.claude.json` (the ten-value run must pass guards
1 to 6, so the fixture creates the owner marker, a mod receipt through
`--install-claude-mod` in the temporary home and a fake socket answering
`pane.get` with a bound `herdr:claude` session and `pane.report_metadata`; it
asserts exit 0 and that the state file was written, otherwise the metric is
null); collector snapshot time with one Claude mapping beside
Codex mappings; Claude row count, available count and snapshot bytes. Baseline
on `7a9fefb` reports the new metrics as null where the feature is absent.
Under the G4 cost path the ten-value run is the expected per-turn path in a
used session, so its wall time, CPU and lock hold are reported as per-turn
costs, and the D4 budget (an absolute ten-value `hook.lock` hold of at most
100 ms, task 5.1) is judged against each such run.

### D12. Spec, AGENTS.md and README wording

The delta specs hold the requirement text. OpenSpec deltas cannot change a
spec's Purpose, so task 5.4 edits these by hand at sync:

- `account-allowances` Purpose. Old: "Provide passive, account-bound
  visibility of Codex subscription allowances and reset passes across the
  existing private fleet." New: "Provide passive, account-bound visibility of
  Codex subscription allowances and reset passes across the existing private
  fleet, and of local Claude Code subscription rate limits."
- `omarchy-companion` Purpose. Old: "...for current Herdr threads, Codex
  allowance and fleet readings." New: "...for current Herdr threads, Codex and
  Claude allowance and fleet readings."

AGENTS.md (task 1.3, before any implementation commit, so no commit
contradicts AGENTS.md; not edited during planning):

- Boundaries, reporter line. Old: "The Claude Code mod only observes events,
  passes each on unchanged, never waits on its reporter and sends only the
  pane, sequence, session id and window as argv; its reporter writes one bound
  window report for agent `claude` and nothing else." New: "The Claude Code mod
  only observes events, passes each on unchanged, never waits on its reporter
  and sends only the pane, sequence, session id, window and, from a measurement
  with fresh evidence (a rate-limit window that moved or appeared, or a grown
  session cost total), at most two `five_hour`/`seven_day` windows as argv,
  never the cost itself; its reporter writes one bound window report for agent `claude` and,
  when attribution is allowed, those windows to the private Claude account
  state file, and nothing else."
- Boundaries, authentication line. Old: "Never parse authentication files.
  Account observations use native read-only Codex account RPCs. No
  login/reset/redemption mutation." New: "Never parse authentication files.
  `~/.claude.json` is provider-owned state, not an authentication file: it is
  read only by the Claude reporter, the collector, identity refresh,
  `--claude-account-key` and `--claude-attribution-check`, which read only
  `oauthAccount.accountUuid`, `oauthAccount.emailAddress` and
  `cachedUsageUtilization`, never log, persist or retain any other value,
  store the email only in the private identity file, and skip the whole file
  when `primaryApiKey` is present; the Claude reporter and attribution check
  may also read whether `~/.claude/settings.json` sets `apiKeyHelper`. Account
  observations use native read-only Codex account RPCs and, for Claude, mod
  rate limits attributed to the account `~/.claude.json` names at report time,
  refused whenever the environment, configuration location or settings could
  select another credential or API endpoint, or Claude Code's usage cache when it names the
  same account and is fresh. No login/reset/redemption mutation." The
  sentences after it are unchanged.
- Native data contracts, allowances line. Old: "Allowances use explicit
  account mappings and source identity, independent of thread activity. Weekly
  windows are selected by duration, not field order." New: "Allowances use
  explicit account mappings and source identity, independent of thread
  activity, except Claude: its rows are local-only and need an active local
  reporting session or Claude Code's fresh account-matched cache. Weekly
  windows are selected by duration (Codex) or by window name (Claude
  `seven_day`), never field order."

README.md and the plugin README: the Claude allowance row, how to get the key
with `--claude-account-key` and add a `"provider":"claude"` mapping, that rows
need an active Claude Code session in a Herdr pane, the refusal conditions in
one list, rerunning `--install-claude-mod` after updating, and removing Claude
mappings before a runtime downgrade.

## Plan gate outcome

The user answered every plan-gate item on 2026-10-04. No gate item remains
open. G1 and G4 differ from the earlier defaults; the design, specs and tasks
are written for the answers below.

- **G1. Decision 2 (inferred attribution).** Accepted, and attribution is also
  refused when `ANTHROPIC_BASE_URL` is set, with any value (D4 step 1). The
  collector, identity refresh and `--claude-account-key` are unchanged.
- **G2. Decision 3 (argv) and in-flight evidence.** Accepted as the default:
  decision 3 as stated, and evidence skipped while a run is in flight is lost
  (D2). No pending flag or evidence-time argv value.
- **G3. Exemption.** Exactly `CLAUDE_CODE_MESSAGING_TOKEN` is exempt, on a
  closed list naming only it; any other matching name refuses (D4 step 1).
- **G4. Source-time rule.** A sample is also fresh, beyond the D2 rules, when
  `cost` is in `changed` and the session's cost total strictly increased over
  the mod's baseline (a real API response). Rewinds and compactions without
  cost growth stay not fresh. The whole-point and window-appears path is kept
  (D2 paths A and B).
- **G5. One stamp per sample.** Accepted: every window of a fresh sample gets
  the same stamp (D6).
- **G6. Account switch within a session.** Accepted: the session is refused for
  good (D6).
- **G7. Readers.** Accepted: `--refresh-identities`, `--claude-account-key` and
  `--claude-attribution-check` may read the same three allowlisted
  `~/.claude.json` fields as the reporter and collector (D5, D8).
- **G8.** Accepted as not built: no `$.session.authorize()` and no
  merged-settings `apiKeyHelper` check through `$.settings.read()`.

## Programme mapping

Change 4 of 4: `research-claude-code-parity`, `add-claude-thread-telemetry` and
`add-claude-status-reporter` (archived), then this change, which closes the
programme. It implements the D11 rows of the research metric mapping
(5-hour and weekly allowances, identity, pacing) for local Claude accounts.

## Migration Plan

1. Update the installed runtime and plugin as usual (private state, config,
   `.accounts.json`, `.peers.json` and the owner marker preserved), then run
   `--install-claude-mod` to refresh the mod. Running sessions pick it up after
   `/reload-plugins` or a new session.
2. Run `anton-runtime --claude-account-key`, add the mapping
   `{"id":…,"label":…,"category":…,"provider":"claude"}` under that key, and
   run `--refresh-identities` for the email.
3. **Rollback:** remove Claude mappings from the configuration first, then
   install the older runtime and rerun its `--install-claude-mod`. An older
   `uninstall.sh` does not know `claude-allowances.json`, so it leaves that file
   and the state directory; remove them by hand or uninstall with the newer
   script first.

## Risks / Trade-offs

- **[Inferred attribution]** Rate limits carry no account. A `/login` in
  another session rewrites `~/.claude.json` while an older session still uses
  the previous account; that older session's first report afterwards is
  attributed under the new account only if it has no session memory (first
  report, or memory lost after 24 hours or eviction). With memory, D6 refuses
  it. A `/login` inside the same session before its first attributed sample is
  not detected either: it relies on the next response carrying the new
  account's windows, which the profile then also names. Accepted by decision
  2; recorded.
- **[Base URL proxy]** `ANTHROPIC_BASE_URL` set in the session's environment
  refuses attribution (G1). A proxy or gateway configured without that
  variable, for example only through settings `env` that Claude Code does not
  export to children, is not detected (see [Credentials outside the
  checks]).
- **[API key appears later]** When `primaryApiKey` appears in `~/.claude.json`,
  nothing new is attributed, but windows stamped earlier stay visible for up to
  ten minutes until they turn stale. This fits decision 1, which forbids new
  attribution, and is recorded so the gate sees it.
- **[Lock hold]** The account step parses up to 4 MiB and writes under
  `hook.lock` after the window outcome is decided; sibling reporters wait at
  most 400 ms for the lock. Under the G4 cost path this happens about once per
  turn. The absolute ten-value hold time is measured against the 100 ms budget
  (D4).
- **[Gateway without `spend_limit`]** A gateway sign-in that omits
  `spend_limit` and sets no `ANTHROPIC_BASE_URL` is not detected without
  `$.session.authorize()`, which is not built (G8).
- **[Credentials outside the checks]** A token read from a well-known path
  without any variable, `apiKeyHelper` in project, local or managed settings,
  and a credential variable set only through settings `env` that Claude Code
  does not export to children are not detected. The first is D11's named
  residual risk with a fixture showing the reporter attributes in that case;
  the others are recorded.
- **[Exemption assumption]** `CLAUDE_CODE_MESSAGING_TOKEN` is assumed to serve
  Claude Code's in-session messaging, not API authentication. Unverified;
  task 5.3 records whether the name is present (names only, through
  `--claude-attribution-check`). The user accepted the exemption (G3).
- **[Staleness]** With the G4 cost path, rows stay fresh while a session
  completes a turn at least every ten minutes, and turn unavailable after ten
  minutes without a turn or a whole-point movement. Unknown is shown, never
  an old value as current.
- **[Cost without rate limits]** The cost path treats a strictly grown cost
  total as proof of a priced API response, and `rateLimits` holds what the
  last response reported. If a priced response carried no rate-limit reading,
  for example a side request the engine does not take rate limits from, the
  sample re-stamps the previous response's values at the new time, so a value
  can look up to one turn newer than its response. Accepted by the user
  (G4); the window values themselves are never invented.
- **[Evidence lost in flight]** See D2; accepted (G2).
- **[Different state or home directory]** A session with another
  `XDG_STATE_HOME` or `HOME` writes account state where the collector does not
  read it; the row stays unavailable.
- **[Cache scale]** The cache's utilisation scale is unconfirmed; values in
  (0, 1] stay unknown, and task 5.3 compares a non-zero cache value with the
  mod's for the same window (equality only).
- **[Provider state growth or mode]** A `~/.claude.json` over 4 MiB, or one
  whose mode gains group or other bits, disables Claude attribution and
  identity (fail closed). The parse cost is measured.
- **[Settings file shape]** If `~/.claude/settings.json` is a symlink (for
  example from a dotfile manager), the no-follow read refuses and the feature
  stays off. Task 5.3 records whether it is a regular file.
- **[Rate limits in the process table]** Used percentages and reset times are
  argv and visible in `/proc/<pid>/cmdline` for under 2 s. They are not
  account identity. Accepted as for change 3's session id.
- **[Mods API changes]** A changed `SessionRateLimit` or `changed` shape fails
  the tail checks, so no rate limits are sent; the window report continues. A
  changed `SessionCost` shape makes the cost total unknown, so only the window
  path can stamp.
- **[Rollback]** A configuration with a `provider` field is rejected by older
  runtimes (Migration plan).

## Alternatives

- **statusLine payload (research surface A).** Not used; change 3 shipped the
  mod.
- **Stamping only on window movement.** Rejected at the plan gate (G4): rows
  would stay fresh only during heavy use.
- **Pending evidence with an evidence-time argv value.** Not chosen (G2).
- **Per-window value-change stamps (D11).** Not chosen (G5).
- **Rate limits in pane metadata.** No free keys, and rate limits belong to an
  account, not a pane.
- **A separate lock for the account state file.** Unneeded: `hook.lock`
  already serialises every writer.
- **Following `CLAUDE_CONFIG_DIR` in the collector.** Rejected: the collector
  runs with the shell's environment and cannot know a session's directory, so
  it refuses when the variable is set in its own environment (D7) rather than
  following it.

## Open questions

These do not change the specs or tasks; task 5.3 records answers where it
can. The former plan-gate questions are answered (Plan gate outcome).

1. **Cache utilisation scale.** Settled by comparing a non-zero cache value with
   the mod's `percentUsed` for the same window at the live check.
2. **`CLAUDE_CODE_MESSAGING_TOKEN` without agent teams.** Whether the name
   appears when agent teams are off. The exemption works either way.
3. **Window leaving at reset.** Whether Claude Code drops or keeps a window at
   its reset with no API response. Neither is fresh under D2, so only
   staleness timing differs.
4. **Unknown reset count on Claude cards.** A later change could add a provider-neutral
   row field for sources without reset passes.

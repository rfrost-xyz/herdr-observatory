# Evidence

Synthetic fixtures only. No private transcripts, session ids, account ids,
emails, paths, provider-state values, settings values or snapshots are recorded
here. Host facts are recorded as key names, types, counts, kinds, booleans and
equality results only.

## Baseline

### Task 1.1: measurement section

Commit `90647cd` (`test(bench): measure claude allowance reporter and
collector costs`) adds the `claude_allowance` section to
`tests/measure_anton_popover.mjs`, with the `--no-claude-allowance-probe`
flag, and changes nothing else:

- `git diff -U0` of the commit has 181 added lines and no removed line; the
  only edits outside the new block are one help line, one
  `report.claude_allowance = null;` line, one call line and appended print rows.
- `--skip-runtime --json` before and after the edit: `provider_coupling` is
  identical (all counts 0, no files), the only new top-level key is
  `claude_allowance` and no other non-timing key differs.
- Fixture: the owner marker, `--install-hooks` then `--install-claude-mod` in
  a temporary home (cleared environment with `HOME`, an empty `PATH` directory
  and `MISE_SYSTEM_CONFIG_DIR`, as the Rust installer fixtures), a fake Herdr
  socket answering `pane.get` with a bound `herdr:claude` session, applying
  `pane.report_metadata` and serving the collector's snapshot, and a
  synthetic `.claude.json` (the D11 fixture uuid and email, a stale cache)
  of exactly 165,000 and 4,194,304 bytes, mode 0600. Reporter runs get only
  an absolute `HOME` and the fixture bin directory as `PATH`, so no inherited
  `ANTHROPIC_*`, `CLAUDE_CODE_*` or `CLAUDE_CONFIG_DIR` name reaches them.
- Per `.claude.json` size, 15 sequential four-value runs and 15 ten-value runs
  (labelled `ten_value_per_turn_path`, the G4 cost path). Each reports wall
  and CPU (bash `time`) and the `hook.lock` hold as an upper bound: from the
  reporter's first connection to the fake socket (`pane.get`, made right
  after it takes the lock) to its exit, so process teardown is included. A
  run that exits non-zero, or a ten-value run that leaves
  `claude-allowances.json` unchanged, nulls the variant with a reason;
  `within_budget` is the maximum ten-value hold against the D4 budget of
  100 ms. The collector then runs for 10 s with the same `--state`, two Codex
  mappings (one served by a fake `codex`) and one Claude mapping, reporting
  rows, Claude rows, available counts, allowances and snapshot bytes, time to
  an available Claude row and CPU per snapshot.
- Check of the collector fixture on the old binary with the `provider` key
  removed from the Claude mapping (a scratch copy of the script, not
  committed): exit 0, 5 snapshots, 3 rows, 1 Codex row available, 0 Claude
  rows. So the null collector result below comes from the Claude mapping
  alone.
- Not yet exercised: on `7a9fefb` the ten-value metrics, the state-file-changed
  check, `within_budget` and the collector's available Claude row detection
  cannot run; they first run after tasks 2.3 and 2.4 and are judged at task
  5.1. Each ten-value run follows four-value runs that already wrote the same
  window, so it measures the no-change return plus the account step (the
  usual per-turn shape), never a metadata write and an account write in one
  run.
- No test is added by tasks 1.1 or 1.2, so no fail-on-old proof applies. The
  old-binary nulls below are the expected absence.

### Task 1.2: `7a9fefb` baseline (2026-10-04)

`git diff --stat 7a9fefb HEAD` before task 1.1 touched only
`openspec/changes/add-claude-allowances-identity/`. Built from
`git archive 7a9fefb` extracted in a private `mktemp -d /tmp/c4x-XXXX`
directory (deleted afterwards), with `CARGO_TARGET_DIR` inside it:
`cargo build --release --locked --offline`, rustc 1.96.0, cargo 1.96.0.

- Release binary sha256:
  `0762dcce1554969cf826d1beb7a627748629dade3de95da60d3a1d4ebe376473`.
- `node tests/measure_anton_popover.mjs --source-root <archive> --binary
  <archive build> --repeat 3` (script at `90647cd`, `TMPDIR` private), 30 s
  windows, 2 hosts, 32 agents per host. Medians: runtime CPU 0.075 s
  (windows 0.074, 0.075, 0.079), peak RSS 4,348 KiB (4,212, 4,604, 4,348),
  mean snapshot 27,677.8 bytes (27,678.9, 27,145.9, 27,677.8), 10 snapshots
  (10, 9, 10). Allowance wire: 2 rows, 337 bytes per row, 11 keys. Claude
  probes: standard 0.117 s CPU, large 0.72 s, window 0.125 s, native agents
  4/4 of 4/4 in each, window variant 2/0 agents with window and
  context_percent.
- `claude_allowance` section: install exit 0 with the receipt written;
  `.claude.json` 165,000 and 4,194,304 bytes.
  - Four-value runs, 165 KB: 15 of 15 exit 0 (first run `pane.get` and
    `pane.report_metadata`, later runs `pane.get` only); wall median 3 ms,
    max 4; CPU median 2 ms, max 3; lock hold first 2.1 ms, median 1.4, max
    2.1.
  - Four-value runs, 4 MiB: 15 of 15 exit 0 (`pane.get` only); wall median 3
    ms, max 3; CPU median 2 ms, max 3; lock hold first 1.5 ms, median 1.4,
    max 1.8.
  - Ten-value runs (per-turn path), both sizes: the first run exits 2, so the
    metrics are null ("run 1 exited 2"), as expected.
  - Collector: exit 1, "Invalid allowance account mapping", all metrics
    null, as expected.
- Suites on the archive (private `TMPDIR`): `cargo test --locked --offline`
  347 passed (271 lib, 22 main, 6 navigation, 48 process), 0 failed;
  `cargo fmt --check` clean; `cargo clippy --locked --offline --all-targets
  -- -D warnings` clean (local 1.96); `node --test` of the Pi hooks, Claude
  mod, State and distribution suites 115 passed, 0 failed;
  `tests/run-qml.sh` 95 passed, 0 failed; `tests/run-qmllint.sh` exit 0
  ("no warnings outside Panel.qml"); `tests/run-shell-harness.sh` exit 0
  ("failures 0", 6 runtime invocations, 0 `--open-thread`).

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
- `SessionCost` (10260-10268): `{usd: number}`, "US dollars, summed over every
  priced API response this session". `SessionMeasureInput.cost?`
  (10391-10394): "absent where the host keeps no ledger".
  `SessionUsage.cost?` (11005-11009): "absent only where the host keeps no
  cost ledger (the CLI always has one)". The `changed` doc in
  `SessionMeasureInput`: "`cost`: the total grew". Re-read on 2026-10-04 for
  plan-gate answer G4 (design D2 path B).
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
  is `CLAUDE_CODE_MESSAGING_TOKEN` (design D4, G3). No `ANTHROPIC_*` name,
  so no `ANTHROPIC_BASE_URL`, was present in that environment (G1). The
  settings `env` names were not checked against the G1 rule and settings were
  not read again for this.
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
  `claude_report_rejects_invalid_arguments_without_socket_access` expects exit
  2 for five values, and
  `claude_report_writes_the_bound_window_once_and_repeats_read_only` asserts
  no "account" or "rate" in the wire.
- `uninstall.sh` removes only the listed state file names.

## Plan gate outcome (2026-10-04)

The user answered every plan-gate item on 2026-10-04. No item remains open.
Planning artefacts updated in the commit `docs(openspec): record the claude
allowances plan gate`.

- G1: accepted, and attribution is also refused when `ANTHROPIC_BASE_URL` is
  set (any value, name only; design D4 step 1). Differs from the earlier
  default.
- G2: accepted default; evidence skipped while a run is in flight is lost.
- G3: exactly `CLAUDE_CODE_MESSAGING_TOKEN` is exempt, on a closed list
  naming only it.
- G4: a sample is also fresh when `cost` is in `changed` and the session cost
  total (`SessionCost.usd`) strictly increased over the mod's baseline; rewinds
  and compactions without cost growth stay not fresh; the whole-point and
  window-appears path is kept (design D2 paths A and B). Differs from the
  earlier default.
- G5: accepted default; one stamp per sample.
- G6: accepted default; a session is refused for good after an account
  switch.
- G7: accepted default; `--refresh-identities`, `--claude-account-key` and
  `--claude-attribution-check` may read the same three allowlisted
  `~/.claude.json` fields.
- G8: accepted default; no `$.session.authorize()` and no merged-settings
  `apiKeyHelper` check.

Validation of the updated artefacts: `OPENSPEC_TELEMETRY=0 openspec validate
add-claude-allowances-identity --strict` reported the change valid, and a
search of the change directory found no em or en dash. No code or test was
changed, so no new test exists yet to show failing on `7a9fefb`.

## Plan review (2026-10-04)

An independent review of the planning artefacts after the plan gate raised one
blocking, six non-blocking and five nit findings. All were applied; none was
declined. No code or test exists yet, so no fail-on-old proof is claimed here;
the tasks now require it for each new test.

1. Peer-side Claude prohibition and "Codex cache untouched" untested: applied.
   D11 and tasks 2.4 and 2.5 add `--allowances-probe`, `--identity-probe` and
   `allowances.json` process fixtures with a Claude mapping, state file and
   synthetic `.claude.json`, each asserting exit 0 and a Codex row or identity
   so they fail on `7a9fefb`.
2. Cost growth during a rewind: applied. D2 now says any measurement with
   strict cost growth and `cost` in `changed` is fresh, whatever triggered it,
   citing Risks [Cost without rate limits]; a mod test (D11, task 3.2) and a
   harness-telemetry scenario pin it; tasks 5.2 and 5.3 adjusted.
3. Regression guards: applied. D11 and the tasks limit them to existing tests;
   new no-change assertions sit in tests that also assert new behaviour.
   Fixtures with a Claude mapping that assert an absence or rejection also
   assert an accepted outcome, and each rejection is paired with an accepted
   neighbour. The State.js and QML cases (task 4.2) cannot fail on `7a9fefb`
   because D9 makes no presentation change, so they are added as assertions
   inside the existing provider-neutral tests and counted as regression
   guards, with no new test function.
4. Lane ordering: applied. Tasks preamble, 3.1, 3.3 and D10 state 3.1 after
   2.1 and 3.3 after 3.1.
5. AGENTS.md timing: applied. New task 1.3 applies the D12 amendments,
   wording unchanged, before 2.1; 4.1 keeps the README work. AGENTS.md itself
   is not edited in this planning commit.
6. Collector tests: applied. Shared four-account cap, ids unique across
   providers, Claude `window_seconds` and an unmapped key in
   `claude-allowances.json`, each with an accepted neighbour (D11, task 2.4,
   account-allowances scenario).
7. Attribution after a failed window report: applied. The harness-telemetry
   requirement says "only when the window report did not fail", with a new
   scenario; the proposal matches.
8. Lock budget: applied as the absolute ten-value `hook.lock` hold of at most
   100 ms in D4, D5, D11, Risks and tasks 1.1 and 5.1.
9. Line cites: applied. Test names replace `:2344` and `:2267` in design D3,
   D10, tasks 2.1 and 2.3 and the [repo] facts above.
10. Proposal first-measurement rule: applied ("nor for another session id").
11. Decision 3 against D2: applied ("strengthened in D2").
12. Measure builder: applied. The builder fix moves to task 3.1, with defaults
    that keep existing argv at four values, so 3.1's `node --test` exercises
    the new path.

## Implementation

_Pending: one section per task group, with commits, test counts and the
new-behaviour tests shown failing on `7a9fefb`._

## After

_Pending (task 5.1)._

## Review rounds

_Pending (task 5.2)._

## Live installed check

_Pending (task 5.3)._

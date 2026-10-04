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

### Task 1.3: AGENTS.md amendments

The three D12 amendments are applied to AGENTS.md with the wording quoted in
design.md and the lines rewrapped to 80 columns. A script collapsing all
whitespace runs to one space confirmed, for each of the three, that the old
sentence no longer occurs and the new sentence occurs verbatim. The sentences
after each amendment are unchanged. AGENTS.md contains no em dash (U+2014).
No test is added, so no fail-on-old proof applies.

### Task 2.1: reporter argv grammar

`ClaudeReport::parse` takes 4, 7 or 10 values; the tail is parsed into
`RateWindow {kind, tenths, resets}` with the D3 grammar and the exact
`S < r <= S + D + 3600` bound from `seq`, and any failure exits 2 before any
file or socket access. `main.rs` needed no change: `--report claude` already
passes every later value through verbatim. The working tree first held two
interleaved partial copies (the parser used `windows` and `RateKind`, the unit
test `tail` and string kinds); the parser's copy was kept and the unit test
aligned to it.

- Tests: `reporter::tests::claude_arguments_are_bounded_digits_and_safe_ids`
  (four values asserted unchanged beside the seven- and ten-value results;
  counts 5, 6, 8, 9 and 11, unknown and repeated kinds, used values `-1`,
  `100.1`, `1.25`, `01`, `5.0`, `1e1`, `+5` and others, resets in the past, at
  `S`, past the bound and with 12 digits, each beside an accepted neighbour)
  and `claude_report_rejects_invalid_arguments_without_socket_access` in
  `native_process.rs` (the same rejections through the CLI with no socket or
  state access, then seven- and ten-value runs exiting 0 with the change 3
  wire). Both were changed intentionally as D3 says: five and six values still
  exit 2.
- On `7a9fefb` (tree extracted with `git archive` into a private temporary
  directory, new test files copied in): the process test fails at
  `tests/native_process.rs:2332`, the seven-value neighbour, with `left:
  Some(2)`, `right: Some(0)`; the unit test does not compile (E0422 and E0433
  for `RateWindow` and `RateKind`, E0560 and E0609 for `windows`).
- On the change: `cargo test --locked --offline` 271 + 22 + 6 + 48 passed,
  0 failed; `cargo fmt --check` and `cargo clippy --locked --offline
  --all-targets -- -D warnings` clean. Tests ran with a private 0700 `TMPDIR`
  short enough for Unix socket paths.

### Task 2.2: attribution rules and provider-state reads

New `src/claude_account.rs` (public, so later tasks can call it). Every
reader takes the home directory as a parameter; the environment rule takes
the name list as a parameter, and `environment_names()` reads names from
`environ` without copying any value. Globs are matched byte by byte, so
`CLAUDE_CODE_FILE_DESCRIPTOR` does not match `CLAUDE_CODE_*_FILE_DESCRIPTOR`.
The legacy file and settings checks use `symlink_metadata` first, so a
dangling link refuses and only a missing file passes. Top-level files are
parsed through an object-only wrapper, because a derived serde struct also
accepts a JSON array (found by the `[]` settings case). Errors are four fixed
strings.

- Unit tests (`claude_account::tests::`):
  `hashes_use_their_own_prefixes`,
  `environment_names_refuse_by_pattern_with_one_exemption`,
  `environment_names_are_the_process_names`,
  `configuration_location_refuses_config_dir_and_legacy_file`,
  `api_key_helper_presence_refuses`,
  `provider_state_gives_only_the_key_or_a_fixed_error`,
  `identity_extraction_adds_only_the_email`,
  `collector_extraction_keeps_only_the_matched_cache_windows`,
  `attribution_names_the_first_refusing_step`,
  `iso_times_follow_the_grammar`. They cover each D4 pattern, the exemption
  alone and beside `CLAUDE_CODE_SESSION_TOKEN`, a non-UTF-8 name,
  `ANTHROPIC_BASE_URL` refusing and `ANTHROPIC_BASE_URL_X` not matching,
  `CLAUDE_CONFIG_DIR`, the legacy file as a file, a dangling link and a
  directory, `apiKeyHelper` as a string, `null`, an object and a number,
  settings absent, linked, over 1 MiB and malformed, `primaryApiKey` as a
  string, `null` and an object, missing and invalid ids, a link, mode 0644,
  over 4 MiB and just under it, mistyped allowlisted fields, the `PRIVATE`
  marker and the fixture uuid and email absent from every error and from the
  cache reading's debug text, and the ISO grammar's accepted and rejected
  shapes.
- On `7a9fefb`: `src/claude_account.rs` does not exist, so all ten unit tests
  fail there (nothing to compile against), recorded by name above.
- Planning fix: no command reaches this module until the reporter calls it in
  task 2.3, so the process fixtures task 2.2 asks for land with task 2.3, and
  2.2 is ticked in that commit. The reporter reaches only D4 steps 1 to 5 and
  the reporter extraction, so those cases get process fixtures there. The
  collector extraction (cache matching, another account's cache, mistyped
  `fetchedAtMs` in the collector), the identity extraction (email) and the
  ISO parser have no command until tasks 2.4 and 2.5, which carry their
  process fixtures; until then they are covered by the unit tests above.
- On the change: `cargo test --locked --offline` 281 + 22 + 6 + 48 passed,
  0 failed; fmt and clippy (`-D warnings`) clean.

### Task 2.3: account state file and the reporter's account step

`claude_account::record` implements D6 (version 1, at most 4 accounts, 32
sessions and 16,384 bytes, newest stamp wins per window, hashed session
memory with permanent refusal after a switch, a missing, malformed,
oversized, other-version or unknown-key owned file replaced, a link, another
type or owner, or a loose mode refusing, `atomic_owned_write`). `used_percent`
is written as an integer when whole (`40`, as in the D6 example).
`report_claude` calls the account step on both success paths (after the
metadata write and on the no-change return), still under `hook.lock`, only
for a tail, never after a failed write; every outcome is silent and the exit
status is unchanged. Two readings of the plan, recorded here: steps 1 to 5
refusing write nothing (no `at` renewal), while a report reaching session
memory always renews `at`, including for a refused session, so a refused
session that keeps reporting does not expire and attribute again; `at` keeps
the later of the stored and reported times, so a late older report does not
lower it. The fake Herdr socket in `native_process.rs` gained a
`fixture_reject` pane flag that fails the metadata write.

- Process fixtures (`native_process.rs`):
  `claude_report_records_fresh_windows_for_the_profile_account` (success, the
  change 3 wire byte-identical with a tail and no "account", "rate" or window
  text in it, the exact state file at mode 0600, an unchanged window still
  recording, a window absent from the tail kept, a four-value run leaving the
  state bytes unchanged);
  `claude_report_refusals_keep_the_window_report_and_write_no_account_state`
  (every D4 refusal through the CLI, exit 0 with the window bound and no state
  file: each environment pattern, `ANTHROPIC_BASE_URL` set to a URL carrying
  `PRIVATE` and set empty, the exempt name beside `CLAUDE_CODE_SESSION_TOKEN`,
  a non-UTF-8 name, `CLAUDE_CONFIG_DIR` empty, at `~/.claude` and elsewhere,
  the legacy file and a dangling link, `apiKeyHelper` as a string, `null` and
  a mistyped object, settings malformed, an array, over 1 MiB and linked,
  `primaryApiKey` as a string and `null`, no account, an empty id, a mistyped
  id, a non-object file, over 4 MiB, mode 0644, a link and missing; accepted
  neighbours writing state: no variable, the exempt name alone,
  `ANTHROPIC_BASE_URL_X`, `ANTHROPIC_MODEL`, a non-UTF-8 value, settings
  without `apiKeyHelper`, a mistyped field outside the extraction; the marker,
  uuid and email absent from stdout, stderr and the state directory after
  every run);
  `claude_report_account_state_keeps_newest_stamps_and_refuses_switched_sessions`
  (two panes on two sessions of one account with the older `seq` second, the
  account switch refusing the session for good, other sessions attributing);
  `claude_report_account_state_bounds_and_replacement` (account and session
  eviction with refused entries last, 24-hour expiry, replacement and
  refusal of unsafe files);
  `claude_report_failed_metadata_write_writes_no_account_state` (exit 1, no
  state; the same report then exit 0 with state);
  `claude_report_with_large_provider_state_lets_a_pi_report_through` (three
  rounds of a ten-value run on a 4 MiB `.claude.json` beside a Pi report,
  both succeeding on one state directory; the fake socket delays the Claude
  metadata reply by 100 ms, the Pi run starts only once that write has
  arrived and the Claude process is still running, and the call order shows
  Pi's `pane.get` after the Claude write). The refusal fixture is the process
  coverage for task 2.2's D4 steps 1 to 5 and reporter extraction cases; the
  collector, identity and ISO cases pass to tasks 2.4 and 2.5 (see task 2.2).
  A debug build takes about 48 ms for the attribution steps on the 4 MiB
  file and a release build 3 to 7 ms (three runs each, a throwaway example
  binary, not committed), well inside the 400 ms lock wait.
- Unit tests (`claude_account::tests::`):
  `state_percentages_round_trip_in_tenths` and
  `state_write_over_the_byte_bound_is_refused` (the byte bound cannot be
  reached through valid reports, so it is tested on the encoder).
- On `7a9fefb` (fresh `git archive` extract, new `native_process.rs` copied
  in): all six new process fixtures fail, each at its first tail run with
  `left: Some(2)`, `right: Some(0)` (`Some(1)` for the failed-write case),
  beside the 2.1 fixture failing as recorded above; the seven existing
  `claude_report_*` fixtures, including the no "account"/"rate" guard in
  `claude_report_writes_the_bound_window_once_and_repeats_read_only`, pass
  there and on the change. The two unit tests have no module to compile
  against there.
- Mutation checks on the change: renaming the `ANTHROPIC_BASE_URL` rule fails
  the refusal fixture at its first case, and running the account step after a
  failed write fails the failed-write fixture; both reverted.
- On the change: `cargo test --locked --offline` 283 + 22 + 6 + 54 passed,
  0 failed; fmt and clippy (`-D warnings`) clean.
- Not arranged: a `.claude.json` or settings file owned by another user (the
  tests run unprivileged); `read_owned` checks the owner.

### Tasks 2.2 and 2.3 fix: objects only, at every depth

A derived serde struct accepts a JSON array as well as an object, at any
depth, so `{"oauthAccount":["<uuid>"]}` attributed, an array-shaped cache or
cache window was read, and an array-shaped account, window or session entry
in `claude-allowances.json` survived instead of being replaced (D6). Every
nested allowlisted struct in the three extractions and in the state file now
goes through the same object-only adaptor as the top level.

- Tests: `claude_account::tests::provider_state_gives_only_the_key_or_a_fixed_error`
  (an array `oauthAccount` refusing beside its object neighbour),
  `claude_account::tests::collector_extraction_keeps_only_the_matched_cache_windows`
  (an array cache, utilisation map and window, each malformed) and
  `claude_report_account_state_bounds_and_replacement` (an array account,
  windows map, window and session entry each replaced).
- On the previous commit's source with these tests: the two unit tests fail
  (`left: Ok(<key>)` and `left: Ok((<key>, Some(Cache {..})))` against
  `right: Err("Provider state malformed")`) and the process fixture fails at
  the array account case, the seeded account kept. On `7a9fefb` the process
  fixture fails at its first tail run, as before; the seven existing
  `claude_report_*` fixtures still pass there.
- On the change: `cargo test --locked --offline` 283 + 22 + 6 + 54 passed,
  0 failed; fmt and clippy (`-D warnings`) clean.

### Task 2.4: Claude allowance rows in the collector

`allowances::mapping` accepts an optional `provider` (`codex` or `claude`);
a Codex mapping's output is unchanged with or without it, and a Claude
mapping's output carries `"provider":"claude"` and no `window_seconds`.
`allowances::mapped` matches a key to a mapping of one provider, and every
Codex match uses it: Codex cache and peer rows in the snapshot, `receive`
(so `allowances.json` never stores a row for a Claude mapping) and the
`--allowances-probe` filter. `snapshot_with` routes Claude mappings to
`claude_account::row`, which reads `claude-allowances.json` from the
collector's own state directory on each snapshot (`read_state`, with its
own object-only structs, separate from the reporter's) and the in-memory
provider-state reading. The local allowance worker replaces that reading
with `claude_account::collector_reading()` at start and every 60 s, before
the Codex refresh, only when a Claude mapping exists; a refusal or failure
stores none, never an older reading. `model.rs` now says at most one window
per row is pacing.

Readings of the plan, recorded here:

- A Claude row is available only when every window stamp lies within
  `[-1, 600]` seconds of now. D7 bounds the oldest stamp; a window stamped
  more than 1 s ahead now also makes the row unavailable, matching the
  future bound on every other source.
- A cache utilisation that is absent, not finite, negative, in (0, 1] or
  above 100 leaves that window's used value unknown and keeps its reset; a
  reset that does not parse, or parses before 1970, drops the window.
- The collector, identity refresh and the key command read
  `$HOME/.claude.json` only for an absolute `HOME`; otherwise nothing.
- In the state file, an account key that is not 64 hex characters or
  appears twice rejects the whole file; a window value of the wrong type
  drops that window; session entries are checked only for their three keys
  and count, their values are skipped.

- Unit tests (`allowances::tests::`):
  `provider_mappings_keep_codex_output_and_validate_claude`,
  `account_cap_and_ids_are_shared_across_providers`,
  `claude_rows_come_from_the_account_state_file` (both windows, an
  unmapped key giving no row, one stale window, a window over 1 s ahead,
  a past reset, `five_hour` only, no state),
  `claude_state_file_is_rejected_whole_or_loses_bad_windows` (other
  version, unknown keys at three depths, an unknown window kind, an array
  where an object belongs, 33 sessions, 5 accounts, a bad or repeated key,
  oversized, 0644, a link, each beside the valid file; ten bad window
  values, `12.25` among them, each dropping only that window),
  `claude_cache_fallback_matches_account_freshness_and_scale` (0 and 1.5
  read, 0.5 and 1 unknown, half-up rounding, another account, stale and
  future fetches, no cache, newer stamp per window both ways) and
  `codex_sources_never_fill_claude_mappings` (a peer row carrying the
  Claude key ignored beside the Claude row and the Codex row; `receive`
  refusing a Claude-mapped row and keeping the Codex one).
- Process fixtures (`native_process.rs`, cleared environment with no
  `CLAUDE_CONFIG_DIR`, stderr captured):
  `claude_collector_rows_follow_the_account_state_file` (state in another
  directory leaves the row unavailable while both Codex rows are
  available; then both windows exactly, no row or value for an unmapped
  key, a peer row carrying the Claude key ignored while the peer's Codex
  row is present; malformed, oversized, other-version, unknown-key, 0644
  and symlinked files each unavailable and then available again; `12.25`
  dropping one window; one stale window; a past reset; with
  `ANTHROPIC_BASE_URL` set throughout; then `allowances.json` and
  `--allowances-probe` holding one Codex row in the legacy key shape;
  no `PRIVATE` marker, uuid, email or Claude key in any snapshot, stderr,
  probe output or cache),
  `claude_collector_cache_fallback_and_refusals` (eleven collectors: a
  matched fresh cache whose resets carry `+01:00` with `.999` and `-02:30`
  with `.5`, the ambiguous scale, the newer stamp per window,
  `ANTHROPIC_BASE_URL` set, each available; another account, stale,
  future, `primaryApiKey`, a mistyped `fetchedAtMs`, `CLAUDE_CONFIG_DIR`
  and a legacy file, each unavailable with the Codex row available) and
  `claude_mappings_share_the_account_cap_and_ids_through_the_cli` (two
  Codex and two Claude mappings accepted with the probe output equal to the
  Codex-only configuration's, a fifth of either provider rejected, a
  shared id rejected beside a distinct one, `window_seconds` 604800
  accepted and 18000 rejected, an unknown provider rejected, and the Codex
  account mapped as Claude giving an empty probe).
- Task 2.2's deferred process coverage of the collector extraction is
  closed here: the matched cache, another account's cache, a mistyped
  `fetchedAtMs` and the ISO parser (offsets and fractions) run through the
  collector in `claude_collector_cache_fallback_and_refusals`. The email
  extraction is covered in task 2.5.
- On `7a9fefb` (full `git archive` extract, new `native_process.rs`
  copied in): all three process fixtures fail, the two collector fixtures
  at their first snapshot wait (`snapshot deadline`: the collector exits on
  the `provider` key) and the CLI fixture at the four-mapping probe (exit 1).
  The six unit tests cannot compile there (`claude_account`,
  `snapshot_full`, `provider`, `has_claude` do not exist).
- Mutation check on the change: dropping the D4 steps 2 and 3 check from
  `collector_reading` fails the cache fixture at its `CLAUDE_CONFIG_DIR`
  and legacy-file cases (`left: "available"`); reverted.
- On the change: `cargo test --locked --offline` 289 + 22 + 6 + 57 passed,
  0 failed; fmt and clippy (`-D warnings`) clean.

### Task 2.5: identity refresh and the Claude commands

`identity::mapped` matches the Codex RPC row and peer `--identity-probe`
rows to Codex mappings only (`allowances::mapped`), and `--identity-probe`
filters the same way. `identity::refresh` then adds the local Claude email
through `claude_email`, which calls the provider-state reader only when a
Claude mapping exists, `HOME` is absolute and D4 steps 2 and 3 pass, and
keeps the email only for a key a Claude mapping names and only when it
passes the existing email check. No peer is asked for Claude identity.
`--claude-account-key` prints only the key and a newline (exit 0) or
nothing (exit 3) after D4 steps 2 and 3, with no owner guard or
configuration. `--claude-attribution-check` runs D4 steps 1 to 5 against
its own environment and the reporter's home resolution (new
`reporter::claude_home`) and prints `claude_account::attribution_check`'s
text, exit 0 for `ok` and 3 otherwise.

- Planning fix (design D8, validated with `openspec validate --strict`):
  D8 left the check's output format open. It now states that the matching
  names follow the first line whatever the outcome (so an `ok` run shows an
  exempt name, which task 5.3 needs), sorted, an exempt one as
  `<name> exempt`; a non-UTF-8 name adds one `non-utf8-name` line; and
  without a home the step after steps 1 and 2 is `provider-state`.
- Unit tests: `identity::tests::claude_email_reads_only_behind_its_gate`
  (the reader is never called without a Claude mapping, without a home,
  with `CLAUDE_CONFIG_DIR` or with a legacy file; `ANTHROPIC_BASE_URL` does
  not stop it; an unmapped or Codex-mapped key, an invalid or absent email
  or a read error store nothing, beside the mapped case),
  `identity::tests::codex_identity_rows_match_codex_mappings_only` and
  `claude_account::tests::attribution_check_prints_only_step_and_names`.
- Process fixtures (`native_process.rs`, cleared environment):
  `claude_account_key_prints_only_the_key` (65 bytes and exit 0, also with
  `ANTHROPIC_BASE_URL` set; exit 3 with empty stdout and stderr for
  `primaryApiKey` as a string and `null`, an invalid id, a missing file,
  `CLAUDE_CONFIG_DIR`, a relative `HOME` and a legacy file, each beside an
  accepted run),
  `claude_attribution_check_names_only_the_first_refusing_step` (`ok`,
  `ok` with the exempt name, `ANTHROPIC_BASE_URL` set to a URL and set
  empty printed by name, `ANTHROPIC_BASE_URL_X` not matching, several
  names sorted with the exempt one marked, a non-UTF-8 name, `config-dir`,
  `api-key-helper` beside settings without it, `provider-state` for
  `primaryApiKey` and a missing file, `legacy-config`; no `PRIVATE`
  marker, uuid, email or key in any output),
  `claude_identity_refresh_stores_the_local_email_for_a_mapped_key` (the
  Claude email beside the Codex email for a mapped key, also with
  `ANTHROPIC_BASE_URL` set; only the Codex email for an unmapped profile,
  `primaryApiKey`, an invalid email, a missing file, `CLAUDE_CONFIG_DIR`,
  a legacy file and a configuration without a Claude mapping, each beside
  the mapped run) and
  `claude_identity_rows_from_codex_sources_never_fill_claude_mappings`
  (a peer row carrying the Claude key and the Codex RPC row whose key a
  Claude mapping names store nothing, while the peer's Codex email and the
  local Claude email are stored; without the local profile the peer row
  still stores nothing; `--identity-probe` with a Claude mapping beside the
  Codex one answers with the Codex identity only and no Claude key, uuid
  or email, and fails when the Codex key is mapped as Claude).
- Task 2.2's deferred identity (email) extraction coverage is closed by the
  identity refresh fixtures above.
- On `7a9fefb` (the same extract, new `native_process.rs` copied in): all
  four fixtures fail: the key command and the check exit 1 as unknown
  commands (`left: Some(1)`, `right: Some(0)`; `left: ""`,
  `right: "ok\n"`), and both identity fixtures fail at their first refresh,
  where the Claude mapping makes the configuration invalid. The three unit
  tests cannot compile there.
- Mutation checks on the change: matching identity rows to any provider's
  mapping fails `claude_identity_rows_from_codex_sources_never_fill_claude_mappings`
  (the Codex RPC email stored under the Claude mapping, and, with that case
  masked, the peer's email stored under it once the local profile is
  removed); reverted.
- On the change: `cargo test --locked --offline` 292 + 22 + 6 + 61 passed,
  0 failed; fmt and clippy (`-D warnings`) clean.

### Task 2.6: lane A gates

At `7a32739`, `cargo fmt --check` and `cargo clippy --locked --offline
--all-targets -- -D warnings` (local Rust 1.96.0) are clean, and
`cargo test --locked --offline` passes 292 library, 22 binary, 6
navigation and 61 process tests, 0 failed (a private 0700 `TMPDIR`). The
lane A source adds no `Some(x).filter(|_| ..)` and no argument-free
`format!` (a scan of the diff from `7a9fefb`), the two patterns the CI
clippy (1.98) rejects, and no standard API newer than the crate's
`rust-version` 1.85.

Lane A commits:

- `33fdc66` feat(reporter): accept claude rate-limit tails of seven or ten values (2.1)
- `39e2aa3` feat(runtime): add claude attribution rules and provider-state reads (2.2)
- `4e2d579` feat(reporter): record attributed claude rate limits in account state (2.3)
- `eb9efa9` fix(runtime): accept only objects in nested claude provider and account state (2.2, 2.3)
- `1322193` feat(allowances): build local claude rows from account state and cache (2.4)
- `4edc041` docs(openspec): define the claude attribution check output format (2.5 planning fix)
- `7a32739` feat(identity): add the local claude email and claude account commands (2.5)

New-behaviour tests shown failing on `7a9fefb` (details under each task):

- Process fixtures that run and fail there: 2.1
  `claude_report_rejects_invalid_arguments_without_socket_access`; 2.3 the
  six `claude_report_*` account-state fixtures; 2.4
  `claude_collector_rows_follow_the_account_state_file`,
  `claude_collector_cache_fallback_and_refusals`,
  `claude_mappings_share_the_account_cap_and_ids_through_the_cli`; 2.5
  `claude_account_key_prints_only_the_key`,
  `claude_attribution_check_names_only_the_first_refusing_step`,
  `claude_identity_refresh_stores_the_local_email_for_a_mapped_key`,
  `claude_identity_rows_from_codex_sources_never_fill_claude_mappings`.
- Unit tests that cannot compile there: 2.1
  `reporter::tests::claude_arguments_are_bounded_digits_and_safe_ids`;
  the thirteen `claude_account::tests::` tests of 2.2, 2.3 and 2.5; the six
  new `allowances::tests::` tests of 2.4; the two new `identity::tests::`
  tests of 2.5.
- The whole process binary at `c1cca5d` (its `native_process.rs` and
  `tests/support/` copied into a fresh `7a9fefb` extract, which includes the
  `Stream::spawn` refactor of the existing `Stream::new`): 47 passed and 14
  failed, the 14 being exactly the fixtures named above.
- Regression guards (existing tests, unchanged by lane A except the two
  intentional D3 contract changes in 2.1) pass on both: the seven change 3
  `claude_report_*` fixtures, the Codex allowance contract, legacy cache
  and peer fixtures, and the identity and snapshot tests in `main.rs` and
  `identity.rs`.

### Task 3.1: mod freshness and the rate-limit tail

`register.js` keeps one baseline per module load (`{session, used, costUsd}`)
and a sticky `spendLimit` flag. On `session.measure` only, right after the
session-id step and before the window guard, dedupe, clock check and
in-flight skip, `observe` sets the flag from any `spend_limit` window,
evaluates D2 rules 1 and 2 with path A (`'rateLimits'` in `changed` and a
`five_hour` or `seven_day` finite used value that differs from the baseline
or is new) or path B (`'cost'` in `changed`, both totals finite and
non-negative, the new one strictly greater), then replaces the baseline. A
non-finite or non-number used value counts as absent, so `NaN` never makes
a measurement fresh. After `lastSeq = seq`, a fresh measurement appends
`rateTail(e, seq)`: integer-tenths percent text, the ISO pattern with
month, day (leap years), hour, minute, second and offset checks, integer
days-from-civil conversion with the fraction truncated, the exact
`S < r <= S + D + 3600` bound from that `seq`, `five_hour` first, a
repeated kind dropping the tail; the tail goes into the same
`$.process.run` argv. `session.end` also clears the baseline and the flag.
The cost total is held only in the baseline. The header comment states
the rule. No seq, in-flight or confirmed-key logic changed.

- Builder fix in `tests/test_claude_mod.mjs`: `measure(window, fields)`
  builds `rateLimits: []`, `changed: ['context']` and `cost: {usd: 3.217}`
  (a value no used or reset text can contain), so every existing test's
  argv stays at four values; a `step` helper settles each run so the
  in-flight skip does not hide a measurement.
- New tests: `a window that moved a whole point sends both windows after
  the window (path A)` (also an appearing `seven_day` window, an unchanged
  one sending no tail, the exact ten-value argv and the timeout) and
  `a strictly grown cost total sends the unchanged windows (path B)` (an
  equal total named as changed sends no tail).
- On `7a9fefb` (its `register.js`, sha256
  `2226a688f9958101a69210267ff1fc203b03464759bae2ca64a44da4a3b5d346`, unchanged
  up to `ee3a1a6`, written to a scratch file and loaded by a copy of the
  new test file): 24 passed, 2 failed, the two failures being exactly the
  new tests. Every existing test passes there with the new builder.
- On the change (`register.js` sha256
  `4d437171ec50def56e91da772dc0078bb6226d18d81524e27b8ee6ae80a129eb`):
  `node --test tests/test_claude_mod.mjs` 26 passed, 0 failed. The payload
  is embedded with `include_str!`, so the crate was rerun:
  `cargo test --locked --offline` 292 + 22 + 6 + 61 passed, 0 failed; fmt
  and clippy (`-D warnings`) clean.

### Task 3.2: mod cases

Twelve new tests in `tests/test_claude_mod.mjs`, each asserting at least one
exact non-empty tail in the same test and session, so each fails on
`7a9fefb`; `step` gained an `absent` marker that leaves a field out of the
event:

- `rewinds, compactions and a total named without growth are not fresh;
  growth is` (rewind with `changed: ['context']`; compaction with the total
  unchanged, and with `'cost'` named at an equal and a lower total; a larger
  total without `'cost'` named, which still moves the baseline so a later
  total above the older baseline but below it is not fresh; then growth,
  and a rewind-shaped measurement with grown total, each sending the tail);
- `the first measurement after load, session.end or for another session id
  is not fresh` (each with `changed: ['context','rateLimits','cost']`, a
  non-zero total and a moved window, then a second measurement with cost
  growth sending the tail);
- `an absent or invalid cost on either side is not fresh by the cost path`
  (absent, `undefined`, `null`, a number, a string, `{}`, `usd` a string,
  `null`, `NaN`, `Infinity`, `-1`, `-Infinity`, first as the current then
  as the baseline total, then valid growth sending the tail);
- `a spend_limit window blocks every tail until session.end, cost growth or
  not` (the window run still made with four values, later measurements
  without `spend_limit` sending none, a new session's second measurement
  sending one);
- `a fresh measurement without a five_hour or seven_day window sends the
  four-value run; the cost never leaves` (also an unknown kind ignored, the
  argv counts 7, 7, 7, 10, and no argv value containing or equal to any
  cost total);
- `changed units without rateLimits or cost, a window leaving, status-only
  and reset-only changes are not fresh` (then a window that appears again
  sends the tail with the moved reset);
- `start and classic events never send rate limits or touch the baseline`;
- `a measurement skipped while a run is in flight moves the baseline and is
  not replayed` (used values and total);
- `percent text is built from integer tenths and other values drop the
  window` (0, 7, 23.5, 99.9, 100, 0.7, `0.1 + 0.2`, `23.500000001`,
  `99.90000000001`, `-0`, 57, `1.1 * 3` kept; 1.25, 0.05, `99.94999999999`,
  100.1, 100.05, -1, -0.1, 101, `NaN`, `Infinity`, `'5'`, `null`,
  `undefined`, 0.001 dropped, a valid `seven_day` beside each);
- `ISO reset times match a Date.parse oracle, and invalid ones drop the
  window` (`Z`, offsets including `+23:59`, `-23:59`, `+00:00` and
  `-00:00`, fractions of one to nine digits, month and year ends, leap days
  in 2024 kept and in 2023 and 2100 dropped; hour 24, minute and second 60,
  offsets `+24:00`, `-24:00` and `+02:60`, day 0, 31 November, 32 December,
  30 February, and shapes without a zone, with a space, lower-case `z`,
  ten fraction digits, a signed year, padding, a trailing newline, a
  compact offset or basic format, and non-strings, each placed inside the
  reset bound so only the date check drops it). Years 2000 and 2400 cannot
  be reached: 2000 fails the epoch-millisecond clock check and 2400 the
  safe-integer `seq`, so the 400-year leap rule is covered by the code
  path only;
- `reset bounds come from the sent seq, at both edges` (with a clock at
  `start + 500` ms: `S + 1` kept, `S` and `S - 1` dropped, `S + 21600` and
  `S + 608400` kept, the next second dropped; after the clock steps back an
  hour, `lastSeq + 1` sets the bound: `S + 21600` kept, `S - 1800` dropped);
- `a repeated kind drops the tail; five_hour comes first and other entries
  are ignored`.

The path B case with unchanged percentages is the 3.1 test. The source scan
(`the source uses only the mods API, with literal names`) is unchanged and
still passes: no `Date`, `import`, timers or Node APIs in `register.js`.

- On `7a9fefb` (its `register.js` loaded by a copy of the test file): 24
  passed, 14 failed, the failures being exactly the two 3.1 tests and the
  twelve above; every existing test passes there.
- Mutations of the new `register.js`, each caught and reverted: `>=` for the
  strict growth (3 tests fail), a non-sticky `spend_limit` flag (1),
  freshness always false (14), no session-id check (1), no `'cost'` in
  `changed` check (1), a repeated kind skipped instead of dropping the tail
  (1), the bound taken from an older second (1), no offset range check (1),
  no day-of-month check (1), no day-0 check (1), hour 24 accepted (1), the
  offset sign ignored (1), `session.end` not clearing the baseline and flag
  (2), no one-decimal check (1). The first offset mutation survived the
  initial case list, whose invalid offsets fell outside the reset bound;
  the cases were moved inside it.
- On the change: `node --test tests/test_claude_mod.mjs` 38 passed, 0
  failed. `register.js` is unchanged by this task, so the crate result of
  3.1 stands.

### Task 3.3: installer refresh and uninstall state

- Installer: `hooks_install/` is unchanged since `7a9fefb`; the mod entry is
  still written as `{"version":1,...}` (`claude_mod.rs:532`), and the
  payload list (`plugin.json`, `hooks.json`, `register.js`) is unchanged,
  so `tests/test_native_distribution.mjs` needs no edit (it passes, 5 of
  5). With the new `register.js` (sha256
  `4d437171ec50def56e91da772dc0078bb6226d18d81524e27b8ee6ae80a129eb`, was
  `2226a688f9958101a69210267ff1fc203b03464759bae2ca64a44da4a3b5d346`)
  embedded, all 30 `hooks_install::claude_mod_tests` pass, including the
  refresh fixtures `a_payload_change_rewrites_only_the_changed_files`,
  `fresh_install_records_the_mod_and_an_identical_reinstall_writes_nothing`,
  `a_refresh_keeps_the_directories_and_uninstall_leaves_no_mod_tree`,
  `an_interrupted_refresh_completes_on_retry` and
  `a_refresh_interrupted_from_a_to_b_completes_with_build_c`, and the
  process fixture `claude_mod_installed_by_the_cli_is_accepted_by_the_reporter`.
  A changed `register.js` is therefore an ordinary refresh.
- `uninstall.sh`: `claude-allowances.json` joins the owned state names in
  the state-directory `case` (not the plugin-file allowlist that the
  distribution test pins).
- New process fixture
  `uninstall_removes_claude_account_state_and_keeps_unknown_state`
  (`native_process.rs`, cleared environment, retired-marker path with a
  fake `omarchy-shell`): `claude-allowances.json` and `allowances.json`
  are removed, an unknown `unrelated.json` is kept byte for byte and the
  state directory holding it stays. A process fixture is used, as for the
  existing uninstall checks, in place of the QML shell harness, which
  does not run `uninstall.sh`.
- On `7a9fefb`'s `uninstall.sh` (unchanged up to `b41a9ae`, swapped in for
  one run and restored): the fixture fails at
  `assertion failed: !state.join("claude-allowances.json").exists()`.
- On the change: `cargo test --locked --offline` 292 + 22 + 6 + 62 passed,
  0 failed; fmt and clippy (`-D warnings`) clean; `bash -n uninstall.sh`
  clean; `node --test tests/test_native_distribution.mjs
  tests/test_claude_mod.mjs` 43 passed.

Lane B commits: `d587c3b` (3.1), `b41a9ae` (3.2) and the commit that
records this entry (3.3).

### Task 4.1: README wording

- AGENTS.md re-checked against D12 (task 1.3, `fa63849`): with whitespace
  runs collapsed, each of the three old sentences is absent and each new
  sentence occurs verbatim; no em dash.
- `README.md`: the summary names Codex and Claude allowances, and the
  architecture paragraph no longer says the mod reports only the context
  window (it now also passes fresh rate-limit windows, attributed locally,
  with Claude rows local-only and needing an active session).
- Plugin README: the opening line; the mod paragraph (the fresh-evidence
  tail of at most two `five_hour`/`seven_day` windows, never the cost, and
  no rate limits in pane metadata), replacing "sends no ... rate limit";
  a new "Claude allowances" section (the row and its 7-day balance, the
  three `~/.claude.json` paths, the active-session and cache-fallback
  freshness, `--claude-account-key`, a synthetic `"provider":"claude"`
  mapping under `allowances.accounts`, the shared four-account cap and
  unique ids, the refusal conditions in one list, the collector's own
  `CLAUDE_CONFIG_DIR`/legacy checks, a mismatched `HOME` or
  `XDG_STATE_HOME`, `--claude-attribution-check`, rerunning
  `--install-claude-mod` after updating and removing Claude mappings before
  a downgrade, with the older `uninstall.sh` leaving
  `claude-allowances.json`); the mapping, identity and concealment
  paragraphs; and the allowance-row list ("at most one" pacing window,
  Claude window shapes, null fields, Claude rows never in the cache, the
  probe or peer rows).
- Commands checked against the release build of this branch in a private
  temporary directory (`mktemp -d /tmp/c4x-XXXX`, mode 0700, deleted
  afterwards), each run with `env -i` and a synthetic `HOME` holding a
  0600 `.claude.json` with the fixture uuid and email and a `PRIVATE`
  marker: `--claude-account-key` exit 0, 65 bytes;
  `--claude-attribution-check` prints `ok`, exit 0, and with
  `ANTHROPIC_BASE_URL` set empty prints `environment` and the name, exit 3;
  `--claude-account-key` with `CLAUDE_CONFIG_DIR` set exits 3; the
  README's mapping JSON, pasted with the printed key into `.config.json`,
  is accepted by `--refresh-identities` (exit 0) and `.accounts.json`
  equals `{"claude-personal": <fixture email>}`; a 4 s collector run with
  it emits one `claude` row, `unavailable` with no windows (no session);
  `--install-hooks` then `--install-claude-mod` (empty `PATH` directory and
  `MISE_SYSTEM_CONFIG_DIR`) exit 0, the installed `register.js` differs
  from the source only in the substituted runtime path, and a second
  `--install-claude-mod` exits 0 with the receipt bytes and the file inode
  unchanged. No uuid, email or `PRIVATE` marker appears in any output,
  snapshot, stderr or file under the plugin root or state directory.
- Documentation only: no test is added, so no fail-on-old proof applies.
  `node --test tests/test_native_distribution.mjs` 5 passed (README.md is
  still in the installed file list).

### Task 4.2: State and QML regression guards

- No new test function. Further assertions in existing provider-neutral
  tests, as D11 directs:
  - `tests/test_omarchy_state.cjs`, `mapped weekly allowance preserves zero
    and rejects expired reset`: five Claude-shaped rows. Both windows with
    `five_hour` listed first at 80 used and `seven_day` at 40 give
    remaining 60, time remaining 50, pace +10 and `3d 12h` (balance from
    the pacing window, not list order) and a null reset count; a
    `five_hour`-only row and an unavailable row give null remaining, time
    remaining, pace and reset and "source unavailable"; 100 used on
    `seven_day` keeps remaining 0; a past `seven_day` reset nulls the
    balance and reset.
  - Same file, `providers group any configured accounts in configured
    order`: the Claude account's key is `claude:team`; a saved alias wins,
    then the legacy `claude:<label>` entry, then the pool;
    `assignAliases` gives the Claude account its own alias beside the three
    Codex accounts.
  - `tests/qml/anton/tst_popup.qml`, `test_12_provider_neutral_rows_render_generically`,
    after its existing assertions: a Codex row and three Claude rows render
    in groups `Codex`, `Claude` (`provider-claude` present); the two-window
    row shows `60%`, `↻ 3d 12h`, pace +10 and a visible fill; the
    `five_hour`-only row shows the unknown balance placeholder with no fill or pace reading; the
    unavailable row shows the placeholder and "Allowance unavailable". With
    `accountEmails` keyed only by mapping id (label differs from id), the
    concealed card shows its alias and no email in the text or accessible
    name; unconcealed it shows the fixture email; a row with no email under
    its id shows its label. `accountEmails` and `namesHidden` are restored
    in a `finally`.
- These are regression guards by design (D9 plans no presentation change),
  so they must pass on `7a9fefb` and the change; the programme's
  fail-on-old rule does not apply to them (D11, task 4.2).
  `git diff 7a9fefb -- omarchy/herdr.observatory/State.js
  omarchy/herdr.observatory/*.qml` is empty, so `providerCoupling()`, which
  scans only those files, is unchanged. With `7a9fefb`'s
  `omarchy/herdr.observatory` and `tests` extracted by `git archive` into a
  private temporary directory and the two extended test files copied in:
  `node --test tests/test_omarchy_state.cjs` 78 passed, 0 failed;
  `bash tests/run-qml.sh` 95 passed, 0 failed.
- The guards bite: with `pacingWindow` in `State.js` mutated to take the
  first listed window (restored afterwards), `test_12` fails at a compare
  and the extended State test fails, beside two existing State tests.
- On the change: `node --test tests/test_omarchy_state.cjs` 78 passed, 0
  failed; `bash tests/run-qml.sh` (private `TMPDIR`) 95 passed, 0 failed.

## After

_Pending (task 5.1)._

## Review rounds

_Pending (task 5.2)._

## Live installed check

_Pending (task 5.3)._

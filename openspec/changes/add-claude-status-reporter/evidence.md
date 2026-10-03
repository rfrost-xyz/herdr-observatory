# Evidence

Synthetic fixtures only. No private transcripts, session ids, paths, account
data or snapshots are recorded here.

## Baseline

Tasks 1.1 and 1.2, before any runtime change.

- Harness: `tests/measure_anton_popover.mjs` at `db0eae2`
  (`test(bench): count claude agents with a window and context percent`).
  Additive only: a new `window` Claude variant (standard transcripts, 4 panes
  per host) gives panes 1 and 2 of each host a bound v2 window report (the
  design D3 16-key wire, `obs_n1` `,200000,,`) and pane 4 a report bound to
  another synthetic id. Every variant now reports `claude_agents_with_window`
  and `claude_agents_with_context_percent` (local and peer); the window
  variant also reports the mismatched pane's own window and percentage. The
  `standard`, `large` and old-local fixtures and every existing metric
  definition are unchanged.
- Binary: the supplied release build of `80f6295`, sha256
  `608c8b6efc99144ca8c97f64610c3f9fbab02e64be24d50aa5e49af8ebbe56be`. It was
  built earlier by the coordinator and not rebuilt here; `git diff 80f6295
  db0eae2 -- omarchy` is empty, so suites on this tree stand for `80f6295`.
- Command: `node tests/measure_anton_popover.mjs --binary <80f6295 build>
  --seconds 15 --repeat 3` (other options default: 32 agents per host, Claude
  probes 4 agents per host, 30 s, 6 MB large), with `TMPDIR` a private
  `mktemp -d /tmp/c3-XXXX` directory and `CLAUDE_CONFIG_DIR` unset. The after
  run (task 5.1) must use the same flags.
- Runtime (medians of 3 windows): CPU 0.020 s (0.020, 0.021, 0.019), peak RSS
  4572 KiB (4556, 4572, 7836), mean snapshot 24493.2 bytes, 6 snapshots,
  8 local Herdr samples, 0 `colors.toml` opens (inotify).
- Claude probe (last snapshot, local/peer; one 30 s run per variant, not
  repeated, so CPU and RSS differences between variants are noise-level):

  | Variant | CPU s | Peak RSS KiB | Native | Window | Percentage |
  |---|---|---|---|---|---|
  | standard | 0.045 | 5136 | 4/4 | 0/0 | 0/0 |
  | large | 0.223 | 16024 | 4/4 | 0/0 | 0/0 |
  | window | 0.049 | 5104 | 4/4 | 2/2 | 0/0 |

  Window variant: mismatched pane seen 1/1, with window 0/0, with percentage
  0/0. This matches the expectation for the old binary: bound local windows
  pass through, local percentage 0, peer window present for the bound peer
  panes, peer percentage 0, mismatched pane none. Expected after the change:
  local percentage 2, peer window and percentage 0/0, mismatched 0.
- Suites on this tree: `cargo test --locked --offline` 223 + 19 + 6 + 32
  passed, 0 failed; `node --test` State, Pi hooks and distribution: 88 passed,
  0 failed; `run-qml.sh` 95 passed, 0 failed; `run-qmllint.sh` no warnings
  outside Panel.qml; `run-shell-harness.sh` failures 0. `cargo fmt --check`
  and `cargo clippy --all-targets --locked -- -D warnings` clean (local
  toolchain).

## Claude Code mods documentation (Claude Code 2.1.287)

Read on 2026-10-03 from `code.claude.com/docs/en/`. Short labels in brackets are
used in design.md.

**[mods overview]** `plugins/mods`

- A mod is a plugin whose ES module registers hooks; a small mod is
  `.claude-plugin/plugin.json`, `hooks/hooks.json` and `hooks/register.js`.
- Mods need v2.1.287 or later and are on by default. `--safe-mode` and
  `disableAllHooks` turn installed mods off; `allowManagedModsOnly` and
  `allowManagedHooksOnly` restrict them. `CLAUDE_CODE_ENABLE_FUNCTION_HOOKS` is
  ignored from 2.1.287, which supersedes the early-access flag recorded in the
  research evidence.
- `--bare`, `--safe-mode` and `disableAllHooks` stop installed mods but not
  built-in ones.
- A mod installed from the shell loads at the next session start or after
  `/reload-plugins`.
- Hooks run in terminal sessions, the Desktop Code tab (not WSL), the VS Code
  extension and `claude -p`.

**[reference]** `plugins/mods/reference`

- `hooks.json`: `"modules": ["./register.js"]`; the module is an ES module
  exporting `register(on, options)`.
- `on(event, [matcher], hook)` returns a registration with `.catch(handler)`.
  `e` is deeply frozen; `next(e)` passes the event on.
- `session.start`: once per loaded mod before the first prompt and after a
  reload of that mod; not after `/clear`, `/resume` or `/branch`.
- `session.end`: on session end and on `/clear`, `/resume` and `/branch`, with
  `e.reason`.
- `session.measure`: after each turn, and when a plan limit's percent used
  changes.
- `classic.<Event>`: each settings hook event, with `e` the hook's stdin JSON.
- `$.session` includes `id`, `model` and `usage`; `usage()` returns
  `{startedAt, context, rateLimits, cost}` with `context` holding `tokens`,
  `window` and `percent`.
- Limits: a hook's own execution time is 10 s, excluding time in `next` and in
  mods API calls other than `$.clock.sleep`; a `.catch` handler 1 s; all
  `session.end` hooks together 1.5 s; `$.process.run` defaults to 30 s.
- Settings that stop mods: `disableAllHooks`, `allowManagedModsOnly`,
  `allowManagedHooksOnly`.
- `claude plugin validate <dir>` (`--strict`, `--json`) lists a mod's events
  and calls without running it.

**[api]** `plugins/mods/api`

- Claude Code waits for `session.start` hooks before the first prompt.
- The hooks module has no Node.js APIs, no timer globals and no file or network
  access of its own; standard JavaScript and web APIs are available. Timers are
  `$.clock.after` and `$.clock.every`; `await $.clock.now()` gives milliseconds.
- `$.env.get`: write the name as a string literal.
- `$.process.run(argv)`: no shell; resolves to `{exitCode, stdout, stderr}`
  whatever the exit code; rejects if the program cannot start or is still
  running at the timeout. The page does not name the options; the published
  types below do.
- Every mods API call is itself an event that earlier mods (for example an
  organisation guard) can refuse.

**[types]** The published mods type declarations,
`mods/types/claude-code.d.ts` in `anthropics/claude-code` (`main`, last changed
by commit `684800b`, 2026-09-29), read on 2026-10-03. The reference page warns
that this copy can be older than the installed build, so task 5.3 checks the
build's own copy.

- `$.clock.now`: "Resolves milliseconds since the epoch, now."
  (`now: () => Promise<number>`).
- `$.process.run(argv, init?: ProcessRunInit)`: "Runs a command on the host by
  its argument vector (no shell) and resolves `{ exitCode, stdout, stderr }`
  once it exits, any exit code." `init` is "`{ cwd, env, stdin, timeoutMs }`
  (cwd the session's by default; timeout 30 s by default, ten minutes at
  most)".
- `ProcessRunInit`: `cwd?: string` ("absent, the session's working
  directory"); `env?: Record<string, string>` ("Variables set over the host
  process's own environment"); `stdin?: string` ("Text written to the child's
  standard input, then closed"); `timeoutMs?: number` ("How long the child may
  run before it is killed and the call rejects, in milliseconds; 30 seconds
  when absent, ten minutes at most").
- `ProcessRunResult.exitCode`: "a child ended by a signal reads as 1".
- `'session.measure'` (doc comment above `'session.measure':
  SessionMeasureInput;`, about line 3594 of the copy fetched on 2026-10-03):
  "Fires when the engine measures the session and a unit moved: after each
  main-thread turn, and when a rate-limit window moves a whole point." and
  "One at a time, a burst folding into one more."

**[loading]** `plugins/loading`

- A directory with `.claude-plugin/plugin.json` under `~/.claude/skills/` loads
  as `<name>@skills-dir`, enabled by the manifest's `defaultEnabled` unless a
  settings file sets that id. It loads in place and is never copied.
- Project-scope skills-directory plugins need workspace trust; personal-scope
  ones have no such restriction.
- Name conflicts: a `--plugin-dir` or installed marketplace plugin with the same
  manifest name replaces a skills-directory plugin.
- The page names `~/.claude/skills/` and does not say whether
  `CLAUDE_CONFIG_DIR` moves it.

The following facts were supplied with the change brief from the same
documentation set and are relied on as stated: `$.session.id()` returns the
transcript file's name; `$.session.model()` returns the model as `/model` shows
it; `session.measure`'s payload is
`{context:{tokens,window,percent} | {window}, rateLimits, cost, changed}`;
mods share one worker, a crashing mod is unloaded and three untraceable crashes
disable all user mods for the session; Anthropic can turn installed mods off
remotely; a plugin name must not start with `claude-`.

## Herdr API (protocol 22, Herdr 0.9.1)

From the bundled schema (`herdr api schema --json`), `PaneReportMetadataParams`:
required `pane_id` and `source`; optional `agent`, `seq`, `display_agent`,
`clear_display_agent`, `title`, `clear_title`, `state_labels`,
`clear_state_labels`, `applies_to_source` and `ttl_ms` (1 to 86,400,000);
`tokens` has at most 16 properties, names matching `^[A-Za-z0-9_-]{1,32}$`, and
string or null values. The Claude report therefore needs no `display_agent` or
`ttl_ms`.

## Repository facts (at `80f6295`)

- `publish_claude` starts from `telemetry_from_agent(agent)`, merges replay usage
  only when replay's `usage_seq` is not older, and removes null `window` and
  `context_percent` from the view. Replay usage (`claude::Row::usage`) has no
  `window` key, so a bound metadata window already passes through.
- `telemetry_view` nulls `context`, `window` and `context_percent` together when
  the context exceeds the window, and nulls every number when `usage_source` or
  `usage_seq` is present but `usage_seq` is invalid.
- `telemetry_from_agent` rejects a v2 report missing any of `obs_n0` to
  `obs_n3`.
- `main.rs` reads stdin for every `--report` before calling the reporter.
- `hooks_install::install` copies the prior receipt before setting its keys, so
  an added `claude_mod` key survives `--install-hooks`.
- The plugin README tells peers to run `--install-hooks --adopt-legacy-hooks`.
- `managed()` detects chezmoi only.
- `reporter.rs` `metadata()` always sets `"agent":"pi"` and a
  `display_agent` label `pi · <phase>[ · <tool>]`.
- `native.rs` `enrich_claude_panes` enriches each Claude session key once and
  copies the first pane's `_native_telemetry` to later panes on the same key.
- `hooks_install::install` (Pi) writes the extension, then the receipt, and
  rolls the extension back only when the receipt write returns an error.
- `hooks_install::uninstall` preflights the Pi extension and shim (including
  `managed()`) before deleting either; a changed file returns an error.
- `uninstall.sh` runs under `set -euo pipefail` and calls `--remove-peers`
  then `--uninstall-hooks` before any local deletion. `--remove-peers` returns
  success when `.peers.json` is absent, so a rerun after a refusal proceeds.
- `common::expand_home` reads `HOME` and falls back to an empty path; the
  state directory defaults to `XDG_STATE_HOME` or `~/.local/state`.
- The Pi extension starts the runtime with `node:child_process` `spawn`, so it
  inherits Pi's environment; the runtime root comes from `current_exe()`.
- `State.js` uses `context_percent` when present, otherwise `context / window`.
- `telemetry_from_agent` accepts `obs_v` `1` or `2`; its v1 branch reads every
  numeric key, including the window, from `obs_<key>` tokens.
- `common::atomic_write` (used by `atomic_owned_write`) writes a temporary
  file named `.anton-write-<pid>-<bits>` (both decimal) in the destination's
  own directory and unlinks it only if the process survives to do so.
- `common::owner_guard` takes `LOCK_SH`; `hooks_install::uninstall` takes no
  guard; peer removal (`packaging.rs`) calls `hooks_install::uninstall`
  directly.
- `uninstall.sh` refuses any file in the plugin root outside its allowlist
  ("Unknown plugin file remains"), and removes only allowlisted names from the
  state directory.
- `install.sh` moves the staged plugin directory into place before it runs
  `--install-hooks`.
- `cli()` (`main.rs`) parses `--root` and `--state` anywhere in argv and
  resolves `state` to a path before dispatching the command.

## Host configuration (counts and kinds only)

- On the development host, the user's mise dotfiles track several sibling
  entries under the personal skills directory individually, and the user
  settings file is tracked (encrypted). No entry covers the proposed
  `anton-observatory` directory. `mise dotfiles paths --json` returns an
  `entries` list with `path` values using `~`, beside `exclude`, `invalid`,
  `omitted`, `plaintext`, `nested` and `incomplete` lists (all empty on this
  host). `mise` accepts `-C <dir>` to fix the directory whose configuration it
  loads. This is why D5 checks mise
  entries by path containment rather than refusing the whole Claude directory.
- The home directory contains an empty `.git` directory. Git does not treat
  the Claude configuration directory as inside a repository. D5 therefore
  counts only a real repository marker.
- No file under the real Claude configuration directory was written during
  planning.

## Implementation

### Lane A: runtime (tasks 2.1 to 2.3)

Commits (signed, no attribution):

- `b660bbc` feat(runtime): show the claude context percent from a bound window
  (task 2.2 and its fixtures).
- `253d579` feat(runtime): add the claude context window reporter (task 2.1
  and its fixtures).
- `f15c107` test(runtime): split the peer claude window drop from its guard.
  The first version tested Claude, Codex and Pi in one loop, so on the old
  code it stopped at Claude and the Codex/Pi guard never ran. After the
  split, both tests were run with only the peer-drop block removed from
  `State::sample` (the rest of the tree at `f15c107`, an exact old
  `State::sample`): the Claude test fails with `(Some(200000), Some(1))`,
  and `peer_codex_and_pi_windows_are_kept` passes.

**Fail-on-old method.** The runtime source was unchanged from `80f6295` when
the tests were written (`git diff --stat 80f6295 HEAD -- omarchy/anton-runtime`
was empty), so the new tests were run before any production edit. Results on
the old code:

| Test | Kind | On `80f6295` | Reason |
|---|---|---|---|
| `claude_bound_window_gives_context_percent_rounded_half_up` | new | fails | `context_percent` null, expected 75 |
| `claude_context_over_window_keeps_context_and_drops_window` | new | fails | `context` nulled, expected 1201 |
| `claude_panes_on_one_session_key_show_their_own_windows` | new | fails | later pane window null, expected 2000 |
| `peer_claude_window_and_context_percent_are_dropped` | new | fails | peer Claude keeps `(200000, 1)` |
| `peer_codex_and_pi_windows_are_kept` | guard | passes | see below |
| `claude_report_rejects_invalid_arguments_without_socket_access` | new | fails | exit 1, expected 2 |
| `claude_report_takes_option_like_values_verbatim` | new | fails | exit 1, expected 3 |
| `claude_report_with_open_stdin_completes_the_write` | new | fails | exit 1, expected 0 |
| `claude_report_needs_only_home_and_refuses_relative_state` | new | fails | exit 1, expected 0 |
| `claude_report_writes_the_bound_window_once_and_repeats_read_only` | new | fails | exit 1, expected 0 |
| `claude_report_refuses_unbound_panes_and_missing_ownership` | new | fails | exit 1, expected 3 |
| `claude_unbound_or_incomplete_reports_give_no_window` | guard | passes | |
| `claude_unknown_replay_with_a_bound_report_invents_no_percentage` | guard | passes | |
| `pi_metadata_bytes_are_unchanged` (exact Pi wire and label) | guard | passes | |
| `pi_report_option_parsing_is_unchanged` (options after `--report pi`) | guard | passes | |

The unit tests that call APIs absent on `80f6295` cannot compile there and
were added after the implementation: `claude_metadata_has_no_label_and_only_the_window`,
`claude_arguments_are_bounded_digits_and_safe_ids`,
`claude_paths_are_absolute_without_the_cwd` (the path resolver: absolute
`HOME`; `HOME` unset, empty or relative falls back to the password database;
relative `XDG_STATE_HOME` ignored; absolute one used; relative `--state`
refused) and `claude_mod_entry_has_the_receipt_shape`.

After the change: `cargo test --locked --offline` passes 233 (lib), 21
(bin), 6 (navigation) and 39 (process) tests; `cargo fmt --check` and
`cargo clippy --all-targets --locked -- -D warnings` (local clippy 0.1.96)
are clean. Tests ran with a private `TMPDIR` and `CLAUDE_CONFIG_DIR` unset;
the reporter process fixtures use `env_clear()` with only a temporary
absolute `HOME`.

**Plan review finding applied.** The open-stdin case holds stdin open and
asserts exit 0 with exactly `pane.get` then one `pane.report_metadata` and
the full wire, so it fails on `80f6295` (exit 1) rather than passing on
timing alone.

**Guard 2 receipt shape (for lane B).** `hooks_install::claude_mod_recorded`
reads `.hooks-receipt.json`, requires `receipt_value` with
`<root>/anton-runtime` and `<home>/.pi/agent/extensions/observatory.ts`, then
`claude_mod_entry`: `version` 1; `root` equal to
`<home>/.claude/skills/anton-observatory` (`claude_mod_root`); a non-empty
`directories` list of absolute paths at or under the root, or its `skills`
parent; `files` exactly the three `CLAUDE_MOD_FILES` paths under the root,
each with a 64-hex `sha256` and, when present, a 64-hex `prior_sha256`.
Lane B's installer should write and reuse this definition.

**Implementation notes.**

- `--report claude` is recognised only as the first command, with `claude`
  next; every later value is taken verbatim and fewer or more than four exit
  2. Exit statuses 0, 2 and 3 print nothing; status 1 prints the error.
- A relative `--root` on this path exits 3 (D3 names no status for it; it
  is a path that would resolve against the cwd).
- Owner-guard, configuration-parse, lock-file and Herdr RPC errors exit 1, as
  on the Pi path; a busy `hook.lock`, no single local host and no valid mod
  receipt exit 3.
- The no-change check returns 0 when the bound metadata view has this window
  and no other numeric value (subagent fields included).
- `metadata()` builds the same object for both agents and adds
  `display_agent` only for `pi`; the Pi bytes are pinned by
  `pi_metadata_bytes_are_unchanged`.
- `claude_window` (D4) skips a zero window as well as one under the context;
  `telemetry_from_agent` already nulls a zero window, so this only guards
  the division.

### Lane B: mod and installer (tasks 3.1 to 3.5)

Commits (signed, no attribution):

- `1c1b394` feat(hooks): add the claude code context window mod (tasks 3.1
  and 3.2; `tests/test_claude_mod.mjs` is also added to the CI node step).
- `8cc0184` feat(runtime): serialise hook receipt writers with a plugin root
  lock (the D5 lock on the existing Pi writers, including
  `repair_retired`).
- `a58bbcc` refactor(runtime): run bounded commands in a directory with null
  stdin (`common::run_process_in`; `run_process` is unchanged for callers).
- `b6d0c1c` feat(runtime): install and remove the claude code mod by receipt
  (task 3.3 and its fixtures, task 3.4).
- `6fa7545` feat(plugin): install the claude code mod after the hooks (task
  3.5).

**Gates on the final tree.** `cargo test --locked --offline`: 254 (lib), 21 (bin),
6 (navigation) and 42 (process) passed. `cargo fmt --check` and `cargo clippy
--all-targets --locked -- -D warnings` clean on local clippy 0.1.96 (CI's
1.98 not run here). `node --test` State, Pi hooks, Claude mod and
distribution: 108 passed. `run-shell-harness.sh` (synthetic `HOME`, never
runs `install.sh`): failures 0. The standalone `a58bbcc` tree also passes clippy and
its 234 + 21 + 6 + 39 tests. Tests ran with a private `TMPDIR` and
`CLAUDE_CONFIG_DIR` unset; in-process installer fixtures pass an explicit
environment (`PATH` holding only a fixture directory), and CLI fixtures use
`env_clear()` with an absolute temporary `HOME` and that `PATH` only.

**Fail-on-old.**

- `tests/test_claude_mod.mjs` (18 tests) and the two new distribution tests
  run against a `git archive 80f6295` export: all fail (no payload, no
  `--install-claude-mod` in `install.sh`). The three existing distribution
  tests pass there and here (guards).
- The three new CLI fixtures in `tests/native_process.rs`
  (`claude_mod_installed_by_the_cli_is_accepted_by_the_reporter`,
  `claude_mod_install_runs_mise_from_the_home_with_null_stdin`,
  `hook_receipt_writers_refuse_as_busy_through_the_cli`) run against a
  `git archive 1aa6d44` export, whose installer code equals `80f6295` apart
  from lane A's read-only guard-2 helpers (`git diff 80f6295 1aa6d44 --
  hooks_install.rs` removes no line; `main.rs` adds no installer command):
  all three fail with `Unknown Anton command`. The busy fixture's lock
  assertion separately fails on the current tree with every `receipt_lock`
  call removed (`--install-hooks` exits 0).
- `pi_receipt_writers_refuse_as_busy_while_the_lock_is_held` fails with the
  three Pi lock calls removed (`Retired hook path appeared or is
  unavailable` instead of busy).
- The in-process mod fixtures call functions absent on `80f6295`, so each
  rule was disabled on the current tree and the named fixture run: write
  order with the manifest first, no debris deletion, `prior_sha256` not
  accepted, prior hash not taken from the bytes on disk, duplicated or
  recomputed `directories`, mise reading `entries` only, mise failing open,
  any `.git` counting, no `CLAUDE_CONFIG_DIR` check, no peer refusal, removal
  using the process `PATH` for chezmoi, `--uninstall-hooks` skipping the mod
  preflight, no symlink check on recorded directories, unrecorded entries
  allowed, no idempotence, every file rewritten, the receipt left dual, the
  placeholder not replaced, no lock on the mod writers, and an unrecorded
  mod tree accepted (changed root). Each fails its fixture; restored, all
  pass. Two mutations first survived (the write-order test compared with the
  constant under test, and the A→B→C test never read the dual receipt); both
  fixtures were tightened and now fail. Fixture for each rule:

  | Rule disabled | Fixture that fails |
  |---|---|
  | write order, manifest first | `writes_go_receipt_first_and_the_manifest_last` |
  | debris deletion; `prior_sha256` accepted | `an_interrupted_refresh_completes_on_retry`, `an_interrupted_refresh_is_removed_by_uninstall_hooks` |
  | prior hash from the bytes on disk | `a_refresh_interrupted_from_a_to_b_completes_with_build_c` |
  | `directories` kept and deduplicated | `a_retry_after_the_receipt_write_lists_each_directory_once` |
  | mise lists and fail-closed | `install_refuses_a_target_listed_by_mise_or_an_unreadable_listing` |
  | real `.git` marker only | `install_accepts_unrelated_markers_and_listings` |
  | `CLAUDE_CONFIG_DIR`; unrecorded entries; changed root | `install_refuses_targets_it_cannot_prove_are_its_own` |
  | peer root | `install_refuses_a_peer_root_and_a_missing_pi_integration` |
  | removal chezmoi through the passed `PATH` | `removal_runs_the_chezmoi_check_only` |
  | mod preflight in `--uninstall-hooks`; recorded directory symlinks | `removal_of_a_changed_file_refuses_before_any_deletion` |
  | lock on the mod writers | `mod_receipt_writers_refuse_as_busy_while_the_lock_is_held` |
  | idempotence; placeholder replaced | `fresh_install_records_the_mod_and_an_identical_reinstall_writes_nothing` |
  | only changed files rewritten | `a_payload_change_rewrites_only_the_changed_files` |
  | receipt cleaned after a dual write | `an_interrupted_refresh_completes_on_retry` |

  Not mutation-checked: `an_existing_skills_directory_is_not_recorded_or_removed`,
  `a_refresh_keeps_the_directories_and_uninstall_leaves_no_mod_tree`,
  `removal_keeps_unrecorded_files_and_reports_success` and
  `uninstall_hooks_removes_pi_the_shim_and_the_mod_together` (each calls
  functions absent on `80f6295`, so cannot pass there).
- `register.js` mutations (token checks in `.then` and `.finally`, no
  in-flight skip, no stale rule, deduplicating `session.measure`, no
  `session.end` reset, no `.jsonl` strip, no classic id check, no clock
  floor, `seq` from the clock only, in-flight set before the run, no
  terminal `.catch`, an `env` option, the window bound) each fail at least
  one node test.

**Regression guards (pass on both).** The existing Pi install, uninstall,
shim, repair and receipt fixtures, unchanged; `pi_extension_bytes_are_unchanged`
pins `hooks/observatory.ts` to the `80f6295` sha256
`d9a998b5…f2fb76`; `pi_receipt_writers_keep_the_mod_entry` (`--install-hooks`
and `--repair-retired-hooks` keep `claude_mod`); the existing distribution
tests and the pinned `install.sh` and `uninstall.sh` file lists.

**Plan review findings applied in this stage.**

- `repair_retired` takes the receipt lock as a fifth entry point and is in
  both busy fixtures.
- The plugin root is fsynced after each receipt write, and the two mod
  directories before the step 5 rewrite, so the dual-hash order also holds
  across a host crash for the renames.
- Removal's chezmoi check takes the passed environment
  (`claude_mod::chezmoi_managed`), so the chezmoi and mise removal cases run
  in-process.
- Write order is checked only through a recording writer, which also sees
  the receipt writes.
- The changed-root case (D7) is a full mod tree beside a receipt with no
  `claude_mod`, in `install_refuses_targets_it_cannot_prove_are_its_own`.

**Deviations and notes.**

- Receipt debris: install and removal delete `.anton-write-*` files only in
  the recorded mod directories. A crash during a receipt write leaves its
  temporary file in the plugin root, where `uninstall.sh` refuses it as an
  unknown file. This is the existing Pi receipt risk, unchanged; D5's "at
  most one `.anton-write-*` temporary file, which install and removal
  delete" holds for the mod directories only. `design.md` was not edited.
- `plugin.json` carries the crate version literally; the distribution test
  and `fresh_install_records_the_mod_and_an_identical_reinstall_writes_nothing`
  fail if `Cargo.toml` changes without it.
- `register.js` checks that `$.clock.now()` returns a number, and that
  `$.process.run` returned a thenable before marking a run in flight.
- mise item paths: `~` and `~/…` expand against the passed home, other
  relative paths are taken as home-relative (the command runs there with
  `-C <home>`), and `~name` or a `..` component refuses. Relative `PATH`
  entries are skipped when looking up `chezmoi` and `mise`.
- `CLAUDE_CONFIG_DIR` set to an empty string refuses.
- chezmoi is asked about the mod directory and each mod file.
- `--uninstall-claude-mod` and `--uninstall-hooks` give the same message,
  naming the path, for a changed, replaced or symlinked recorded file or
  directory.
- A kept mod directory (it holds a file Anton did not write) is reported on
  stderr; the command still succeeds.
- `--uninstall-hooks` now opens the plugin root for its lock, so with no
  plugin root at all it fails where it used to return success (no receipt).
  `uninstall.sh` and peer removal always run inside an existing root.
- The README step for existing installations (run `--install-claude-mod`
  after updating the runtime) belongs to task 4.1 and is not done here.

### Contracts and docs (tasks 4.1 and 4.2)

Commits `919ff72` and `ea1fc51` (AGENTS.md), `60322c2` (README.md and the plugin README)
and `7805fde` (State.js test and shell harness).

**4.1.** AGENTS.md (D8): the `pane.get` and `pane.report_metadata` boundary
covers the Claude Code mod's reporter and its argv-only, observe-only rules;
the reporter child is bounded and short-lived, not a daemon; the Claude context
line names the bound local mod report as the only window source, with the
rounding, smaller-window and peer rules from the delta spec; managed
configuration for the mod names mise dotfiles and Git repositories with a real
`.git` marker, with the receipt lock. The plugin README has a "Claude Code
context window mod" section (location, what it sends, binding, new session or
`/reload-plugins`, refusals, `--uninstall-claude-mod`, refused removal and
retry, the statusLine fallback), the update step for existing installations
(run `--install-claude-mod` after updating the runtime, from the plan review),
the mod in complete uninstall, and `tests/test_claude_mod.mjs` in the
development checks. The root README names the mod.

**4.2.** `claude telemetry projects a reported window percentage and none from
a window alone` in `tests/test_omarchy_state.cjs`; the shell-harness fake now
serves `claude-a` (window 200,000, context 48,000, `context_percent` 24) and
`claude-b` (window only, context unknown), and `shell.qml` checks 24 and null.
Both are regression guards, as the plan review noted: `State.js` does not
change, so both pass on `80f6295` (State suite 78 of 78 with the new test;
shell harness against a `80f6295` plugin tree, 0 failures). Sensitivity check:
with `State.js`'s `context !== null &&` guard removed, the State test fails
(0 for null) and the harness fails 4 checks. `design.md` D7 now labels these
cases as regression guards, and D8 names the update step for existing
installations. AGENTS.md's in-flight wording was corrected in `ea1fc51` to match
the spec's "Report process that never finishes" scenario (a later event may
start a new run after the bounded time).

**Gates.** `node --test` over the Pi hooks, Claude mod, State and distribution
suites: 109 passed. `run-qml.sh`: 95 passed. `run-qmllint.sh`: no warnings
outside `Panel.qml`. `run-shell-harness.sh`: 0 failures, no `--open-thread`.
`OPENSPEC_TELEMETRY=0 openspec validate --all --strict`: 4 passed. Private
`TMPDIR`, removed afterwards; `CLAUDE_CONFIG_DIR` unset. No Rust source changed
in this stage, so `cargo fmt --check` and clippy (`--all-targets --locked -- -D warnings`, local clippy) were rerun clean on HEAD.

## After

Task 5.1, on `3467190` (`docs(openspec): record the claude contracts and docs
stage`), the tree that holds every source change in tasks 1.1 to 4.2.

- Gates, all with `TMPDIR` a private `mktemp -d /tmp/c3-XXXX` directory
  (removed afterwards) and `CLAUDE_CONFIG_DIR` unset:
  - `cargo fmt --check`: clean. `cargo clippy --locked --offline
    --all-targets -- -D warnings`: clean on local clippy 0.1.96 (rustc
    1.96.0). CI's clippy 1.98 is not installed here and was not run.
  - `cargo test --locked --offline`: 254 (lib) + 21 (bin) + 6
    (`native_navigation`) + 42 (`native_process`) passed, 0 failed.
  - `node --test tests/test_*.cjs tests/test_*.mjs` (State, Pi hooks, Claude
    mod, distribution): 109 passed, 0 failed.
  - `tests/run-qml.sh`: 95 passed, 0 failed. `tests/run-qmllint.sh`: no
    warnings outside `Panel.qml`. `tests/run-shell-harness.sh`: failures 0,
    `--open-thread` 0.
  - `omarchy-plugin-validate omarchy/herdr.observatory`: exit 0.
  - `OPENSPEC_TELEMETRY=0 openspec validate --all --strict`: 4 passed,
    0 failed (INFO notes on long requirement text only).
- Binary: a fresh release build of `3467190` in a new, empty target directory
  (`cargo build --release --locked --offline`), sha256
  `5031eec063a547236783e9820026a00f911b9b67a325ef0ebb37aa0e9cc98ecf`.
- Command: the baseline's, `node tests/measure_anton_popover.mjs --binary
  <3467190 build> --seconds 15 --repeat 3`, other options default.
- Runtime (medians of 3 windows), baseline then after:

  | Metric | Baseline (`80f6295`) | After (`3467190`) |
  |---|---|---|
  | CPU s | 0.020 (0.020, 0.021, 0.019) | 0.017 (0.017, 0.020, 0.017) |
  | Peak RSS KiB | 4572 (4556, 4572, 7836) | 4468 (4468, 4704, 4448) |
  | Mean snapshot bytes | 24493.2 | 24491.5 |
  | Snapshots / local samples / `colors.toml` opens | 6 / 8 / 0 | 6 / 8 / 0 |
  | Update median ms (32 / 128 threads) | 0.0519 / 0.1239 | 0.0535 / 0.1215 |

  All differences are within run-to-run noise. Fields per thread (16/44),
  host (6/7) and allowance (10/10) are unchanged. Allowance wire bytes per
  row read 337 (baseline 336); earlier runs in this change also read 336,
  336.5 and 337 on unchanged code, so this is timing-dependent noise.
- Claude probe (last snapshot, local/peer; one 30 s run per variant):

  | Variant | CPU s | Peak RSS KiB | Native | Window | Percentage |
  |---|---|---|---|---|---|
  | standard | 0.045 (was 0.045) | 5236 (was 5136) | 4/4 | 0/0 | 0/0 |
  | large | 0.241 (was 0.223) | 14720 (was 16024) | 4/4 | 0/0 | 0/0 |
  | window | 0.048 (was 0.049) | 8424 (was 5104) | 4/4 | 2/0 (was 2/2) | 2/0 (was 0/0) |

  Window variant: mismatched pane seen 1/1, with window 0/0, with percentage
  0/0. This is the expected result: bound local reports now give a
  percentage (2), peers carry neither window nor percentage (0/0), and the
  mismatched report still gives nothing. Single-run CPU and RSS differences
  between variants are noise-level, as in the baseline; the window
  variant's RSS peak is one 50 ms sample in a single run.
- Raw output is kept outside the repository because it contains a local path.

## Review round 1 and remediation

Four review lenses. Lens 2 was clean; lens 4 raised one nit. Three blocking
and two non-blocking findings came from lenses 1 and 3. Each code or test fix
below has a test that was run against the unfixed code first and failed
there; lens 3 #3 changed documents only.

- **Sequence above the safe integer range (lens 1, blocking).** The clock
  guard had no upper bound, so a microsecond or nanosecond `clock.now()`
  passed it and `seq` (and `lastSeq`) went above `Number.MAX_SAFE_INTEGER`,
  repeating one out-of-range value from then on. `register.js` now returns
  when `seq` is not a safe integer, before `lastSeq` is assigned (`a3034a7`).
  Tests: the not-epoch-milliseconds list gains `1.7e15` and `1.7e18` (no
  run), and a new test fires each bad reading and then a valid `start`
  reading and asserts `argv[4] === String(start * 1000)`. Both failed before
  the fix. With the guard moved after `lastSeq = seq`, the second test still
  fails, so it catches a check placed after the assignment. Design D2 steps
  4 and 5, D7 and open question 7 updated.
- **Symlinked `.git` (lens 3 #1, blocking).** `git_managed` used
  `symlink_metadata` and so ignored a `.git` symlink, which Git follows. Any
  `.git` symlink in an ancestor up to the home now counts as a marker,
  whether it resolves or not, and the check never follows it (`1276886`).
  Fixtures: `~/.git` linked to a directory holding `HEAD`, `~/.claude/.git`
  linked to a `gitdir:` file, and a dangling `~/.claude/.git` link. Each
  case was run against the unfixed `git_managed` on its own (earlier cases
  removed in a scratch copy, then the files restored): each install
  succeeded and the refusal test panicked naming that case. Design D5 and
  D7, the spec requirement and task 3.4 name the rule.
- **Snapshot helpers blind to directories (lens 3 #2, blocking).** `tree()`
  in `hooks_install.rs` and `file_tree()` in `tests/native_process.rs` now
  record every entry including directories and the root itself: path, kind,
  inode and mode, with bytes only for regular files (`5a08b8d`). The
  reviewer's mutation (`create_dir_all(mod_root.join("hooks"))` before
  `claude_managed` in `install_mod`) now fails
  `install_refuses_a_target_listed_by_mise_or_an_unreadable_listing` with
  "nothing changes"; it passed before. The mutation was then removed and the
  file restored to its committed bytes. D7 records the snapshot rule.
- **Plugin-root receipt temporary file (lens 3 #3, non-blocking).** Design D5
  and the spec requirement claimed that any leftover installer temporary
  file is accepted by a retry or removal. That holds only inside recorded mod
  directories. D5 and the requirement now say so, and D5 describes the
  plugin-root case. `uninstall.sh` is unchanged. See the follow-up below.
- **Removal order (lens 3 #4, non-blocking).** A refresh appends a recreated
  `skills/` after its recorded children, and removal in reverse list order
  tried `skills/` first and left it. Removal now sorts recorded directories
  deepest first by component count (`f0ec766`). Fixture
  `a_skills_directory_recreated_after_install_is_removed`: `skills/` exists
  at install, is deleted, a second install recreates and records it last,
  and uninstall must remove it. It failed before the fix ("the recorded
  skills directory is removed"). `--uninstall-hooks` removes the mod through
  the same `claude_mod::remove`, so it gets the same order.
- **Shell harness coverage (lens 4, nit).** `tests/shell/fake-runtime.mjs`
  adds `claude-c` (context 48,000, no window, no percentage), and `shell.qml`
  checks that it projects `contextPercent === null` with 90,000 input tokens
  (`e9cdd00`). `claude-a` now supplies `context_percent` 25 against a plain
  ratio of 24: `State.js` uses a valid supplied percentage, as it does for
  Codex's reserve, so this fits the design and shows the value is passed
  through. The new checks failed (8 failures) before the fixture change.
  With `State.js` changed to recompute the ratio, the harness fails 4 checks;
  `State.js` was then restored.

**Follow-up (not fixed in this change).** `atomic_owned_write` creates its
temporary file in the destination's directory, so a crash during a receipt
write (D5 steps 2 and 5, and Pi's existing receipt writes) can leave
`.anton-write-<pid>-<bits>` in the plugin root. The installer neither deletes
nor refuses it, but `uninstall.sh` refuses any unknown plugin-root file
("Unknown plugin file remains"), so plugin removal stops until the file is
deleted by hand. The same holds on a peer, where `record_peer` and
`remove_peer` refuse unknown peer-root files after a Pi receipt write; the
Claude Code mod is never installed on a peer. The exposure predates this change (Pi receipt writes) and is
shared with Pi; this change adds receipt writes on a refresh. A fix would let
`uninstall.sh` delete regular, owned files matching
`^\.anton-write-[0-9]+-[0-9]+$` only while holding `flock -x` on the owner
marker, because other plugin-root writers (`.peer-receipt.json`,
`.accounts.json`, `.peers.json`) do not take the receipt lock.

**Gates (HEAD after the fixes).** `cargo fmt --check` clean; `cargo clippy
--all-targets --locked -- -D warnings` clean (local clippy 0.1.96; no
`Some(x).filter(|_| ..)` or argument-less `format!` added); `cargo test --locked
--offline`: 255 + 21 + 6 + 42 passed. `node --test tests/test_*.cjs
tests/test_*.mjs`: 110 passed. `tests/run-shell-harness.sh`: 0 failures, 6
rows. `OPENSPEC_TELEMETRY=0 openspec validate --all --strict`: 4 passed.
Private `TMPDIR` under `/tmp/c3f-*`, removed afterwards. No write under the
real `~/.claude` and no `claude` CLI run; every fixture is synthetic.

## Review round 2 and remediation

Four lenses on `7a615a9`: the mod (three nits), the reporter wire (one
blocking, one non-blocking), the installer (one blocking, two non-blocking,
one nit) and the collector (one blocking, one non-blocking, one nit). Every
finding was fixed; none was declined. Each code fix has a test that was run
against the unfixed code (the fix reverted or mutated in place, then the file
restored) and failed there. Test-only findings name the mutation that their
new test catches.

- **Peer output depended on reporter metadata (collector lens #1,
  blocking).** On a host probed as a peer that also runs the mod, an
  incomplete replay pass returned the window-only metadata fallback, which
  made the local's `retain_claude` drop its retained copy instead of
  re-emitting it. `--probe` now uses `NativeTelemetry::peer()`: its
  `publish_claude` starts from an empty object instead of
  `telemetry_from_agent`, the shared-session overlay passes no window, and
  `collection::normalise` gives a Claude pane without native telemetry no
  fallback on a peer (`444668a`). The whole metadata is ignored, not only the
  window: with the window alone dropped, the report's `seq` still stamped the
  sample, so peer output would still differ from change 2. Tests:
  `claude_peer_output_ignores_bound_reporter_metadata` (native: a bound pane
  is collected exactly as an unbound one on a caught-up, an incomplete and a
  restarted pass; the report's `seq` is newer than every replay time) fails
  with either half reverted, at the caught-up comparison for the metadata
  half and at the incomplete pass for the fallback half;
  `peer_claude_pane_with_a_reporter_window_keeps_the_retained_sample`
  (`main.rs`, paired with it: `collection::normalise` of the bound pane with
  no native telemetry, then `State::sample`, re-emits `total_input` 3461 on
  the peer path and loses it on the local path) fails with the fallback half
  reverted; the process fixture `claude_peer_probe_ignores_a_bound_reporter_window`
  fails with the `--probe` arm back on `NativeTelemetry::default()`. The
  shared-session copy branch of the metadata half (`self.peer` passed to
  `claude_metadata` for a later pane on the same key) had no failing test
  until review round 3 added
  `claude_peer_later_pane_on_a_shared_key_ignores_its_report`, which fails
  with that argument mutated to `false`. A change
  2 peer runtime on such a host keeps the old behaviour until it is upgraded;
  D6 records the row.
- **Local collector analysis (coordinator decision 1).** The local follower
  keeps its retained replay sample in its own binding, independent of
  metadata, and on an incomplete pass `publish_claude` merges that sample over
  the bound metadata, whose `usage_seq` is null, so the window only overlays
  it. `State::sample` keeps no Claude copy for local hosts. No loss was found;
  `claude_local_incomplete_pass_overlays_a_bound_window_on_the_retained_sample`
  pins it as a regression guard (it passes before and after).
- **Debris outside the written directories (installer lens #1,
  blocking).** `delete_debris` now looks only in the recorded
  `anton-observatory/.claude-plugin/` and `anton-observatory/hooks/`, in both
  install and removal (`166cf11`). A debris-named file in the mod root is an
  unrecorded conflict (refused, kept), and one in `skills/` is kept by
  install, `--uninstall-claude-mod` and `--uninstall-hooks`. New refusal case
  and `debris_names_outside_the_written_directories_are_kept` failed before
  the fix; with only the install path fixed the removal assertion failed
  ("--uninstall-claude-mod keeps it"). D5, D7 and the spec narrowed.
- **Null stdin not proven (reporter lens #1, blocking).** The mise fixture
  now runs the installer with a held stdin pipe (`cb6500d`). With
  `run_process_in` changed to `Stdio::inherit()` the fake `mise` wrote
  `other` and the test failed; before this change it passed under that
  mutation.
- **`CLAUDE_CONFIG_DIR` CLI wiring untested (reporter lens #2,
  non-blocking).** Process fixture
  `claude_mod_install_refuses_another_claude_config_dir_through_the_cli`:
  exit 1, stderr names `CLAUDE_CONFIG_DIR`, tree unchanged; a value naming
  `~/.claude` installs (`0580c54`). It fails with `ClaudeEnv::process()`
  reading `config_dir: None`.
- **Default paths not run as the mod runs them (collector lens #3, nit).**
  `claude_report_as_the_mod_runs_it_derives_root_and_state` runs the
  installed runtime copy with only `HOME`, no `--root` or `--state`, and
  asserts exit 0, both RPCs and `hook.lock` under
  `<home>/.local/state/herdr.observatory` (`0580c54`). It fails with the
  default state leaf mutated. D7 notes this one fixture's exception to the
  explicit `--root`/`--state` rule.
- **chezmoi-managed parents (installer lens #2, non-blocking).**
  `claude_managed` also runs the chezmoi check on `~/.claude/skills` and
  `~/.claude` (`6265f0e`). New refusal cases (a fake `chezmoi` managing only
  either) failed before the fix. Narrowed: the home directory itself is not
  asked, because it is chezmoi's destination root and `chezmoi source-path`
  succeeds for it on any chezmoi host; a fake `chezmoi` that answers only for
  the home is accepted, and that case fails if the home is included. The
  development host has no `chezmoi` on `PATH`, so task 5.3 is unaffected. D5
  and Risks record that any `dot_claude/` source state now refuses the mod.
- **Removal message through a symlink (installer lens #3,
  non-blocking).** Removal now names the outermost symlinked component:
  `Refusing Claude Code mod removal through symlink <link>; replace it with
  the real directory or file, then retry` (`2d26c5c`). "file changed" is kept
  for hash and type mismatches. `removal_through_a_symlink_refuses_and_names_the_link`
  (symlinked `~/.claude`, mod directory, `hooks/` and recorded file; each
  removal succeeds once the link is replaced) failed before the fix with the
  old message naming `.claude/skills`. D5, D7 and the plugin README updated.
- **No sync before the receipt change on removal (installer lens #4,
  nit).** Removal now syncs each surviving recorded directory and the
  surviving parent of each removed one before returning, so both
  `--uninstall-claude-mod` and `--uninstall-hooks` sync before they rewrite or
  remove the receipt (`e80e513`). `removal_syncs_the_surviving_directories`
  records the syncs through a `remove_with` seam and failed with the sync
  call removed. The ordering relative to the receipt write is structural
  (both callers write after `remove` returns) and is not separately tested;
  a power-loss reordering cannot be reproduced in a test.
- **Socket path taken verbatim (collector lens #2, non-blocking).** The
  Claude path expands `~` and `~/` in `socket_path` against the resolved home
  (`common::expand_home_in`, which `expand_home` now calls with `HOME`, so the
  collector is unchanged) and exits 3 for a path still relative (`06fd071`).
  The home is the one `claude_paths` resolved, which equals `HOME` whenever
  `HOME` is absolute; it differs from `expand_home` only where `HOME` is unset
  or relative, where `expand_home` would give a relative path. The Pi
  reporter is unchanged. `claude_report_expands_a_home_socket_and_refuses_a_relative_one`
  failed before the fix (exit 1 for `~/herdr.sock`), and its relative case
  failed (exit 0, RPC sent to the cwd socket) with only the absolute check
  removed. D3 updated.
- **Session id in the confirmed key (mod lens #1, nit).** Node test: a run
  for session A in flight across `session.end`, settling afterwards with exit
  0, then `classic.SessionStart` for session B with the same window starts a
  run whose `argv[5]` is `session-b` (`bbde839`). With the key changed to
  `String(size)` it fails. `generation` is not reset in `session.end`. D2
  step 7 records why.
- **Backward clock step (mod lens #2, nit).** A reading earlier than the
  in-flight run's `startedAt` now counts as stale (`1b21bd3`). Node test: a
  never-settling run, the clock stepped back an hour, and the next measure
  starts a second run with `seq` `start * 1000 + 1`; it failed before the
  fix. This restores run starts only: that `seq` is ahead of the stepped-back
  clock, so the reporter refuses it as a future sequence (exit 2) until the
  clock catches up. D2 step 4 says so.
- **sec-default (mod lens #3, nit).** D1, the D2 events table and the plugin
  README now say that where `sec-default@builtin` loads (Team or Enterprise
  sign-in, or managed settings), `classic.SessionStart` does not reach the
  mod, so after `/resume` the window arrives with the next turn. The source
  is the reviewer's reading of the anthropics/claude-code
  `mods/sec-default/README.md` and the mods reference on 2026-10-03; it was
  not re-read here.

**Gates (HEAD after the fixes).** `cargo fmt --check` clean; `cargo clippy
--all-targets --locked -- -D warnings` clean (local clippy 0.1.96; no
`Some(x).filter(|_| ..)` or argument-less `format!` added); `cargo test --locked
--offline`: 260 + 22 + 6 + 46 passed. `node --test tests/test_*.cjs
tests/test_*.mjs`: 112 passed. `tests/run-shell-harness.sh`: 0 failures, 6
rows. `OPENSPEC_TELEMETRY=0 openspec validate --all --strict`: 4 passed.
Private `TMPDIR` under `/tmp/c3f-*`, removed afterwards. No write under the
real `~/.claude` and no `claude` CLI run; every fixture is synthetic.

## Review round 3 and remediation

Three lenses on `6947468`: the mod (one non-blocking, one nit), the
collector (one non-blocking) and the installer (one blocking). A fourth lens
was clean. Every finding was fixed; none was declined. Each code fix has a
test that was run against the unfixed code (the fix reverted or mutated in
place, then the file restored) and failed there.

- **mise declarations outside history (installer lens, blocking).**
  `mise dotfiles paths --json` lists only history-tracked entries, so a
  `[dotfiles]` declaration of `~/.claude/skills` in copy (or template) mode
  went unnoticed and the mod was written into a directory mise copies from
  its source. The mise check now also reads every `[dotfiles]` declaration,
  whatever its mode, and refuses under the same equals/contains/is-contained
  rule (`bb359c9`; D5, D7, the spec delta, the plugin README and AGENTS.md
  in `439810f` and `31ca964`).
  - Choosing the read. On mise 2026.9.16 in a sandbox home, a config whose
    `[env]` called `exec` to create a marker file showed which commands
    evaluate templates: `mise config ls --json` and `mise dotfiles paths
    --json` created the marker (as `dotfiles status --json` does; its help
    also says template entries are rendered); `mise config get`, keyed or
    whole-file, with or without `-f`, did not, and neither did
    `config ls --tracked-configs` or `trust --show`, which do not list the
    loaded files. A template-mode entry whose source calls `exec` was not
    rendered by `config get -f`; in that sandbox `dotfiles status` did not
    render it either (its source was probably not resolved), so the probe
    shows only that `config get` does not read sources. `config get` without `-f` reads only the
    highest-precedence file, so the installer lists candidate files itself
    (D5) and reads each with `mise -C <home> config get -f <file>`, from the
    home with null stdin, bounded as before. `run_process_in` discards
    stderr, so a keyed read's "Key not found" cannot be told from a failure;
    the whole file is read instead and parsed by a minimal TOML reader.
  - Candidate coverage was checked in the sandbox against `mise config ls
    --json` (a development-time check only; the installer never runs it):
    with 24 loaded files spanning home, `.config`, `mise/`, `.mise/`,
    `conf.d` files and folder fragments, `.local` variants, `MISE_ENV=dev`
    variants and a moved system directory, every loaded file was a
    candidate (two extra candidates: `settings.toml` and `.rtx.toml`). With
    `XDG_CONFIG_HOME`, `MISE_CONFIG_DIR` and `MISE_GLOBAL_CONFIG_FILE` moved
    in turn, none was missed either.
  - Tests: `install_refuses_a_target_declared_in_mise_dotfiles_in_any_mode`
    (empty `paths` output; copy, raw inline copy, link, template, track, a
    plain string entry, the home itself, `conf.d` file and folder fragment,
    `~/.mise.local.toml`, and directories moved by `MISE_CONFIG_DIR`,
    `XDG_CONFIG_HOME` and `MISE_SYSTEM_CONFIG_DIR`; each refuses with the
    home snapshot unchanged) and `install_refuses_unreadable_mise_declarations`
    (`config get` failing, unparseable output, `[[dotfiles]]`, `~other`, an
    unlistable `conf.d`) fail with the candidate list replaced by an empty
    one. The process fixtures `claude_mod_install_refuses_a_copy_mode_mise_declaration`
    and the D7 argv fixture `claude_mod_install_runs_mise_from_the_home_with_null_stdin`
    (now asserting every run, `paths` first, then `config get -f` of the
    home's config file, each from the home with null stdin) fail the same
    way. The sibling case (`~/.claude/skills/other`, `~/.claude/settings.json`
    and `~/.bashrc` declared, with a `[dotfiles]` line inside a task's
    multi-line string) is accepted. The reader's unit tests cover each key
    form, multi-line values and header-like lines inside strings. Installer
    fixtures set `MISE_SYSTEM_CONFIG_DIR` so the host's `/etc/mise` is never
    read.
  - Real host (read-only). A throwaway harness in a private copy of the
    crate ran the same check with the host's real `mise` from the real home
    (`dotfiles paths --json` included, as the installer runs it), printing
    only counts, relations and the verdict. The first run refused
    as unreadable: one config file declares an array of tables under a
    target (`[[dotfiles."<target>".<key>]]`), which the reader had refused.
    That form names a target like any other subtable, so it is now recorded
    as a declaration and only `[[dotfiles]]` itself refuses; the reader test
    gained the case and fails under the earlier rule. After that fix the
    real install is **not refused**: every declaration and every history
    entry is unrelated to the mod directory, so no rule fired. No path or
    content from the host was recorded.
  - Finding against the round-3 brief: the retained `mise dotfiles paths
    --json` evaluates `[env]` templates when it loads the config (the
    sandbox marker above), so the mise check as a whole still runs one
    command that can execute template functions. It is kept as the
    coordinator instructed; D5 records it.
- **Overlapping hook dispatches (mod lens #1, non-blocking).** A run now
  counts as in flight while `Math.abs(now - inflight.startedAt) <= 3000`
  (`2ccf320`). Node test `overlapping dispatches start one run when the
  earlier reading arrives last`: `session.start` and `session.measure`, each
  with its own stubbed `$` whose clock the test resolves by hand; the later
  reading (5 ms later) arrives first and starts a run, then the earlier one
  arrives. It failed before the fix with 2 runs. The one-hour step-back test
  still passes. D2 step 4 and the D7 node list record the symmetric rule and
  its cost: after a backward step of 3 s or less, a run that never settles
  blocks until the clock passes `startedAt + 3000`, at most about 6 s
  (`ff63222`).
- **Open question 9 (mod lens #2, nit).** The published types answer it:
  the `'session.measure'` doc comment reads "Fires when the engine measures
  the session and a unit moved: after each main-thread turn, and when a
  rate-limit window moves a whole point." (re-fetched on 2026-10-03, quoted
  under [types] above). Open question 9, the subagent risk and task 5.3 now
  cite it and keep the live subagent check as confirmation, because the
  GitHub copy can lag the installed build (`ff63222`).
- **Shared-key peer copy untested (collector lens, non-blocking).**
  `claude_peer_later_pane_on_a_shared_key_ignores_its_report` runs
  `NativeTelemetry::peer().enrich` on `[agent(), bound(2000)]` and asserts
  the later pane's telemetry has no `window` or `context_percent`, keeps
  `context` 1201, and equals the output for `[agent(), agent()]`
  (`b621f89`). It fails with `self.peer` mutated to `false` on the copy
  branch (`"window":2000`). The round-2 entry now names it.

**Gates (HEAD after the fixes).** `cargo fmt --check` clean; `cargo clippy
--all-targets --locked -- -D warnings` clean (local clippy 0.1.96; no
`Some(x).filter(|_| ..)` or argument-less `format!` added); `cargo test --locked
--offline`: 266 + 22 + 6 + 47 passed. `node --test tests/test_*.cjs
tests/test_*.mjs`: 113 passed. `tests/run-shell-harness.sh`: 0 failures, 6
rows. `OPENSPEC_TELEMETRY=0 openspec validate --all --strict`: 4 passed.
Private `TMPDIR` under `/tmp/c3f-*`, removed afterwards. No write under the
real `~/.claude` and no `claude` CLI run; every fixture is synthetic.

## Live installed check

_Pending (task 5.3)._

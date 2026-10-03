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

_Pending._

## After

_Pending._

## Live installed check

_Pending (task 5.3)._

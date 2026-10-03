# Design

## Context

This change implements research decision D10 of the archived change
`2026-10-01-research-claude-code-parity` (the Claude reporter for the context
window), as revised at the change 3 gate:

- **Surface:** a Claude Code mod (a plugin with a hooks module) installed as a
  personal skills-directory plugin. The statusLine wrapper, D10 option A, is not
  built. It is recorded under Alternatives as the documented fallback.
- **Binding:** by Claude session id, not by model string.
  `$.session.model()` returns the model "as `/model` shows it", a display
  string, so the D10 rule that compared a canonical model with replay's last
  model is dropped. `$.session.id()` is the transcript file's name, which is
  Herdr's `agent_session.value` for kind `id`.
- **Scope:** only the context window, and so `context_percent`, for local
  Claude panes. Rate limits and identity are change 4. This change sends and
  stores no account or rate-limit data.

Change 2 (`2026-10-03-add-claude-thread-telemetry`) left the window out of
Claude replay (its D4). Its `publish_claude` (`native.rs`) builds each Claude
sample from `telemetry_from_agent(agent)` (the pane's bound v2 metadata) and
then merges replay usage. Replay usage has no `window` key, so a bound metadata
window already passes through ("a metadata value stays"). Nothing writes Claude
metadata today. The collector work here is therefore narrow: compute the
percentage, protect replay context from a stale window, and keep peer windows
unknown. It does not rebuild the merge.

Evidence and documentation citations are in `evidence.md`. Existing Codex and
Pi behaviour, output, checkpoints and the Pi extension bytes must not change.

## Goals / Non-Goals

**Goals:**

- A local Claude Code thread inside Herdr shows a context window and percentage
  after its first turn in a session that loaded the mod.
- Every failure (mod not loaded, outside Herdr, unbound pane, wrong session,
  runtime missing, Herdr down) writes no metadata and never affects Claude
  Code. Without an earlier bound report for the same session the window stays
  unknown; with one, the earlier window stays (D4).
- The installer owns exactly the mod files it wrote, proves that with hashes,
  and refuses anything else.

**Non-Goals:**

- Rate limits, allowance rows and identity (change 4).
- Remote Claude windows. Peers keep an unknown window. The v1 envelope and the
  probe are unchanged.
- The statusLine wrapper, `settings.json` edits and `enabledPlugins` edits.
- Claude Code's own `context.percent`. Anton computes its own (D4).

## Architecture

- **`hooks/claude/anton-observatory/` (new source tree):**
  `.claude-plugin/plugin.json`, `hooks/hooks.json`, `hooks/register.js`.
  `register.js` contains the line `const nativeRuntime = '';`, replaced at
  install with the JSON-quoted runtime path, exactly as `hooks/observatory.ts`
  is. All three files are embedded with `include_str!`, so the installed plugin
  directory gains no file and `install.sh` and `uninstall.sh` file lists are
  unchanged.
- **`src/main.rs`:** `--report claude` is dispatched before the existing stdin
  read and takes exactly six arguments. `State::sample` drops `window` and
  `context_percent` from peer Claude telemetry.
- **`src/reporter.rs`:** a sibling `report_claude` with argv-only input and the
  same guards as Pi. `metadata()` takes the agent as a parameter and builds the
  `display_agent` label only for Pi (D3).
- **`src/native.rs`:** `publish_claude` computes the percentage and applies the
  context-over-window rule (D4). `enrich_claude_panes` overlays each copied
  pane's own bound window (D4).
- **`src/hooks_install.rs`:** `install_claude_mod`, `uninstall_claude_mod`, the
  receipt entry and the managed-configuration checks for the mod (D5).
  `uninstall` also removes a recorded mod.
- **`omarchy/herdr.observatory/install.sh`:** after `--install-hooks`, runs
  `--install-claude-mod` and prints a warning instead of failing when it is
  refused.
- **Tests:** Rust unit and process fixtures; `tests/test_claude_mod.mjs`; State.js
  and measurement-harness additions (D7).

## Decisions

### D1. Surface, loading and naming

- The plugin directory is `~/.claude/skills/anton-observatory/`. It loads in
  place as `anton-observatory@skills-dir`, enabled by the manifest's
  `defaultEnabled` unless a settings file sets that id. Personal-scope
  skills-directory plugins need no workspace trust dialog [loading]. The name
  does not start with `claude-`.
- `plugin.json` is `{"name":"anton-observatory","version":"<crate version>",
  "description":"Reports the Claude Code context window to the Anton Herdr
  plugin","defaultEnabled":true}`. It has no `types` entry, because the mod
  uses no `$.state` and adds no namespace.
- `hooks.json` is `{"modules":["./register.js"]}` and holds no settings hooks.
- The mod loads at the next session start or `/reload-plugins` [mods overview].
  A running session that never reloads stays unknown, as does every session in
  which mods are off: an untrusted workspace (open question 8), `disableAllHooks`, `--bare`, `--safe-mode`,
  `allowManagedModsOnly`, `allowManagedHooksOnly`, the remote kill switch, a
  name conflict with a same-named `--plugin-dir` or marketplace plugin, or a
  WSL Desktop session [reference, loading]. Each fails closed.
- The installer writes only under `~/.claude/skills/anton-observatory/`. It
  refuses when `CLAUDE_CONFIG_DIR` is set in its own environment to anything
  other than `~/.claude`, because the documentation names only
  `~/.claude/skills/` (open question 2).

### D2. Mod behaviour (`register.js`)

The module follows the mods rules: an ES module exporting `register(on)`, event
names as string literals, `$` calls written in full, `$.env.get` names as string
literals, and no Node APIs, `import`, timer globals or `Date` dependence
[api, reference]. Module-scope variables are shared by its hooks.

**Events.**

| Event | Why | Window source |
|---|---|---|
| `session.measure` | Primary. Fires after each turn and after a plan-limit change, so the latest report carries the window after a `/model` switch's next turn. Never skipped on the confirmed key (step 4), so a window lost from Herdr's metadata returns on the next turn. | `e.context.window`, else `(await $.session.usage()).context.window` |
| `session.start` | Covers mod reload and a launch with `--resume`, where replay already has context before any new turn. | `(await $.session.usage()).context.window` |
| `classic.SessionStart` | Covers `/clear`, `/resume` and `/branch` inside a running process, which do not refire `session.start`. Useful mainly for `/resume`, whose transcript already has context. | `(await $.session.usage()).context.window`, only when `e.session_id` equals the session id read below |
| `session.end` | Clears the confirmed key (step 7), so the next session reports again. | none |

A fresh `/clear` has no replay context until its first turn, so its percentage
is unknown until `session.measure` anyway; `classic.SessionStart` adds only an
earlier window.

**Every hook body** is:

```js
on('session.measure', async ($, e, next) => {
  try { await sample($, e.context?.window, 'measure'); } catch { /* never affects Claude Code */ }
  return next(e);
});
```

Each hook also has a `.catch` handler that returns quietly. A hook never
returns anything other than `next(e)` and never awaits the runtime process, so
`session.start`, which Claude Code waits for before the first prompt
[api: Add a command], is not delayed.

**`sample($, window, source)`** (`source` is `measure`, `start` or `classic`):

1. `if (!nativeRuntime) return;` (an uninstalled source copy never runs).
   `if ((await $.env.get('HERDR_ENV')) !== '1') return;`
   `const pane = await $.env.get('HERDR_PANE_ID'); if (!pane) return;`
   The pane is passed as a string; the runtime validates it.
2. `let id = await $.session.id();` If it is a string ending in `.jsonl`, that
   suffix is removed once (open question 1). Any other non-string skips.
3. The window must satisfy `Number.isSafeInteger(window) && window >= 1 &&
   window <= 100000000`. Otherwise the sample is skipped. A `{window}`-only
   context is accepted, because only the window is used.
4. `key = id + ':' + window`. If `source !== 'measure'` and `key === confirmed`,
   return: the confirmed key only deduplicates `session.start` and
   `classic.SessionStart`. `session.measure` always proceeds, so a window that
   Herdr dropped from the pane's metadata (an unverified case, see Risks) is
   restored on the next turn; the runtime's no-change check (D3 step 7) keeps a
   repeat to one `pane.get` and no write.
   Next, `const now = await $.clock.now();` If
   `!Number.isSafeInteger(Math.floor(now))` or `now < 1e12`, return: a clock
   that is not epoch milliseconds fails closed instead of sending a small
   `seq` that guard 6 would refuse for good.
   If a run is in flight (`inflight !== null`) and
   `now - inflight.startedAt <= 3000`, store `pending = {id, window, source}`
   (latest wins) and return. A run older than 3 s (above the 2 s `timeoutMs`)
   is stale: it is abandoned and a new run starts. This covers a worker that
   never settles an un-awaited promise (open question 3).
5. `seq = Math.max(lastSeq + 1, Math.floor(now) * 1000)`, kept in module scope,
   so sequences are strictly increasing epoch microseconds, also across a mod
   reload (which resets `lastSeq`).
6. `const token = ++generation; inflight = {token, startedAt: now};
   pending = null;` (the sample being started is the latest, so any held
   sample is older and is dropped), then start, without awaiting:
   `$.process.run([nativeRuntime, '--report', 'claude', pane, String(seq), id, String(window)], {timeoutMs: 2000})`.
   `.then(r => { if (token === generation && r.exitCode === 0) confirmed = key; })`,
   `.catch(() => {})`, and `.finally(() => { if (token !== generation) return;
   inflight = null; start pending if its key differs from confirmed or its
   source is measure })`. A late callback from an abandoned run (its token is
   no longer current) changes nothing: it cannot set `confirmed`, clear the
   newer run's in-flight state or drain `pending`.
7. `session.end` clears `confirmed` (but not `lastSeq`), so the next session
   reports again.

**Why this shape.**

- Argv only. The published types (evidence.md, [types]) document `cwd`, `env`
  (set over the host process's own environment) and `stdin` options. The mod
  passes none of them: the session id and window are short, so stdin adds
  nothing, and leaving `env` unset means the child inherits Claude Code's
  environment unchanged. `cwd` therefore defaults to the session's working
  directory, which is why the reporter refuses relative home and state paths
  (D3).
- `timeoutMs: 2000` is the documented `ProcessRunInit.timeoutMs` [types]. The
  reporter's own waits (400 ms lock, 400 ms per RPC) finish well inside it.
- The key is confirmed only on exit status 0, which the runtime returns only
  when the pane's metadata holds this exact bound window afterwards (D3). A
  report that loses the race with Herdr's own SessionStart hook (Herdr still
  shows the previous session id) exits non-zero and is retried on the next
  event.
- If the worker does not keep an un-awaited promise alive after a hook returns
  (open question 3), neither `.then` nor `.finally` runs. The stale rule in step
  4 then lets the next event after 3 s start a new run, so at worst one event
  inside the 3 s window is dropped, and `session.measure` keeps reporting each
  later turn. The node test pins this with a promise that never settles.
- The mod reads no model, cost, rate limits, messages, files or settings, and
  makes no network call. Change 4 can extend `sample` to carry rate limits
  through its own channel; nothing here sends them.

### D3. Reporter wire (`anton-runtime --report claude`)

**Command line.** `--report claude <pane> <seq> <session-id> <window>`, exactly
six arguments. `main.rs` dispatches it before the existing `input()` call, so
stdin is never read and a closed or absent stdin cannot stall it. The Pi form
(`--report pi <pane> <seq>` with JSON stdin) is unchanged.

**Validation, before any file or socket access:**

- `valid_pane(pane)`;
- `seq` parses as `u64`, is at most 9,007,199,254,740,991 and is not in the
  future beyond `telemetry_view`'s rule;
- `safe_id(session, 128)`, the same check `claude_session` applies to Herdr's
  value;
- `window` parses as a decimal `u64` with no sign or leading `+`, in
  `1..=100_000_000`.

Any failure exits with status 2 and no output.

**Paths, before any file access.** The child inherits Claude Code's
environment and runs in the session's working directory (D2), so
`--report claude` resolves its own paths without relying on either:

- Home is `HOME` when it is set to an absolute path, otherwise the home
  directory of `getpwuid(getuid())`. This fallback is local to
  `--report claude`; the shared `expand_home`, the Pi reporter and the
  collector are unchanged.
- The state directory is `--state` when given, otherwise `XDG_STATE_HOME`
  when it is absolute (a relative value is ignored, as the XDG base directory
  rules require), otherwise `<home>/.local/state`, then the usual
  `herdr.observatory` leaf.
- An explicit relative `--state`, or a home that is still empty or relative
  after the fallback, exits 3 with no file access, so no path ever resolves
  against the Claude project directory.

If Claude Code runs with a different `XDG_STATE_HOME` from the collector, the
reporter takes a different `hook.lock`. That only weakens serialisation between
Claude reporters on the same host, which guard 6 (`obs_seq`) already orders;
the collector writes no metadata. This is recorded under Risks.

**Guards, in the Pi order:**

1. `owner_guard` on `.herdr-observatory-install`.
2. `.hooks-receipt.json` exists, passes `receipt_value`, and has a valid
   `claude_mod` entry (D5), including an entry still carrying `prior_sha256`
   values from an interrupted refresh. A mod that arrived by other means, or a
   runtime whose user uninstalled the mod, reports nothing.
3. `.config.json` has exactly one local host; otherwise status 3.
4. The state directory and `hook.lock`, with the same 400 ms bounded wait.
5. `pane.get` (400 ms, 1 MiB). The pane must have `agent == "claude"`,
   `agent_session.agent == "claude"`, `source == "herdr:claude"`,
   `kind == "id"` and `value == session`, and `telemetry::session_binding` must
   succeed. Otherwise status 3.
6. If the pane's `obs_seq` is greater than or equal to `seq`, status 3.
7. If `telemetry_from_agent(pane)` is `Some`, its bind equals this binding,
   its `window` equals `window`, and every other numeric field is null: exit 0
   without writing (the no-change check).

**Write.** One `pane.report_metadata` (400 ms), then exit 0:

```json
{"pane_id":"<pane>","source":"user:observatory","agent":"claude","seq":<seq>,
 "tokens":{"obs_v":"2","obs_bind":"<binding>","obs_seq":"<seq>",
  "obs_event":"session","obs_phase":"ready","obs_tool":null,"obs_model":null,
  "obs_result":null,"obs_usage_source":null,
  "obs_n0":",,,","obs_n1":",<window>,,","obs_n2":",,,","obs_n3":",",
  "obs_children":null,"obs_completion":null,"obs_outcomes":null}}
```

- The raw sample `{seq, event:"session", phase:"ready", window}` is run through
  `telemetry_view` first, and the tokens are produced by the existing
  `metadata` packing, so the 16-key and 80-character limits and the v2 shape
  are shared with Pi. Today `metadata()` hard-codes `"agent":"pi"` and always
  adds a `pi · <phase>` `display_agent` label. It gains an agent parameter:
  for `pi` its output is byte-identical to today; for `claude` it sets
  `"agent":"claude"` and adds no `display_agent` key.
- All four `obs_n*` groups are written, because `telemetry_from_agent` rejects
  a v2 report missing any group. Only the `window` slot is filled.
- `obs_usage_source` and `usage_seq` stay null. `publish_claude` merges replay
  usage only when replay's `usage_seq` is not older than the metadata's, so a
  metadata `usage_seq` would block replay. A null `usage_source` also keeps
  `telemetry_view` from nulling the numbers.
- `obs_model` stays null. The mod's model is a display string, and replay
  supplies `model` from the transcript.
- No `display_agent`, `title`, `state_labels` or `ttl_ms`. Herdr's schema makes
  each optional (protocol 22). The pane's label and lifecycle are untouched.
  Like Pi, the metadata has no expiry; it is bound to the session by `obs_bind`
  and replaced by the next report.
- Explicit nulls revoke any earlier keys on the pane. Only this reporter writes
  Observatory metadata on Claude panes, so nothing native replay provides is
  overwritten: replay never writes metadata, and its values are merged in the
  collector.

**Exit statuses.** 0: the bound window is in the pane's metadata. 2: invalid
arguments. 3: not applicable (not bound, other session, older `seq`, no single
local host, lock busy, no mod receipt, no absolute home or state path). 1: any other error, as today. The Pi
path's statuses are unchanged.

### D4. Collector window and percentage

**Where the window comes from.** For a local Claude pane, only from bound
reporter metadata:

- `telemetry_from_agent` accepts it only when `obs_v` is `2` and `obs_bind`
  equals `session_binding(agent)`, the hash of `claude:id:<value>`. That is the
  pane's current Claude session, so a report for an earlier session, another
  harness or another pane is ignored.
- `claude_session(agent)` must also succeed (kind `id`, safe id), as for replay.
- Context comes only from replay (change 2 D4). The reporter writes no context.
- **Panes sharing a session key.** `enrich_claude_panes` enriches each Claude
  session key once, by its first pane, and copies that `_native_telemetry` to
  later panes on the same key. The copy would carry the first pane's window
  (or its absence) to a pane with its own report. After the copy, each later
  pane removes the copied `window` and `context_percent`, takes the window from
  its own bound metadata (`telemetry_from_agent` with its own binding), and
  recomputes the percentage with the rules below, including the
  context-over-window rule. Fixtures cover both directions: only the later pane
  has a report, and two panes report different windows.

**`publish_claude` changes**, after the usage merge and before
`telemetry_view_at`:

- If `context` and `window` are both present and `context > window`, remove
  `window` and `context_percent` and keep `context`. Without this,
  `telemetry_view` nulls all three, so a stale window (for example after
  switching to a smaller-window model, before the next turn reports) would
  erase valid replay context.
- If both are present and `context <= window`, set
  `context_percent = (context * 100 + window / 2) / window` in `u128`
  arithmetic (integer division): rounded half up, at most 100, no reserve.
  For example context 1 of window 200 (0.5 %) gives 1, and context equal to
  the window gives 100. The Codex 12,000-token
  reserve applies only to Codex.
- Otherwise `context_percent` is absent. The existing omit-null step keeps
  absent values absent rather than null, as change 2 does.

Claude Code's own `context.percent` is not used: its numerator is
undocumented, and replay's context is the D4 occupancy Anton already shows
(open question 5).

**Unknown stays unknown.**

- No bound report: no window, no percentage (change 2 behaviour).
- Bound report but replay unknown (the all-null sample, an ambiguous binding or
  a deadline skip): the window may appear and the percentage stays absent.
- The collection fallback (`telemetry_from_agent` when no `_native_telemetry`
  exists) shows at most the window, with no context and so no percentage in
  `State.js`.

**Freshness.** The report's `seq` becomes the sample's `seq` when it is the
latest stamp, as for Pi. Display freshness follows `usage_seq`, so a report
never makes old usage look current. A window older than the context is
acceptable: the window changes only with the model, and the next turn reports
it.

**Peers.** `State::sample` sets `window` and `context_percent` to null for peer
agents whose harness is `claude`, after its existing revalidation. This holds
whatever the peer version, including change 2 peers that would pass a metadata
window through. The peer never installs the mod (D5).

### D5. Installer, receipt and removal

**Commands.**

- `--install-claude-mod`: install or refresh the mod. The local `install.sh`
  runs it after `--install-hooks`, with
  `|| echo "Claude Code context reporter not installed; the context dial stays unknown" >&2`,
  so a refusal never blocks the plugin.
- `--uninstall-claude-mod`: remove only the mod and its receipt entry.
- `--uninstall-hooks`: also removes a recorded mod, preflighting it with the Pi
  extension and the compatibility shim before deleting anything.

**Recommendation: a separate command, run by default by the local installer.**
The peer provisioning instructions run `--install-hooks --adopt-legacy-hooks` on
every SSH peer. If the mod rode on `--install-hooks`, re-provisioned peers would
install it and report remote windows, which this change excludes. A separate
command also keeps the Pi install path, its errors and its receipt writes
byte-identical, and lets the user remove the mod alone. `--install-claude-mod`
refuses a peer root (`herdr.observatory-peer`) as a second guard.

**Preconditions (all checked before any write):**

- `owner_guard` and `runtime_owned`, as for `--install-hooks`.
- An existing hook receipt that passes `receipt_value` and `native_extension`
  (the mod is installed after the Pi extension, never instead of it).
- `~/.claude` exists, is a real directory owned by the user and is not a
  symlink; otherwise refuse ("Claude Code configuration not found"). Every
  component from the home directory to each target is checked with the existing
  `regular` no-symlink walk.
- `CLAUDE_CONFIG_DIR` is unset or names `~/.claude` (D1).
- **Managed configuration.** A new `claude_managed(path)` runs the existing
  `managed` (chezmoi), then refuses when:
  - **mise.** If no `mise` executable is on `PATH`, the target is not
    mise-managed and this check passes. If `mise` is present, `mise dotfiles
    paths --json` runs bounded to 3 s and 1 MiB. The target is refused when an
    entry, after `~` expansion, equals, contains or is contained by the mod
    directory. The check also fails closed (refuses) when the command exits
    non-zero, times out, exceeds the bound or prints unparseable JSON, for
    example a `mise` without `dotfiles`.
  - **Git.** Any directory from the mod directory up to and including the
    home directory holds a real repository marker: a `.git` directory that
    contains a regular file `HEAD`, or a regular `.git` file whose first line
    starts with `gitdir:`. Any other `.git` entry, such as an empty `.git`
    directory, does not count. The development host has an empty `~/.git`
    directory that Git itself does not treat as a repository (evidence.md),
    so a bare `.git` existence test would refuse every install there. Bare
    dotfile repositories driven by `--git-dir`/`--work-tree` leave no marker
    and are not detected (see Review resolutions).

  The Pi path keeps calling plain `managed`, so its behaviour is unchanged.
- **Target state.** The mod directory is absent, or every entry under it is a
  regular file or directory owned by the user and recorded in the receipt's
  `claude_mod` entry, and each recorded file present has one of its accepted
  hashes (its `sha256`, or its `prior_sha256` while an interrupted refresh is
  recorded). An unrecorded entry, a modified recorded file or a directory with
  no receipt entry is a conflict: refuse and preserve it.
- **Same root.** The receipt's `claude_mod.root` must equal the mod directory
  computed now. A receipt whose runtime path differs is already refused by
  `receipt_value` as a conflicting owner, and a different plugin root
  (another `XDG_CONFIG_HOME`) has no receipt of its own, so an existing mod
  directory there is unrecorded and refused as a conflict.

**Receipt.** `.hooks-receipt.json` gains an optional key:

```json
"claude_mod":{"version":1,"root":"/abs/.claude/skills/anton-observatory",
 "directories":["/abs/.claude/skills/anton-observatory",
  "/abs/.claude/skills/anton-observatory/.claude-plugin",
  "/abs/.claude/skills/anton-observatory/hooks"],
 "files":[{"path":".../.claude-plugin/plugin.json","sha256":"..."},
  {"path":".../hooks/hooks.json","sha256":"..."},
  {"path":".../hooks/register.js","sha256":"...","prior_sha256":"..."}]}
```

- `directories` lists only directories this installer created, in creation
  order. If `~/.claude/skills` did not exist, it is created and listed first.
  A refresh keeps the prior `directories` list unchanged and adds only a
  directory it creates itself; it never recomputes the list from scratch,
  because on a refresh every directory already exists.
- `prior_sha256` is present only while a refresh is in progress (below). It
  is the hash of the bytes verified on disk at the start of that refresh.
- Older receipts without `claude_mod` stay valid. `install` already copies the
  prior receipt before setting its own keys, so a later `--install-hooks`
  preserves `claude_mod`, and older builds ignore the key.
- `register.js` must also contain the exact `const nativeRuntime = <json>;`
  declaration for this runtime, as `native_extension` requires for Pi.

**Install and refresh order (dual hash).**

1. Preflight as above. For each recorded file present, note the hash of its
   verified bytes (`on_disk`).
2. Write the receipt with the complete new `claude_mod` entry. For each file
   whose new bytes differ from `on_disk`, the entry records the new `sha256`
   and `prior_sha256 = on_disk`. A fresh install (no files) records no
   `prior_sha256`. `directories` is the prior list plus each directory that is
   missing now and will be created in step 3.
3. Create missing directories with mode 0700.
4. Write each changed file with `atomic_owned_write` (mode 0600), skipping a
   file whose bytes already match. Reinstalling identical bytes over a
   receipt that already holds exactly the new entry (no `prior_sha256`)
   performs no file write and no receipt write (idempotent); steps 2 and 5
   are skipped when the entry they would write equals the recorded one.
5. If any `prior_sha256` was recorded, rewrite the receipt with the new hashes
   only.

A crash at any step leaves every recorded file absent, at its prior hash or at
its new hash, each of which the receipt accepts. A retry restarts from step 1:
it rebuilds `prior_sha256` from the bytes verified on disk, never from the old
receipt, so an interrupted refresh from build A to B followed by an install of
build C still works. `--uninstall-hooks` and `--uninstall-claude-mod` accept
either hash too, so removal also succeeds after a crash. Pi's order (files
first, then the receipt, with rollback) was not used: its rollback runs only
when the receipt write returns an error, and a crash after the file write but
before the receipt write leaves new bytes beside old hashes, which both retry
and removal would refuse.

**Removal:**

1. Preflight, before any Pi, shim or mod deletion: every recorded file must be
   absent, or a regular, user-owned, non-symlink file with one of its accepted
   hashes. A modified or replaced recorded file refuses the whole removal,
   keeps every file and the receipt, and exits non-zero with
   `Claude Code mod file changed: <path>; restore or remove it, then retry`
   (open question 6, confirmed). This matches the Pi extension rule and avoids
   an orphan: `uninstall.sh` deletes the receipt once `--uninstall-hooks`
   returns, after which no build could prove ownership.
   Removal also runs the chezmoi check (`managed`) on each present recorded
   file, as the Pi uninstall does, and nothing else: it does not run the mise
   or `.git` checks. Removal only deletes files whose bytes match hashes Anton
   wrote, so it cannot destroy content a dotfile manager holds, and a
   fail-closed `mise` check would otherwise block every uninstall on a host
   whose `mise` lacks `dotfiles`. A fixture covers a mise-tracked mod
   directory (fake `mise` listing it) that is still removed, and a
   chezmoi-managed one that is refused.
2. Remove the recorded files, then `remove_dir` each recorded directory in
   reverse order. An absent recorded directory is fine. `remove_dir` removes
   only empty directories, so a directory holding an unrecorded file stays;
   the command reports it on stderr and still succeeds, because that file was
   never Anton's.
3. Remove the `claude_mod` entry (`--uninstall-claude-mod`), or the whole receipt
   as today (`--uninstall-hooks`).

**Plugin uninstall on a refused mod.** `uninstall.sh` runs under
`set -euo pipefail` and calls `--remove-peers`, then `--uninstall-hooks`, before
it deletes anything locally. A refused mod removal makes `--uninstall-hooks`
fail with the message above, so `uninstall.sh` stops with the plugin, the Pi
extension, the shim and the receipt intact. This is how a changed Pi extension
is handled today. Peers recorded in `.peers.json` have already been removed by
then, and `--remove-peers` is idempotent (it returns success once the peer
receipt is gone), so the user restores or deletes the named file and reruns
`uninstall.sh`, which then completes. `uninstall.sh` itself needs no change.

A loaded mod in a running Claude Code session keeps its module until reload. Its
later `process.run` fails (runtime removed) or the reporter exits 3 (no mod
receipt), and both are caught.

### D6. Compatibility

| Case | Result |
|---|---|
| New runtime, mod not installed or not loaded | Change 2 behaviour: no window. |
| Mod loaded, old runtime (no `--report claude` path) | The old runtime rejects the argument count with status 1. Nothing is written. |
| Old local, new peer | The peer never runs the mod. No change. |
| New local, change 2 peer | Peer Claude windows are dropped locally (D4). |
| Downgrade to a build without `claude_mod` support | The older `--uninstall-hooks` ignores the key and removes the receipt, leaving the mod files unowned. Recorded under Risks; reinstalling a new build first avoids it. |
| Pi extension | Byte-identical; `tests/test_pi_hooks.mjs` and the receipt fixtures stay green. |

### D7. Tests and measurement

All fixtures are synthetic: temporary homes, synthetic Herdr sockets, synthetic
session ids and panes. No test reads or writes the real `~/.claude`, and the
Rust and node harnesses set `HOME` to a temporary directory and remove
`CLAUDE_CONFIG_DIR`. No automated fixture relies on guard order to avoid real
state: every reporter process fixture passes an explicit `--root` and `--state`
under a temporary directory.

Each list is split in two. **New behaviour** tests must fail on `80f6295`
(the feature is absent there). **Regression guards** pin existing behaviour and
must pass on both `80f6295` and the change.

**Rust reporter (`reporter.rs`, `main.rs`, `tests/native_process.rs`).**

New behaviour (must fail on `80f6295`):

- Argument validation: pane, `seq` bound, unsafe ids (`.`, `/`, 129 bytes),
  windows `0`, `-1`, `+5`, `1e6`, 100,000,001 and non-ASCII digits; each exits 2
  with no socket connection.
- `--report claude` with stdin left open never blocks (process fixture with a
  held pipe, finishing well inside 1.5 s).
- Environment: a run as `env -i HOME=<temporary absolute home>` with explicit
  `--root` and `--state` under a temporary directory succeeds against the
  synthetic socket, proving nothing else in the environment is needed; an
  explicit relative `--state` under a temporary home exits 3 with no file
  created in the working directory (the fixture's cwd is a temporary directory
  it then checks is empty). The path resolver is unit-tested without a
  process: `HOME` absolute is used; `HOME` unset or relative falls back to the
  `getpwuid` home; a relative `XDG_STATE_HOME` is ignored; an absolute one is
  used. No process fixture runs without an absolute temporary `HOME`.
- Synthetic Herdr socket: wrong agent, kind `path`, other session id, older
  `obs_seq`, no mod receipt, two local hosts: each exits 3 with `pane.get` only
  or no RPC at all.
- Success: exactly `pane.get` then one `pane.report_metadata` with 16 keys,
  `agent == "claude"`, no `display_agent` key, only `window` numeric, no
  `usage_seq`, model or account field; a repeat with the same window performs
  `pane.get` only and exits 0.
- A mod receipt entry still carrying `prior_sha256` is accepted by guard 2.

Regression guards (must pass on both):

- The Pi report fixture is unchanged and still passes, including its exact
  `"agent":"pi"` and `display_agent` label from `metadata()`.

**Collector (`native.rs` Claude tests, `main.rs`).**

New behaviour (must fail on `80f6295`):

- A bound report plus caught-up replay gives `window` and `context_percent`
  rounded half up, with no reserve: 150,000 of 200,000 gives 75; context 1 of
  window 200 (a tie at 0.5) gives 1; context equal to the window gives 100.
- Context over window: context kept, window and percentage absent.
- Two panes on one session key: only the later pane has a bound report (it
  shows its window, the first pane none), and the two panes report different
  windows (each shows its own, with its own percentage).
- `State::sample` drops `window` and `context_percent` from a peer Claude agent.

Regression guards (must pass on both):

- A report bound to another session id, a v1 report, a missing `obs_n*` group
  and a Pi-bound report on a Claude pane: no window, no percentage.
- Replay unknown with a bound report: no percentage and no zero invented
  anywhere. (On `80f6295` the window already passes through; the guard
  asserts only the absent percentage and the absent zeroes.)
- Codex and Pi outputs for existing fixtures are byte-identical, and
  `State::sample` leaves peer Codex and Pi telemetry unchanged.

**Installer (`hooks_install.rs`, temporary homes).**

New behaviour (must fail on `80f6295`):

- Fresh install; identical reinstall writes nothing (mtimes and inodes
  unchanged, receipt bytes unchanged).
- Payload change from a new build, same runtime path (a fixture payload with a
  different `register.js` or `plugin.json` version): only the changed files are
  rewritten, the others keep their inodes, and the receipt ends with the new
  hashes and no `prior_sha256`.
- Changed root: an existing mod directory under a plugin root that has no
  `claude_mod` entry (another `XDG_CONFIG_HOME`) is refused as a conflict and
  preserved.
- Interrupted refresh, built as on-disk states rather than injected faults:
  (a) dual receipt written, all files old; (b) dual receipt, files mixed old
  and new; (c) dual receipt, all files new. From each state a retry succeeds
  and ends with new hashes only, and, separately, `--uninstall-hooks` succeeds
  and leaves an empty tree. A further state, a dual receipt A→B with build C
  installing, also succeeds.
- Refresh then uninstall: a refresh keeps the prior `directories` list, and a
  later `--uninstall-claude-mod` leaves no `anton-observatory/` (and no
  `skills/` when the installer created it).
- Refusals: existing unowned directory, unrecorded file, modified recorded
  file, symlinked `~/.claude`, `skills` or target; chezmoi; a fake `mise` on
  `PATH` printing a covering entry; a fake `mise` that fails and one that
  hangs past 3 s; a `.git` directory with a regular `HEAD` and a `.git` file
  starting `gitdir:` in an ancestor; `CLAUDE_CONFIG_DIR` elsewhere; peer root.
- Not refused: an empty `.git` directory in the temporary home; a `.git`
  directory without `HEAD`; no `mise` on `PATH`.
- Removal: with a modified file it refuses before any Pi, shim or mod deletion,
  keeps everything and names the path; with an extra unrecorded file it removes
  the recorded files and keeps the directory; a mod directory listed by a fake
  `mise` is still removed (removal runs chezmoi only); a chezmoi-managed one is
  refused; `--uninstall-hooks` removes Pi, the shim and the mod together after
  one preflight.

Regression guards (must pass on both):

- A receipt without `claude_mod` still installs and uninstalls Pi exactly as
  before; the Pi extension bytes are identical before and after; the existing
  Pi install, uninstall, compatibility-shim and receipt fixtures pass
  unchanged.

**Mod (`tests/test_claude_mod.mjs`, `node --test`).** It imports the payload
with the runtime placeholder replaced, calls `register(on)` with a recording
`on`, and drives hooks with a stubbed `$` (`env.get`, `session.id`,
`session.usage`, `clock.now`, `process.run`). No Claude Code binary is used.
All cases are new behaviour (the file does not exist on `80f6295`):

- the exact registered events, all string literals, and a `.catch` handler
  attached to every registration;
- no run with an empty `nativeRuntime` (the unreplaced payload), without
  `HERDR_ENV=1` or without a pane;
- argv shape `[runtime,'--report','claude',pane,seq,id,window]` and
  `{timeoutMs:2000}`, with no `env`, `cwd` or `stdin`;
- `.jsonl` stripped once; other ids passed through for the runtime to validate;
- invalid windows skipped (0, negative, fractional, string, over the bound,
  missing);
- `session.start` and `classic.SessionStart` skipped after exit 0 on the same
  key, while `session.measure` on the same key still runs; retry after a
  non-zero exit or a rejection; latest pending sample sent after an in-flight
  run;
- a `process.run` that never settles: an event inside 3 s is held as pending,
  and a later event with a new window after 3 s (stubbed clock) starts a second
  run;
- a first run that settles late, after a second run started: its `.then` does
  not set `confirmed`, and its `.finally` neither clears the second run's
  in-flight state nor drains `pending`;
- take-over drops older pending samples: run 1 hangs, event A inside 3 s is
  held, event B after 3 s starts run 2, run 2 settles, and no third run
  starts (A is never sent over B);
- `session.end` clears `confirmed`, so the next `session.start` reports again;
- strictly increasing `seq`, including equal clock readings;
- `$.clock.now()` below `1e12` (and a non-number): no run;
- `classic.SessionStart` with a mismatched `session_id` skipped;
- every hook resolves to the value of `next(e)` and never throws, even when
  every `$` call throws or rejects;
- a static scan of the source: no `import`, `require`, `setTimeout`,
  `setInterval` or `Date`, and no bare `process` identifier (one not preceded
  by `$.`, so `$.process.run` is allowed); the same not-preceded-by-`$.` rule
  applies to any other forbidden name that also exists under `$`; every
  `$.env.get` argument is a literal.

`claude plugin validate --strict --json` on a staged copy is run at the live
check, with the user's consent, not in automated tests (it needs the Claude
binary).

**State.js and the shell harness** (new behaviour): a Claude thread with
`window` and `context_percent` projects a percentage; one with only `window`
projects none. Existing State.js cases are regression guards.

**Measurement (`tests/measure_anton_popover.mjs`, additive, own commit before
code):** the Claude probe gives half of each host's Claude panes a synthetic
bound v2 window report and one pane a report bound to another id, and records
`claude_agents_with_window` and `claude_agents_with_context_percent` (local and
peer). The baseline runs the unchanged binary. Expected change: local percentage
from 0 to the bound count, peer window and percentage to 0, mismatched pane 0.
The existing CPU, RSS and snapshot metrics are recorded with `--repeat 3`.

### D8. Spec, AGENTS.md and README wording

- `harness-telemetry`:
  - "Supplementary harness reports" names the Claude Code mod, local-only.
  - "Minimal adapter lifecycle" covers the mod as Claude Code's one owned
    integration, with hash-proven removal.
  - "Scoped cumulative harness metrics": the context sentence names the Claude
    calculation.
  - "Native Claude Code transcript replay": the window and percentage are taken
    from a bound reporter sample when present, and its scenario no longer says
    "no context percentage" unconditionally.
  - A new requirement, "Claude Code context window reporter", holds the mod,
    wire, binding and installer rules.
- AGENTS.md: "Required Pi reporters may read `pane.get` …" also covers the Claude
  mod's reporter; the Claude context line names the compliant window source;
  "Installers refuse … managed configuration" names mise dotfiles and Git
  repositories with a real `.git` marker for the mod; the detached reporter child is bounded and short-lived, not
  a daemon.
- README.md and the plugin README: what the mod is, where it is installed, how to
  remove only the mod, and that the dial needs a new or reloaded session.

## Programme mapping

Change 3 of 4: `research-claude-code-parity` (archived),
`add-claude-thread-telemetry` (archived), this change, then
`add-claude-allowances-identity`. Change 4 can add rate limits to the same mod,
through a private account channel that change 4 designs and gates; this change
carries no rate-limit or account field, and its receipt and wire need no change
for that.

## Risks / Trade-offs

- **[Mods API changes]** Mods are new in 2.1.287. A renamed event or method makes
  the hook throw inside `try`, so the window stays unknown. The live check and
  `claude plugin validate` detect it; the debug log shows it.
- **[Crash budget]** Three untraceable mod crashes disable all user mods for the
  session. Every hook is wrapped and has a `.catch`, and the module does no work
  at load time beyond defining functions.
- **[Ten-second hook limit]** Hooks never await the runtime, and each `$` call
  is outside the limit.
- **[Order with Herdr's SessionStart hook]** A report before Herdr rebinds the
  pane exits 3 and is retried on the next event (D2). After `/clear` the first
  turn's `session.measure` reports.
- **[Process cost]** One short-lived runtime process per turn
  (`session.measure` is never deduplicated), plus at most one per session
  start. A repeat costs one `pane.get` and no write. Measured in evidence.md.
- **[mise check fails closed]** A host with a `mise` lacking `dotfiles` cannot
  install the mod until `mise` is updated or removed from `PATH`. This is the
  conservative reading of "refuse managed configuration". Removal does not run
  the mise check (D5), so it never blocks uninstall.
- **[Herdr clears metadata (unverified)]** Whether Herdr clears pane metadata
  on a Herdr restart, reattach or agent re-detection is unverified (research
  design, change 3 risk). Because `session.measure` always reports, a cleared
  window returns after the next turn; until then it is unknown, never zero.
  Task 5.3 exercises a restart when the user agrees.
- **[Session id in the process table]** The session id and pane are argv, so
  other local users can read them in `/proc/<pid>/cmdline` while the reporter
  runs. Accepted: the process lives for well under 2 s, runs as the same user,
  and the values are identifiers, not paths or content. The Pi path passes its
  event on stdin because it carries more fields.
- **[Different state directory]** If Claude Code's environment sets another
  absolute `XDG_STATE_HOME` than the collector's, the reporter's `hook.lock`
  differs. Only Claude reporters write Claude pane metadata and guard 6 orders
  them by `seq`, so the effect is limited to weaker serialisation (D3).
- **[Undetected Git dotfiles]** A bare dotfile repository used through
  `--git-dir`/`--work-tree` leaves no marker under the home directory and is
  not detected (D5). The files Anton writes are new, hash-recorded and
  removable, and such a repository shows them as untracked.
- **[Downgrade orphan]** See D6.
- **[Metadata without expiry]** As for Pi. `obs_bind` stops a stale report
  applying to a new session.
- **[Window and context from different moments]** The window can lag one turn
  behind a model switch; D4 keeps context when the two disagree.

## Alternatives

- **statusLine wrapper (research D10 option A).** Not built. It needs an edit to
  `~/.claude/settings.json`, which the user's dotfiles track (and encrypt), a
  wrapper that preserves the user's command byte for byte, and a detached
  reporter per render. It remains the documented fallback if mods are withdrawn
  or cannot load: the wire in D3 would serve it unchanged, with the wrapper
  passing `session_id` and `context_window.context_window_size` as argv.
- **Model-to-window table.** Rejected in research D4.
- **Binding by model string.** Dropped: the mod's model is a display string.
- **Claude Code's `context.percent`.** Not used (D4, open question 5).
- **Mod on `--install-hooks`.** Rejected because peers run it (D5).

## Open questions

1. **`$.session.id()` and `.jsonl`.** The documentation says it returns the
   transcript file's name. The mod strips one trailing `.jsonl`; `safe_id`
   rejects any dot, so a different shape fails closed. The node test pins the
   stripping. The live check confirms the match through the pane's metadata:
   `obs_bind` equals `sha256("claude:id:" + agent_session.value)` and `obs_n1`
   holds the window. The reporter exits 0 only after binding to Herdr's value,
   so a bound report implies the ids matched.
2. **Skills directory and `CLAUDE_CONFIG_DIR`.** The documentation names only
   `~/.claude/skills/`. The installer refuses when `CLAUDE_CONFIG_DIR` points
   elsewhere instead of guessing.
3. **Un-awaited promises after a hook returns.** Not documented. If the
   worker drops them, the stale-after-3-s rule (D2 step 4) still lets later
   events report; the node test pins it with a never-settling promise.
4. **`$.process.run` options.** Answered by the published types [types]:
   `ProcessRunInit` has `cwd` (default: the session's), `env` (set over the
   host process's own environment), `stdin` and `timeoutMs`. The mod uses only
   `timeoutMs`. Task 5.3 checks the build's own types copy, because the GitHub
   copy can lag the installed build.
5. **Claude Code's own `context.percent`.** Its numerator is undocumented. Anton
   computes its own from replay context and may differ from Claude Code's
   display by a few points. Change 4 or a later change can compare them.
6. **Uninstall with a modified mod file.** Confirmed at review: a modified
   recorded file refuses the whole removal and keeps everything, and
   `uninstall.sh` stops before deleting anything, as for a changed Pi
   extension (D5). Unrecorded extra files are left and reported without
   failing.
7. **`$.clock.now()` versus `Date.now()`.** Answered by the published types
   [types]: `clock.now` "resolves milliseconds since the epoch". The mod still
   fails closed below `1e12` (D2 step 4) in case a build differs, and task 5.3
   checks the build's own types copy and a report after `/reload-plugins`.
8. **Untrusted workspaces.** The change brief lists an untrusted workspace as a
   condition that turns mods off. The loading page says only that
   project-scope skills-directory plugins need workspace trust and that
   personal-scope plugins have none of those restrictions; it does not say
   whether a personal mod's hooks run in an untrusted folder. The design treats
   an untrusted workspace as fail-closed (window unknown), and the live check
   runs the Claude Code session in a trusted folder.

## Review resolutions

Plan review findings that were declined or narrowed, with the reason:

- **Detect bare-repo dotfiles (`--git-dir`/`--work-tree`).** Declined. Such a
  repository leaves no marker on disk under the home directory, and running
  `git` against guessed directories would be a heuristic. The real-marker rule
  (D5) is kept; the gap is recorded under Risks.
- **Bounded `git rev-parse` instead of the marker rule.** Not used. The
  coordinator chose the marker rule, which needs no subprocess and handles the
  empty `~/.git` on the development host.
- **Pi's install order (files first, receipt after, rollback).** Not used for
  the mod: it does not survive a crash between the file write and the receipt
  write (D5). The dual-hash order does.
- **Runtime-assigned `seq` (`max(now_us, obs_seq + 1)` under `hook.lock`).**
  Declined. The published types state that `clock.now` is epoch milliseconds,
  the mod fails closed below `1e12`, and keeping `seq` in argv keeps the Pi
  wire shape and guard 6 unchanged.
- **Baking `--state` (or home) into `register.js`.** Declined. The installer's
  environment is no more authoritative than Claude Code's, a baked path would
  go stale when the user changes `XDG_STATE_HOME`, and it would add a second
  install-time declaration to verify. The reporter instead resolves absolute
  paths itself and refuses relative ones (D3).
- **Removal running all managed checks.** Narrowed to chezmoi only, as Pi
  does (D5), because removal deletes only bytes Anton wrote and a fail-closed
  mise check would block uninstall.
- **Changed-runtime refresh test.** Replaced: a different runtime path cannot
  reach the mod code (`receipt_value` refuses it). The plan tests a payload
  change with the same runtime path and refuses a changed root instead (D7).

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
  read and takes exactly four verbatim positional values after `claude`;
  `cli()` keeps `state` optional until dispatch (D3). `State::sample` drops `window` and
  `context_percent` from peer Claude telemetry.
- **`src/reporter.rs`:** a sibling `report_claude` with argv-only input and the
  same guards as Pi. `metadata()` takes the agent as a parameter and builds the
  `display_agent` label only for Pi (D3).
- **`src/native.rs`:** `publish_claude` computes the percentage and applies the
  context-over-window rule (D4). `enrich_claude_panes` overlays each copied
  pane's own bound window (D4). A `--probe` follower
  (`NativeTelemetry::peer()`) ignores Claude reporter metadata, and
  `collection::normalise` then skips the metadata fallback for Claude panes
  (D4, Peers).
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
  WSL Desktop session [reference, loading]. Each fails closed. Separately,
  where `sec-default@builtin` loads (a Team or Enterprise sign-in, or a
  machine with managed settings), the mod loads but its
  `classic.SessionStart` hook never runs, because that guard continues
  `classic.*` events past the user tier (the anthropics/claude-code
  `mods/sec-default/README.md` and the mods reference, as read by review
  round 2 on 2026-10-03). Only the early `/resume` window is lost there; the
  next turn reports it.
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
| `session.measure` | Primary. Fires after each turn and after a plan-limit change, so the latest report carries the window after a `/model` switch's next turn. Never skipped on the confirmed key (step 4), so a window lost from Herdr's metadata returns on the next turn. | `e.context.window` (the documented payload always carries `context`; an absent one skips) |
| `session.start` | Covers mod reload and a launch with `--resume`, where replay already has context before any new turn. | `(await $.session.usage()).context.window` |
| `classic.SessionStart` | Covers `/clear`, `/resume` and `/branch` inside a running process, which do not refire `session.start`. Useful mainly for `/resume`, whose transcript already has context. Where Claude Code's built-in guard `sec-default@builtin` loads (Team or Enterprise sign-in, or managed settings), `classic.*` events continue past the user tier and never reach this personal mod, so after `/resume` the window arrives with the next turn's `session.measure`. | `(await $.session.usage()).context.window`, only when `e.session_id` equals the session id read below |
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
[api: Add a command], is not delayed. The mod starts a run only inside a hook
invocation, with that invocation's `$`; nothing is queued or started after a
hook has returned, and every promise the mod creates ends in a terminal
`.catch`.

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
   `seq` that guard 6 would refuse for good. A reading that is too large (a
   microsecond or nanosecond clock) is caught in step 5.
   If a run is in flight (`inflight !== null`),
   `now >= inflight.startedAt` and `now - inflight.startedAt <= 3000`, return:
   the sample is skipped, not held. The next `session.measure`, which is never deduplicated, carries the
   latest window. A run older than 3 s (above the 2 s `timeoutMs`) is stale:
   it is abandoned and a new run starts. This covers a worker that never
   settles an un-awaited promise (open question 3). A reading earlier than
   `startedAt` (the wall clock stepped back, for example a manual change, a
   VM restore or an NTP step) also counts as stale; otherwise a run that never
   settles would block every event until the clock passed `startedAt + 3000`
   again. This restores run starts, not reports: step 5 then sends
   `lastSeq + 1`, which is ahead of the stepped-back clock, so the reporter
   refuses it as a future `seq` (exit 2), and Herdr's `obs_seq` refuses any
   lower one, until the clock passes the earlier reading again.
5. `seq = Math.max(lastSeq + 1, Math.floor(now) * 1000)`, kept in module scope,
   so sequences are strictly increasing epoch microseconds, also across a mod
   reload (which resets `lastSeq`). If `!Number.isSafeInteger(seq)`, return
   before `lastSeq` is assigned: a clock reading above about `9.007e12`
   (microseconds or nanoseconds) or a `lastSeq + 1` overflow starts no run,
   would only be refused by the D3 bound, and must not leave an out-of-range
   `lastSeq` that every later valid reading would inherit.
6. Call `$.process.run([nativeRuntime, '--report', 'claude', pane, String(seq), id, String(window)], {timeoutMs: 2000})`
   synchronously inside `sample`, without awaiting it and never from a
   deferred callback. Only after the call returns a promise:
   `const token = ++generation; inflight = {token, startedAt: now};`, so a
   synchronous throw (caught by the hook's `try`) leaves `inflight` and
   `generation` unchanged; only `lastSeq` has advanced, which keeps `seq`
   strictly increasing.
   The chain is
   `run.then(r => { if (token === generation && r.exitCode === 0) confirmed = key; })
   .finally(() => { if (token === generation) inflight = null; })
   .catch(() => {})`, with the terminal `.catch` last, because `.finally`
   returns a new promise. The `.finally` callback starts nothing. A late
   callback from an abandoned run (its token is no longer current) changes
   nothing: it cannot set `confirmed` or clear the newer run's in-flight
   state.
7. `session.end` clears `confirmed` (but not `lastSeq` or `generation`), so the
   next session reports again. A run still in flight across the end can
   settle afterwards with its token current and put the old session's key
   back into `confirmed`; the session id in the key keeps that from
   suppressing the next session's `classic.SessionStart` for the same window.
   `generation` is not reset, because the old run's `.finally` must still
   clear `inflight`.

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
- No held sample. An earlier plan kept the latest sample skipped during a run
  and started it from `.finally`, after the hook had returned, with a `$`
  whose lifetime is undocumented and outside any `try`. It is dropped: a
  skipped sample is at most one turn old, and the next `session.measure`
  carries the current window.
- If the worker does not keep an un-awaited promise alive after a hook returns
  (open question 3), neither `.then` nor `.finally` runs. The stale rule in step
  4 then lets the next event after 3 s start a new run, so at worst the events
  inside the 3 s window are skipped, and `session.measure` keeps reporting each
  later turn. The node test pins this with a promise that never settles.
- The mod reads no model, cost, rate limits, messages, files or settings, and
  makes no network call. Change 4 can extend `sample` to carry rate limits
  through its own channel; nothing here sends them.

### D3. Reporter wire (`anton-runtime --report claude`)

**Command line.** `--report claude <pane> <seq> <session-id> <window>`.
`main.rs` dispatches it before the existing `input()` call, so stdin is never
read and a closed or absent stdin cannot stall it. The Pi form
(`--report pi <pane> <seq>` with JSON stdin) is unchanged.

- Option parsing is unchanged for every other command, including
  `--report pi`. Only `--report claude` stops it: once `cli()` reads
  `--report claude`, it takes the next four values verbatim as positionals,
  so a pane or session id spelt `--state` is a value, not an option, and
  `--root` or `--state` for this command must come before `--report`. Any
  value after the window exits 2.
- `cli()` keeps `state` as an `Option` until dispatch, so `--report claude`
  can tell an explicit `--state` (refused when relative) from an
  environment-derived one (a relative `XDG_STATE_HOME` is ignored). Other
  commands resolve the default exactly as today.

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
- The local host's `socket_path` from `.config.json` has `~` or a leading
  `~/` expanded against that home, as the collector expands it with
  `expand_home` (the same rule, here against the resolved home). A socket
  path that is still relative exits 3 before the lock or any RPC. The Pi
  reporter's socket handling is unchanged.

If Claude Code runs with a different `XDG_STATE_HOME` from the collector, the
reporter takes a different `hook.lock`. That only weakens serialisation between
Claude reporters on the same host, which guard 6 (`obs_seq`) already orders;
the collector writes no metadata. This is recorded under Risks.

**Guards, in the Pi order, with a new receipt guard 2** (the Pi reporter has
no receipt guard; it goes from `owner_guard` to `.config.json`):

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
local host, lock busy, no mod receipt, no absolute home, state or socket path). 1: any other error, as today. The Pi
path's statuses are unchanged.

### D4. Collector window and percentage

**Where the window comes from.** For a local Claude pane, only from bound
reporter metadata:

- `telemetry_from_agent` accepts it only when `obs_v` is `1` or `2` (both
  are accepted today, for migration; the reporter writes `2`) and `obs_bind`
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

A peer's own Claude output does not depend on reporter metadata either. A host
probed as a peer can also run the local plugin with the mod, so its Claude
panes can carry bound reports. `--probe` uses `NativeTelemetry::peer()`, whose
`publish_claude` starts from an empty object instead of `telemetry_from_agent`
(so neither the window nor the report's `seq`, `event` or `phase` reaches the
sample), and `collection::normalise` gives a Claude pane without
`_native_telemetry` no telemetry instead of the metadata fallback. Peer Claude
output is then byte for byte what change 2 produced. Without this, an
incomplete replay pass on such a peer returned the window-only fallback, which
the local's `retain_claude` treats as a sample: it dropped the retained copy
instead of re-emitting it, against the "Intermittent Claude Code replay"
scenario. Dropping only the window would still let the report's `seq` stamp
the sample. The local collector is unaffected: its follower keeps the retained
replay sample independently of metadata, and on an incomplete pass
`publish_claude` merges that sample over the metadata, so a bound window only
overlays it (review round 2, evidence.md).

### D5. Installer, receipt and removal

**Commands.**

- `--install-claude-mod`: install or refresh the mod. The local `install.sh`
  runs it after `--install-hooks`, with
  `|| echo "Claude Code context reporter not installed; the context dial stays unknown" >&2`,
  so a refusal never blocks the plugin.
- `--uninstall-claude-mod`: remove only the mod and its receipt entry.
- `--uninstall-hooks`: also removes a recorded mod, preflighting it with the Pi
  extension and the compatibility shim before deleting anything.

**Receipt lock.** `--install-hooks`, `--install-claude-mod`,
`--uninstall-claude-mod` and `--uninstall-hooks` (including the call from peer
removal in `packaging.rs`) all read, modify and write `.hooks-receipt.json`.
Each takes an exclusive `flock` on the plugin root directory itself (opened
`O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC`) around its whole
read-modify-write. Each command takes it once, at its public entry point in
`hooks_install` (`install`, `uninstall`, `install_claude_mod`,
`uninstall_claude_mod`), so the CLI and peer removal are both covered; the
internal helpers, such as the mod removal that `uninstall` reuses, run under
the held lock and never take it again (a second `flock` from another
descriptor of the same directory would conflict with the first). The wait is bounded to 3 s; a lock still held
then refuses with `Hook receipt busy; retry` and writes nothing. `owner_guard`
stays shared, because the running collector holds it. The directory is used
instead of a lock file because `uninstall.sh` refuses any unknown file in the
plugin root, and `install.sh` moves the staged directory into place before it
runs any installer, so the locked inode is stable. This lock is the one change
to the Pi install path: its receipt bytes are unchanged, and the busy refusal
is its only new error.

**Recommendation: a separate command, run by default by the local installer.**
The peer provisioning instructions run `--install-hooks --adopt-legacy-hooks` on
every SSH peer. If the mod rode on `--install-hooks`, re-provisioned peers would
install it and report remote windows, which this change excludes. A separate
command also keeps the Pi install path and its receipt writes byte-identical
(apart from the shared receipt lock above), and lets the user remove the mod
alone. `--install-claude-mod`
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
  `managed` (chezmoi) on the mod directory, each mod file and each ancestor
  below the home directory (`~/.claude/skills` and `~/.claude`), so a
  chezmoi-managed parent refuses as the mise and Git checks already do. The
  home directory itself is not asked: it is chezmoi's destination root, for
  which `chezmoi source-path` succeeds on any host that uses chezmoi. It then
  refuses when:
  - **mise.** If no `mise` executable is on `PATH`, the target is not
    mise-managed and this check passes. If `mise` is present,
    `mise -C <home> dotfiles paths --json` runs with null stdin, its working
    directory set to the home directory, bounded to 3 s and 1 MiB, so a
    `mise.toml` in the installer's own working directory plays no part. The
    output's `entries`, `incomplete`, `invalid`, `nested` and `omitted` lists
    are all read, because a declaration that history could not honour may
    still be meant to cover the target. `exclude` and `plaintext` are not
    read: they qualify what an entry tracks, and the entry itself already
    decides coverage. The target is refused when any item
    in those lists, after `~` expansion of its `path`, equals, contains or is
    contained by the mod directory, or when an item has no readable string
    `path`. The check also fails closed (refuses) when the command exits
    non-zero, times out, exceeds the bound, prints unparseable JSON or lacks
    `entries`, for example a `mise` without `dotfiles`.
  - **Git.** Any directory from the mod directory up to and including the
    home directory holds a real repository marker: a `.git` directory that
    contains a regular file `HEAD`, a regular `.git` file whose first line
    starts with `gitdir:`, or a `.git` symlink of any kind. Git follows a
    symlinked `.git` (for example a dotfiles setup that links the Git
    directory in), so a `.git` symlink counts as a marker whether it resolves
    or not; the check never follows it. Any other `.git` entry, such as an
    empty `.git` directory, does not count. The development host has an empty `~/.git`
    directory that Git itself does not treat as a repository (evidence.md),
    so a bare `.git` existence test would refuse every install there. Bare
    dotfile repositories driven by `--git-dir`/`--work-tree` leave no marker
    and are not detected (see Review resolutions).

  `claude_managed` takes the environment it consults (`PATH` and
  `CLAUDE_CONFIG_DIR`) as a parameter and uses that `PATH` for both the
  chezmoi and the mise lookups; the CLI passes the process environment.
  `managed` stays a thin wrapper that passes the process `PATH`, so the Pi
  path keeps its behaviour.
- **Installer debris.** Inside the recorded mod directories that receive
  atomic writes, `anton-observatory/.claude-plugin/` and
  `anton-observatory/hooks/`, a regular, user-owned, non-symlink file whose
  name matches `^\.anton-write-[0-9]+-[0-9]+$` is Anton's own `atomic_write`
  temporary file (`.anton-write-<pid>-<bits>`, created in the destination's
  directory and left behind only when the process dies before its
  `unlinkat`). After the preconditions above pass, install deletes such files
  before the target-state check. The receipt lock serialises installers, so no
  live write's temporary file can be deleted. No file is written directly into
  `skills/` or the mod root, so a debris-named entry there cannot be Anton's:
  in the mod root it is an unrecorded conflict (refused and kept), and in
  `skills/`, which the target-state check does not walk, it is kept. Any other
  unrecorded entry remains a conflict.
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
  directory it creates itself that the list does not already hold; it never
  recomputes the list from scratch, because on a refresh every directory
  already exists.
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
   missing now, will be created in step 3 and is not already listed (a retry
   after a crash that followed this step finds them listed), in creation
   order.
3. Create missing directories with mode 0700.
4. Write each changed file with `atomic_owned_write` (mode 0600), skipping a
   file whose bytes already match, in the order `hooks/hooks.json`,
   `hooks/register.js`, then `.claude-plugin/plugin.json` last, so a Claude
   Code session that starts during a fresh install, or after a crash between
   writes, finds no manifest whose module is missing. Reinstalling identical bytes over a
   receipt that already holds exactly the new entry (no `prior_sha256`)
   performs no file write and no receipt write (idempotent); steps 2 and 5
   are skipped when the entry they would write equals the recorded one.
5. If any `prior_sha256` was recorded, rewrite the receipt with the new hashes
   only.

A crash at any step leaves every recorded file absent, at its prior hash or at
its new hash, each of which the receipt accepts. A crash during a file write
in step 4 can also leave one `.anton-write-*` temporary file in
`.claude-plugin/` or `hooks/`, which install and removal delete as debris. A crash during a
receipt write in step 2 or 5 can leave one in the plugin root instead; that
file is outside this guarantee: the mod installer neither deletes it nor
refuses because of it, but `uninstall.sh` refuses any unknown plugin-root file, so
plugin removal then stops until it is deleted by hand. Pi's receipt writes
already had this exposure; it is recorded as a follow-up in evidence.md.
A retry restarts from step 1:
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
   absent, or a regular, user-owned file with one of its accepted hashes, read
   through the same `regular()` walk the install path uses, so a symlink in
   any component from the home directory (for example a symlinked
   `~/.claude`, `anton-observatory/` or `hooks/`) refuses the removal. That
   refusal names the outermost symlinked component:
   `Refusing Claude Code mod removal through symlink <link>; replace it with the real directory or file, then retry`,
   because removing a recorded path below the link could never satisfy the
   check. A modified or replaced recorded file refuses the whole removal,
   keeps every file and the receipt, and exits non-zero with
   `Claude Code mod file changed: <path>; restore or remove it, then retry`
   (open question 6, confirmed); that message is reserved for hash and type
   mismatches. This matches the Pi extension rule and avoids
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
2. Remove the recorded files and any installer debris (defined above, so only
   in `.claude-plugin/` and `hooks/`), then `remove_dir` each recorded
   directory, deepest
   first (by path component count). A refresh appends a recreated parent such
   as `skills/` after its recorded children, so list order alone would try the
   parent while it still holds them. Debris is deleted only after the preflight passes, so a refused
   removal still keeps everything. An absent recorded directory is fine. `remove_dir` removes
   only empty directories, so a directory holding an unrecorded file stays;
   the command reports it on stderr and still succeeds, because that file was
   never Anton's.
   Each recorded directory that survives, and the surviving parent of each
   one removed, is then synced (`fsync`), so the receipt change in step 3
   cannot reach disk ahead of the unlinks, the same ordering reason the
   install path syncs for.
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
| New local, change 2 peer on a host that also runs this change's mod locally | The old peer runtime returns the window-only metadata fallback on an incomplete pass, so the local drops its retained copy for that pass instead of re-emitting it; the values are unknown until the next caught-up pass. The local cannot tell this from an all-null sample. Upgrading the peer runtime fixes it. |
| Downgrade to a build without `claude_mod` support | The older `--uninstall-hooks` ignores the key and removes the receipt, leaving the mod files unowned. Recorded under Risks; reinstalling a new build first avoids it. |
| Pi extension | Byte-identical; `tests/test_pi_hooks.mjs` and the receipt fixtures stay green. |

### D7. Tests and measurement

All fixtures are synthetic: temporary homes, synthetic Herdr sockets, synthetic
session ids and panes. No test reads or writes the real `~/.claude`, and the
Rust and node harnesses set `HOME` to a temporary directory and remove
`CLAUDE_CONFIG_DIR`. No automated fixture relies on guard order to avoid real
state: every reporter process fixture passes an explicit `--root` and `--state`
under a temporary directory, before `--report` (as the existing fixture
helper already does), except one that runs the installed runtime copy as the
mod does, whose root is its own temporary directory and whose state derives
from the temporary `HOME`.

Installer fixtures never consult the host's `mise` or `chezmoi` and never call
`std::env::set_var`, which races under the parallel test runner. In-process
fixtures pass `claude_managed` an explicit environment (a `PATH` holding only a
temporary directory with the fake `mise` or `chezmoi`, or none, and the
`CLAUDE_CONFIG_DIR` value under test). CLI-level cases run as process fixtures
with `env_clear()` and an explicit `HOME` and `PATH`, as
`tests/native_process.rs` already does with `.env(...)`.

Each list is split in two. **New behaviour** tests must fail on `80f6295`
(the feature is absent there). **Regression guards** pin existing behaviour and
must pass on both `80f6295` and the change.

**Rust reporter (`reporter.rs`, `main.rs`, `tests/native_process.rs`).**

New behaviour (must fail on `80f6295`):

- Argument validation: pane, `seq` bound, unsafe ids (`.`, `/`, 129 bytes),
  windows `0`, `-1`, `+5`, `1e6`, 100,000,001 and non-ASCII digits, and an
  extra value after the window; each exits 2 with no socket connection.
- Verbatim positionals: a session id spelt `--state` (which passes `safe_id`)
  is taken as the id, not as an option, and exits 3 at the binding check; a
  pane spelt `--state` (which passes `valid_pane`) is likewise taken as the
  pane and exits 3 after `pane.get`, with no metadata write.
- `--report claude` with stdin left open never blocks (process fixture with a
  held pipe, finishing well inside 1.5 s).
- Socket path: a `~/` socket path reaches the socket under the temporary home
  (exit 0), and a relative one exits 3 with no RPC even when the working
  directory holds a socket of that name.
- As the mod runs it: the installed runtime copy with only `HOME`, no
  `--root` or `--state`, derives its root from its own path and takes
  `hook.lock` under `<home>/.local/state/herdr.observatory`.
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

- A bound v2 report plus caught-up replay gives `window` and `context_percent`
  rounded half up, with no reserve: 150,000 of 200,000 gives 75; context 1 of
  window 200 (a tie at 0.5) gives 1; context equal to the window gives 100.
- Context over window: context kept, window and percentage absent.
- Two panes on one session key: only the later pane has a bound report (it
  shows its window, the first pane none), and the two panes report different
  windows (each shows its own, with its own percentage).
- `State::sample` drops `window` and `context_percent` from a peer Claude agent.

Regression guards (must pass on both):

- A report bound to another session id, a missing `obs_n*` group and a
  Pi-bound report on a Claude pane: no window, no percentage. (v1 reports
  stay accepted for migration, as today, and are not pinned here.)
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
  and new; (c) dual receipt, all files new; (d) dual receipt, files mixed,
  and a leftover `.anton-write-<pid>-<bits>` temporary file in `hooks/`. From
  each state a retry succeeds and ends with new hashes only and no temporary
  file, and, separately, `--uninstall-hooks` succeeds and leaves an empty
  tree. A further state, a dual receipt A→B with build C installing, also
  succeeds. A retry after a crash that followed the receipt write (directories
  already listed) leaves `directories` without duplicates.
- Write order: a fresh install writes `hooks/hooks.json`, `hooks/register.js`
  and `.claude-plugin/plugin.json` in that order (asserted through inode
  creation order or a recording write hook).
- Receipt lock: with the plugin root directory locked by the fixture, each of
  `--install-hooks`, `--install-claude-mod`, `--uninstall-claude-mod` and
  `--uninstall-hooks` refuses as busy within its bound and leaves every file
  and the receipt unchanged.
- Refresh then uninstall: a refresh keeps the prior `directories` list, and a
  later `--uninstall-claude-mod` leaves no `anton-observatory/` (and no
  `skills/` when the installer created it). This includes a `skills/` that
  existed at install, was deleted, and was recreated and recorded by a later
  install after its children.
- Snapshots: the "nothing changes", "everything is kept" and "writes
  nothing" comparisons record directories as well as files (path, kind,
  inode, mode), so a directory created or removed fails them.
- Refusals: existing unowned directory, unrecorded file, modified recorded
  file, symlinked `~/.claude`, `skills` or target; chezmoi; a fake `mise` on
  `PATH` printing a covering entry; a fake `chezmoi` that manages only
  `~/.claude` or only `~/.claude/skills`; a fake `mise` that fails and one that
  hangs past 3 s; a `.git` directory with a regular `HEAD` and a `.git` file
  starting `gitdir:` in an ancestor, and a `.git` symlink to such a directory,
  to such a file, and one that does not resolve; a fake `mise` listing the target only in
  `incomplete`, `invalid`, `nested` or `omitted`; `CLAUDE_CONFIG_DIR`
  elsewhere; peer root; a file named like debris that is a symlink or does not
  match the pattern, or a debris-named file in the mod root (each kept and
  refused as unrecorded). A debris-named file in a recorded `skills/` is kept
  by install, `--uninstall-claude-mod` and `--uninstall-hooks`.
- mise invocation: the fake `mise` records its argv and working directory; the
  fixture asserts `-C <home> dotfiles paths --json`, the home directory as
  working directory and a null stdin, with the installer started from a
  different temporary directory that holds its own `mise.toml`.
- Not refused: an empty `.git` directory in the temporary home; a `.git`
  directory without `HEAD`; no `mise` on `PATH`; a fake `chezmoi` that
  answers only for the home directory.
- Removal: with a modified file it refuses before any Pi, shim or mod deletion,
  keeps everything (including any debris) and names the path; a symlinked
  `~/.claude`, `anton-observatory/`, `hooks/` or recorded file refuses the
  removal the same way, names the link, and succeeds once the link is
  replaced by the real entry; with an extra unrecorded file it removes
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
  missing, and a `session.measure` with no `e.context`);
- `session.start` and `classic.SessionStart` skipped after exit 0 on the same
  key, while `session.measure` on the same key still runs; retry after a
  non-zero exit or a rejection;
- an event during an in-flight run starts nothing, and nothing is started
  when that run settles (no held sample): the number of `process.run` calls
  stays at one until the next event after it settles;
- a `process.run` that never settles: an event inside 3 s starts nothing, and
  a later event with a new window after 3 s (stubbed clock) starts a second
  run; a clock stepped back an hour also starts a second run, with `seq`
  `lastSeq + 1`;
- a first run that settles late, after a second run started: its `.then` does
  not set `confirmed`, and its `.finally` does not clear the second run's
  in-flight state;
- take-over: run 1 hangs, event A inside 3 s is skipped, event B after 3 s
  starts run 2, run 2 settles, and no third run starts (A is never sent);
- no unhandled rejection: with a `process.on('unhandledRejection')` listener
  installed by the test, cases where `$.clock.now` throws, `$.clock.now`
  rejects, `$.process.run` throws synchronously, `$.process.run` rejects, and
  the run's result makes the `.then` callback throw; after each the test lets
  the event loop turn (`await new Promise(setImmediate)`), then asserts the
  listener saw nothing, the hook resolved to `next(e)`, and the module state
  is consistent (a synchronous `process.run` throw leaves no run in flight,
  so the next event starts one; a rejection clears the in-flight state);
- `session.end` clears `confirmed`, so the next `session.start` reports again;
  a run in flight across `session.end` that settles afterwards with exit 0
  does not suppress the next session's `classic.SessionStart` for the same
  window (the session id is in the key);
- strictly increasing `seq`, including equal clock readings;
- `$.clock.now()` below `1e12` (and a non-number), or a microsecond or
  nanosecond reading (`1.7e15`, `1.7e18`) whose `seq` is not a safe integer:
  no run; after such a too-large reading the next valid reading `start` sends
  `argv[4] === String(start * 1000)`, so the bad reading left `lastSeq`
  unchanged;
- `classic.SessionStart` with a mismatched `session_id` skipped;
- every hook resolves to the value of `next(e)` and never throws, even when
  every `$` call throws or rejects;
- a static scan of the source: no `import`, `require`, `setTimeout`,
  `setInterval` or `Date`, and no bare `process` identifier (one not preceded
  by `$.`, so `$.process.run` is allowed); the same not-preceded-by-`$.` rule
  applies to any other forbidden name that also exists under `$`; every
  `$.env.get` argument is a literal.

`claude plugin validate --strict --json` runs on a staged copy (a temporary
directory holding the installed bytes), never on the installed mod directory,
so nothing it might write lands under `~/.claude/skills/anton-observatory/`.
It runs at the live check (task 5.3), with the user's consent, not in
automated tests (it needs the Claude binary).

**State.js and the shell harness** (regression guards, since `State.js` does
not change and all three cases already hold on `80f6295`): a Claude thread with
`window` and `context_percent` projects the supplied percentage (the shell
harness fixture's value differs from the plain ratio, so a recomputation would
fail it); one with only `window` (replay context unknown) projects none; one
with replay context and no window (the mod not installed or not reported yet)
projects none.

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
  For existing installations (updates are manual and `install.sh` refuses an
  installed plugin), the plugin README says to run `--install-claude-mod`
  after updating the runtime.

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
- **[chezmoi-managed Claude directory]** A user whose chezmoi source state
  holds any file under `~/.claude` (a `dot_claude/` directory) has a managed
  `~/.claude`, so the mod is refused there, even when nothing tracks the
  skills directory. This is the conservative reading of "refuse managed
  configuration"; the development host has no `chezmoi`.
- **[Undetected Git dotfiles]** A bare dotfile repository used through
  `--git-dir`/`--work-tree` leaves no marker under the home directory and is
  not detected (D5). The files Anton writes are new, hash-recorded and
  removable, and such a repository shows them as untracked.
- **[Downgrade orphan]** See D6.
- **[Receipt lock contention]** A receipt writer waits at most 3 s for
  another and then refuses as busy; `install.sh` prints its warning and the
  user reruns. No writer holds the lock for longer than its own bounded
  checks.
- **[Subagent windows (unverified)]** See open question 9.
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
   events report; the node test pins it with a never-settling promise. The
   mod never starts work after a hook has returned, so this affects only
   whether `confirmed` and `inflight` are updated.
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
   fails closed below `1e12` (D2 step 4) and when `seq` is not a safe integer
   (D2 step 5) in case a build differs, and task 5.3
   checks the build's own types copy and a report after `/reload-plugins`.
8. **Untrusted workspaces.** The change brief lists an untrusted workspace as a
   condition that turns mods off. The loading page says only that
   project-scope skills-directory plugins need workspace trust and that
   personal-scope plugins have none of those restrictions; it does not say
   whether a personal mod's hooks run in an untrusted folder. The design treats
   an untrusted workspace as fail-closed (window unknown), and the live check
   runs the Claude Code session in a trusted folder.
9. **Subagent turns and `session.measure`.** Not documented: whether a
   subagent's turn fires `session.measure` with the subagent's
   `context.window` while `$.session.id()` still returns the parent's id. If it
   does, a subagent on a model with a different window would pass every
   reporter guard and write a wrong window. Task 5.3 starts a subagent on a
   model with a different window when the account offers one and confirms
   `obs_n1` does not change. If it changes, the change does not ship until
   `sample` filters on a payload field that identifies the main session; if it
   cannot be exercised, that is recorded as an open risk.
10. **Claude Code writing into the plugin directory.** Not documented. Any
    file Claude Code (or `claude plugin validate`) writes under the mod
    directory is unrecorded, so later refreshes would refuse and removal would
    keep the directory. Task 5.3 lists the directory after a live session and
    re-runs the installer to confirm nothing appeared.

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
  the mod fails closed below `1e12` and above the safe `seq` range, and
  keeping `seq` in argv keeps the Pi wire shape and guard 6 unchanged.
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

Second plan review:

- **Pending-sample drain.** Removed rather than guarded, by coordinator
  decision: a sample skipped during a run is not held, and the next
  `session.measure` carries the window (D2).
- **v1 reports on Claude panes.** The plan text was corrected to say
  `telemetry_from_agent` accepts versions 1 and 2, and the v1 case was dropped
  from the regression guard. Rejecting v1 was not chosen, because the
  unchanged "Scoped cumulative harness metrics" requirement accepts valid
  previous-version samples during migration.
- **Receipt lock file (`.hooks-receipt.lock`).** Narrowed to an exclusive
  `flock` on the plugin root directory. A new file in the plugin root would
  make `uninstall.sh` refuse ("Unknown plugin file remains"), and a state-file
  lock would need new allowlist entries in `uninstall.sh` and `packaging.rs`.
- **Session-measure fallback to `$.session.usage()`.** Removed from the events
  table rather than implemented: the documented payload always carries
  `context`, and an absent one is skipped (and tested).
- **Rejecting a pane value before the reporter runs (mod-side
  `valid_pane`).** Not used: the reporter reads its four values verbatim, so
  an option-like value cannot be misparsed, and the mod stays free of
  duplicated validation.
- **Settings file hash in evidence.** The live check compares the hash before
  and after but records only equality, because evidence.md holds no private
  values.

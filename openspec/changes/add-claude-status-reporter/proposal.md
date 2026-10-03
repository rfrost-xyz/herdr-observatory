# Proposal

## Why

After change 2 (`add-claude-thread-telemetry`), local and peer Claude Code
threads show deduplicated totals, last-response usage, context occupancy,
compactions, child outcomes and turn timing from native transcript replay. The
context dial stays unknown, because transcripts carry no context window and a
model-to-window table is rejected (research D4, D10). Codex reads its window from
its rollout and Pi reports it through `hooks/observatory.ts`. Claude Code needs an
equivalent reporter.

Claude Code 2.1.287 ships mods: plugins with an in-process hooks module. A mod
receives `session.measure` after each turn and can read `$.session.usage()`,
whose `context.window` is the window Claude Code itself uses. A personal
skills-directory plugin loads without a settings edit. The user chose this
surface over the statusLine wrapper (research D10 option A), so no tracked
settings file is changed.

This is change 3 of 4 in the Claude Code parity programme.

## What Changes

- Add an installer-owned Claude Code mod, `anton-observatory`, installed as a
  personal skills-directory plugin at `~/.claude/skills/anton-observatory/`
  (`.claude-plugin/plugin.json`, `hooks/hooks.json`, `hooks/register.js`). It
  loads as `anton-observatory@skills-dir` through the manifest's
  `defaultEnabled`, with no `settings.json` or `enabledPlugins` edit.
- The mod observes `session.start`, `session.measure` and `classic.SessionStart`.
  Inside a Herdr pane it starts the installed runtime with argv only:
  `anton-runtime --report claude <pane> <seq> <session-id> <window>`. It never
  blocks, alters or answers an event, and always returns `next(e)`.
- `anton-runtime --report claude` validates its arguments, checks the owner,
  the local host and the hook lock, then requires Herdr's pane to be agent
  `claude` with session kind `id` equal to the reported session id. It then
  writes one bound v2 metadata report carrying only the `window` slot. It sends
  no model, totals, `usage_seq`, `display_agent` or account data, and reads no
  stdin.
- The collector takes the window for a local Claude pane only from a bound
  reporter sample for the pane's current Claude session. It computes
  `context_percent` from replay's context and that window without the Codex
  reserve. It drops only the window and percentage, never the replay context,
  when the context exceeds the window. Unknown stays unknown, never zero.
- Peer Claude threads keep an unknown window. The local collector drops
  `window` and `context_percent` from peer Claude samples, the peer never
  installs the mod, and the v1 envelope is unchanged.
- Add `--install-claude-mod` and `--uninstall-claude-mod`. The local
  `install.sh` runs the install after `--install-hooks`, and `--uninstall-hooks`
  also removes a receipt-recorded mod. Ownership is proven by a per-file sha256
  in the existing `.hooks-receipt.json`, which stays readable by older and newer
  builds. The installer refuses targets it cannot prove are its own, symlinks,
  managed configuration (chezmoi, mise dotfiles and enclosing Git work trees)
  and peer roots. The Pi extension is unchanged byte for byte.
- Update the `harness-telemetry` spec, AGENTS.md and the READMEs. The statusLine
  wrapper is recorded only as a documented fallback.
- Measure before and after with `tests/measure_anton_popover.mjs`, adding a
  window metric for Claude agents.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `harness-telemetry`: supplementary reports extend from Pi to a local Claude
  Code mod for the context window; the adapter lifecycle owns one integration
  per harness, including the mod; Claude context percentage is defined; native
  Claude replay no longer omits the window and percentage when a bound reporter
  sample exists.

## Impact

- **Runtime:** `reporter.rs` (Claude path), `main.rs` (argv-only dispatch before
  stdin and peer window drop), `native.rs` (`publish_claude` window and
  percentage), `hooks_install.rs` (mod install, uninstall, receipt and managed
  checks).
- **Payload:** new `hooks/claude/anton-observatory/` source tree, embedded in
  the binary with `include_str!` like `hooks/observatory.ts`. No new file in the
  installed plugin directory.
- **Shell:** `install.sh` runs `--install-claude-mod` and warns instead of
  failing when it is refused. `uninstall.sh` is unchanged apart from any
  message, because `--uninstall-hooks` covers the mod.
- **Tests:** Rust unit and process fixtures with synthetic Herdr sockets and
  temporary homes; a new `tests/test_claude_mod.mjs` (node `--test`) that loads
  `register.js` with stubbed `on` and `$`; State.js and measurement harness
  additions. No test writes under the real `~/.claude`.
- **Docs:** AGENTS.md (reporter and installer lines), README.md, the plugin
  README and `harness-telemetry`.
- **User configuration:** the only real write outside the plugin is the mod
  directory, made at the live installed check after the user is asked once.
  `~/.claude/settings.json` is never read or written.
- **Out of scope:** rate limits, allowances and identity (change 4); remote
  Claude windows; the statusLine wrapper.

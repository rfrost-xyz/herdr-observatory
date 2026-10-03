# Tasks

Change 3 of 4 in the Claude Code parity programme. Work in this worktree on
`feat/claude-status-reporter`. The coordinator owns `tasks.md`, `evidence.md`,
the delta spec and AGENTS.md. Stage explicit paths only. Use atomic,
signed Conventional Commits with no attribution trailers. Use synthetic fixtures
only: no private transcripts, ids, paths or account data. No test or
implementation step writes under the real `~/.claude`; tests use temporary homes
with `CLAUDE_CONFIG_DIR` removed. Run every `openspec` command with
`OPENSPEC_TELEMETRY=0`.

## 1. Baseline (coordinator, before code)

- [ ] 1.1 Add `claude_agents_with_window` and `claude_agents_with_context_percent` (local and peer) to the Claude probe of `tests/measure_anton_popover.mjs`, with synthetic bound and mismatched v2 window reports (design D7), in its own `test(bench)` commit, without changing existing definitions.
- [ ] 1.2 Build the unchanged `80f6295` release binary and record in evidence.md its sha256, `--repeat 3` CPU, RSS and snapshot size, the new Claude metrics and the suite results.

## 2. Runtime (lane A)

- [ ] 2.1 `main.rs`: dispatch `--report claude` with exactly six arguments before the stdin read; keep the Pi form unchanged. `reporter.rs`: `report_claude` with argv validation, owner, mod receipt, single local host, `hook.lock`, `pane.get` binding, `obs_seq` and no-change checks, one metadata write and the exit statuses in design D3.
- [ ] 2.2 `native.rs` `publish_claude`: context-over-window rule and `context_percent` without reserve (D4). `main.rs` `State::sample`: drop `window` and `context_percent` from peer Claude agents.
- [ ] 2.3 Rust unit and process fixtures for D3 and D4 (design D7), each failing on `80f6295`: argument rejection without socket access, open stdin, wrong binding, success wire, no-change repeat, percentage, mismatched bind, context over window, unknown replay, peer drop. Codex and Pi outputs unchanged.

## 3. Mod and installer (lane B)

- [ ] 3.1 Add `hooks/claude/anton-observatory/` (`plugin.json`, `hooks.json`, `register.js`) per design D1 and D2.
- [ ] 3.2 `tests/test_claude_mod.mjs` (`node --test`) with stubbed `on` and `$`, covering the D7 list, including the static source scan and the never-throws, always-`next(e)` cases.
- [ ] 3.3 `hooks_install.rs`: `--install-claude-mod`, `--uninstall-claude-mod`, the `claude_mod` receipt entry, `claude_managed` (chezmoi, mise dotfiles, `.git` ancestors), peer-root and `CLAUDE_CONFIG_DIR` refusals, and the mod in `--uninstall-hooks` with a single preflight (D5). Embed the payload with `include_str!`.
- [ ] 3.4 Installer fixtures in temporary homes (D7): ownership, refusals, modified and extra files, idempotent reinstall, runtime-path change, combined uninstall, receipts without `claude_mod`, Pi extension bytes unchanged. Each fails on `80f6295`.
- [ ] 3.5 `install.sh`: run `--install-claude-mod` after `--install-hooks`, warning on refusal. Extend `tests/test_native_distribution.mjs` for the embedded payload and the unchanged installed file list.

## 4. Contracts and docs (coordinator)

- [ ] 4.1 AGENTS.md: the reporter, Claude context and installer lines (design D8). README.md and the plugin README: the mod, its location, removal of the mod alone, the new-session requirement and the statusLine fallback note.
- [ ] 4.2 State.js test and shell-harness fake: a Claude thread with a window and percentage, and one with a window only.

## 5. Verify, review, publish

- [ ] 5.1 Locked offline fmt, clippy (`-D warnings`) and tests; `node --test` (State, Pi hooks, Claude mod, distribution); `run-qml.sh`, `run-qmllint.sh`, `run-shell-harness.sh`; `omarchy-plugin-validate`; `OPENSPEC_TELEMETRY=0 openspec validate --all --strict`. Record after measurements with `--repeat 3` beside the baseline.
- [ ] 5.2 Independent adversarial review of the frozen source until clean, with lenses for mod safety (never throws, never blocks, argv only), reporter binding and privacy, installer ownership and managed refusal, and collector unknown-versus-zero. Fix findings, re-verify and record each round in evidence.md.
- [ ] 5.3 Live installed check. Ask the user once before the only real write outside the plugin. Update the installed plugin with private state, `.config.json`, `.accounts.json`, `.peers.json` and the owner marker inode preserved, then run `--install-claude-mod`. Run `claude plugin validate --strict --json` on the installed mod. The user starts a new Claude Code session in a Herdr pane and sends one prompt. Confirm `anton-observatory` appears in `/plugin`'s mod line, the reported id equals Herdr's session value (open question 1), and the popover shows a context window and percentage for that thread. Confirm Codex and Pi threads are unchanged, source hashes match and the shell harness passes against the installed modules. If the mod does not load or report, record the blocker with evidence (debug log lines, `/plugin` errors) and do not ship.
- [ ] 5.4 Sync specs, archive, publish the PR and merge with a merge commit after review.

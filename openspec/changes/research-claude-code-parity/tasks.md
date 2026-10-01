# Tasks

Change 1 of 4 in the Claude Code parity programme. Planning only: no source,
test, installed plugin or peer changes.

## 1. Research

- [x] 1.1 Survey the local Claude Code transcript corpus for structure and counts only: layout, identity, usage and split responses, children, turns, compaction, sizes. No content, ids or paths are recorded.
- [x] 1.2 Establish statusLine input, hooks, `CLAUDE_CONFIG_DIR` and plugin hook support from documentation and the installed binary, without running Claude Code.
- [x] 1.3 Establish how Herdr 0.9.1 identifies Claude panes and reports their session id.
- [x] 1.4 Map the Codex and Pi telemetry pipeline, peer probe and allowance contract in this repository.
- [x] 1.5 Read the Omarchy Claude usage collector for format knowledge only.
- [x] 1.6 Classify `~/.claude.json` by key names and types only, and check that its usage cache binds to the account profile.
- [x] 1.7 Run an independent adversarial critique of the synthesis and fold every correction into design.md.
- [x] 1.8 Review how harness hooks are created today (Herdr integration, user scripts, Anton's installer) and which hook payloads carry the context window or rate limits.
- [x] 1.9 Record the user's decisions on `~/.claude.json` and on the Claude reporter.

## 2. Decisions

- [x] 2.1 Record the metric mapping and decisions D1 to D11 in design.md, closing each open question with a decision.
- [x] 2.2 Name the follow-up changes and their order in proposal.md.
- [x] 2.3 Record the change 2 fixture inventory and the counts-only corpus check.

## 3. Baseline

- [x] 3.1 Build the unchanged `2d2be90` release binary and run `tests/measure_anton_popover.mjs --repeat 3 --json`.
- [x] 3.2 Run the Rust (three times), JS, QML, qmllint and Omarchy shell harness suites on the unchanged tree.
- [x] 3.3 Confirm the installed plugin matches `2d2be90`.

## 4. Review and delivery

- [ ] 4.1 Strict OpenSpec validation and a privacy scan of the staged artefacts.
- [ ] 4.2 Independent adversarial review of the artefacts until clean.
- [ ] 4.3 Archive without spec sync, publish, and merge with a merge commit.

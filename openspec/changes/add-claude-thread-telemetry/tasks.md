# Tasks

Change 2 of 4 in the Claude Code parity programme. Lanes run in order, A then B, each in this worktree. The coordinator owns `tasks.md`, `evidence.md`, the delta spec and AGENTS.md. Stage explicit paths only. Use atomic Conventional Commits with no attribution trailers. Use synthetic fixtures only: no private transcripts, ids, paths or account data.

## 1. Baseline (coordinator)

- [x] 1.1 Fix the intermittent Rust fixture directory collision (`0b9669e`). Five consecutive full test runs pass.
- [ ] 1.2 Add the additive Claude metrics to `tests/measure_anton_popover.mjs` in their own `test(bench)` commit, without changing existing definitions. Record the baseline from the unchanged `3ed6c11` binary in evidence.md: Claude agents with native telemetry = 0.

## 2. Native Claude replay (lane A)

- [ ] 2.1 `turns.rs`: add narrow helpers to start, finish and abort intervals and mark unknown, keeping `Turns` serialisation and `validate` unchanged. Test that existing Codex turn tests still pass and that the helpers preserve the invariants.
- [ ] 2.2 `claude.rs`: implement discovery (D1), including positive rediscovery, truncated-scan unknown and the bounded predecessor scan, and identity (D2), including the header, `sessionId`, `forkedFrom`, records without `sessionId`, and the excluded paths.
- [ ] 2.3 `claude.rs`: implement deduplicated usage replay, oversized classification and `last_valid` (D3), context (D4), compactions (D5), children (D6), turns (D7) and the `ClaudeCursor` block (D8), with byte cursors and the `TAIL` and deadline bounds.
- [ ] 2.4 `native.rs`: dispatch Claude panes, add the namespaced hashed key and the `claude` block on `Cursor`, accept `unknown` in `Cursor::validate`, require the block on Claude rows, and add local retention, the all-null caught-up sample and `seq`/`coverage_seq` publication. `telemetry.rs`: add `claude-transcript` to the allowlist. Codex output, cursors and checkpoints stay byte-identical for Codex-only inputs.
- [ ] 2.5 Unit fixtures for the full inventory in design.md, each failing on the old behaviour. Tests that need enrichment fail on `3ed6c11` because no Claude telemetry is produced.

## 3. Peers, harness and presentation (lane B)

- [ ] 3.1 `main.rs` `State::sample`: re-run `telemetry_view` and `turn_timing_view` on peer agents, and add local retention of peer Claude samples (D3), with drop triggers and the no-cursor case.
- [ ] 3.2 `tests/native_process.rs`: a peer probe returning Claude telemetry, an old-cursor round trip (block dropped, fresh replay), and the retention cases.
- [ ] 3.3 `tests/bench_anton_native.mjs` and the measurement harness: a synthetic Claude home and agents. A State.js test shows Claude telemetry projected with no context percentage. The shell-harness fake runtime adds a Claude thread with telemetry.
- [ ] 3.4 `examples/claude_corpus.rs`: a counts-only corpus replay. Run it locally, and record only aggregates in evidence.md.

## 4. Contracts and docs (coordinator)

- [ ] 4.1 AGENTS.md: make lines 18, 46 and 57 (Codex-only enrichment, records and child evidence) and the turn-timing wording harness-neutral for Claude Code. Update the plugin README.

## 5. Verify, review, publish

- [ ] 5.1 Locked offline fmt, clippy (`-D warnings`) and tests, plus the JS, QML, qmllint and Omarchy shell harness suites. Record the after measurements with `--repeat 3`.
- [ ] 5.2 Independent adversarial review until clean.
- [ ] 5.3 Live installed check: install the plugin, verify a real Claude thread shows telemetry in the popover (the user clicks), private state preserved, source hashes, owner restart, and the shell harness against the installed modules.
- [ ] 5.4 Sync specs, archive, publish, merge with a merge commit.

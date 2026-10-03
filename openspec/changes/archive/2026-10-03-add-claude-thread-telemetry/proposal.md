# Proposal

## Why

Claude Code threads in the Anton popover show only Herdr status and title, because
native enrichment is gated on `agent == "codex"` (`native.rs:680`). The archived
research change `research-claude-code-parity` established that Claude Code's
local transcripts carry everything Codex replay provides except the context
window. It also fixed the binding, deduplication, child, turn, checkpoint and
peer rules that keep Claude data under the same AGENTS.md contracts as Codex.

This is change 2 of 4 in the Claude Code parity programme.

## What Changes

- Bind a Claude pane to exactly one transcript, `<projects-root>/<entry>/<id>.jsonl`, using Herdr's `agent_session` id, and verify its identity by content.
- Fail closed on ambiguous, truncated, forked or predecessor bindings.
- Replay Claude transcripts natively with checkpointed byte cursors to produce:
  - deduplicated cumulative and last-response token counters;
  - context occupancy;
  - model;
  - compactions;
  - Agent child completion with outcomes;
  - wall-clock turn timing.
- Persist Claude parser state in a required, versioned `claude` block inside existing checkpoint rows. Keys are hashed and namespaced.
- Retain the last valid sample locally for an incomplete replay, on local and peer hosts. Re-validate peer telemetry and turn timing at the peer boundary.
- Peers return Claude telemetry over the existing probe and v1 envelope with no protocol change. Old and new collectors and peers interoperate.
- Add `claude-transcript` to the `usage_source` allowlist. The context window and percentage stay omitted until change 3.
- Add an optional `claude_projects_root` plugin config value. The default is `$CLAUDE_CONFIG_DIR/projects` or `~/.claude/projects`.
- Make the harness-telemetry contracts and AGENTS.md wording harness-neutral, keeping Codex behaviour unchanged.
- Fix the intermittent Rust test fixture directory collision.
- Add additive Claude metrics to the measurement harness, and a counts-only corpus check that prints aggregates only.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `harness-telemetry`: native transcript replay, cumulative metrics, outcome summaries, turn timing and checkpoints extend from Codex to Claude Code. This includes Claude-specific binding, deduplication and fail-closed rules.

## Impact

- **Runtime:**
  - new `omarchy/anton-runtime/src/claude.rs`;
  - `native.rs` (dispatch, cursor block, retention, rediscovery);
  - `telemetry.rs` (allowlist);
  - `main.rs` (peer retention and revalidation);
  - `config.rs` (projects root).
- **Tests:**
  - Rust unit and process fixtures, all synthetic;
  - `tests/measure_anton_popover.mjs` and `tests/bench_anton_native.mjs` (additive);
  - State and shell-harness fixtures for a Claude thread.
- **Docs:** AGENTS.md data-contract wording and the plugin README.
- **Install and peers:** the installed plugin runtime is updated. SSH peers keep working without redeployment, so redeployment is optional. Remote Claude values need the new peer.

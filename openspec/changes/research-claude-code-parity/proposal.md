# Proposal

## Why

Codex threads in the Anton popover show native session telemetry: context, input,
output, cache and uncached tokens, compactions, subagent completion with
outcomes and wall-clock turn timing. Codex also has allowances and verified
identity, and Pi reports metrics through `hooks/observatory.ts`. Claude Code
threads show only Herdr status and title, because native enrichment is gated on
`agent == "codex"` (`native.rs:680`).

Before building Claude support, we need evidence of what Claude Code exposes
locally and which sources satisfy the AGENTS.md data contracts. Without that
evidence, a later change would guess at transcript semantics, risk counting split
responses two or three times, or reach for an authentication file.

## What Changes

This is change 1 of 3 in the Claude Code parity programme. It is planning only
and changes no runtime, test or presentation code.

- Record the Claude Code local data surfaces: transcript format, subagent
  records, compaction records, turn timing records, statusLine input, hooks,
  `CLAUDE_CONFIG_DIR`, `~/.claude.json` and Herdr session identification. The
  evidence is structure, counts and documentation only.
- Map every Codex metric that Anton shows to a Claude source, or mark it
  unavailable with the reason.
- Decide session binding, deduplication, turn, completion, compaction and
  checkpoint rules for change 2, so that implementation does not improvise them.
- Assess allowance and identity sources against "never parse authentication
  files", and define the evidence gate that starts change 3.
- Record the unchanged baseline: test suites, the measurement harness and the
  release binary hash.

The programme sequence, each change merged before the next starts:

1. `research-claude-code-parity` (this change): research and decisions.
2. `add-claude-thread-telemetry`: native Claude transcript replay for local and
   peer threads, under the same contracts as Codex.
3. `add-claude-allowances-identity`: Claude allowances and identity from a
   compliant provider-owned source, or a recorded source blocker.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change records research and design decisions only.
`skip_specs: true` avoids claiming behaviour that is not implemented. Changes 2
and 3 carry the `harness-telemetry` and `account-allowances` deltas.

## Impact

Planning artefacts under `openspec/changes/research-claude-code-parity` only. No
source, test, installed plugin or peer changes. The visible consequence for later
changes: the Claude context dial stays unknown, because no compliant source for
the context window size was found (design D4).

# Verification

## Traceability

- Compact measured panels: browser host histories and CSS frames/gauges; UI tests cover distinct samples, finite-reading/gap counts, expiry/recovery and no invented peaks.
- Glanceable presentation and interaction: four persistent totals, removed host/theme labels, no inspector, card hover/focus details, bounded click pulse, no duplicate native state, hidden `.bare`; UI tests cover missingness, privacy clearing, local activation and reduced motion.
- Cumulative metrics: Codex numeric reader, Pi extension, reporter and probe; tests cover cumulative/last-response separation, cache accounting, complete-history compactions, malformed/cross-field values, source expiry and matching native context label.

## Sources and scope

Codex formula and statusbar fields were checked against official `rust-v0.155.1` sources:
- https://github.com/openai/codex/blob/rust-v0.155.1/codex-rs/tui/src/token_usage.rs
- https://github.com/openai/codex/blob/rust-v0.155.1/codex-rs/tui/src/chatwidget/status_controls.rs
- https://github.com/openai/codex/blob/rust-v0.155.1/codex-rs/tui/src/chatwidget/status_surfaces.rs

Context uses the last active usage and the harness's baseline-adjusted percentage. The statusbar's UsedTokens item is cumulative uncached input plus output, not active context. Session input/output come from native cumulative fields. Cache counters are tokens, not request counts. Pi APIs/schema were checked against installed documentation. Compactions remain unknown when full bounded coverage cannot be established.

## Gates and browser acceptance

89 Python and 170 JavaScript tests pass. Syntax, strict OpenSpec and whitespace checks pass. Independent reviews resolved context/cache consistency, Pi missing auxiliary usage, hover density/freshness, accessible percentage and stale history handling. Final independent review clean.

Synthetic three-host/eight-thread browser checks pass at 1280x720 and 1920x1080. Document dimensions equal viewport dimensions. Every card, fleet panel and hover detail has matching client/scroll heights. Keyboard activation opens no dialogue and preserves card-local details; the old inspector is absent. No live private content is included in these fixtures.

## Deployment

The first deployment exposed Herdr's native 16-key report limit: the expanded flat report was rejected. A direct current-session reporter check returned `invalid_metadata_token`. The corrective atomic v2 format sends 13 keys with four immutable numeric groups, each at most 67 characters. Tests now enforce native key/value limits, complete populated roundtrip, retained-v1 migration and malformed-group rejection. Corrected release `2a787b1`, identical image `sha256:ed937b58747c44fd1aa85cf53cf2e54db7ef57f92698ebd8978e8954adabd612`, is healthy on both hosts. All 89 Python tests also pass in that image. A direct current-session native report succeeds, and live Personal snapshots expose cumulative input/output, cached/uncached tokens and harness context percentage on two active Codex threads. Complete compaction counts remain unknown for these large histories. Work threads are idle with no fresh usage; no activity was manufactured to test them. Pi is verified with adapter fixtures and needs reload in existing sessions.

App/stylesheet/HTML and installed hook payload hashes match across both hosts. Work filtering contains only Work agents; both source hosts, palette and music feeds are available. Only current `2a787b1` and known-good rollback `d3cf8ec` images remain. Rollback config was preserved across the corrective deployment. Synthetic server/tab removed after geometry checks. Windows desktop visual QA is not claimed.

Native contract: https://herdr.dev/docs/socket-api/#agent-state-reporting

The actual local dashboard was refreshed and verified: four ordered state boxes, no host/theme labels, no inspector, eight hover containers, two live session-total panels and no document overflow. The existing PR is published ready and remains unmerged.

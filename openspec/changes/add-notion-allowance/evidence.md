# Notion allowance evidence

## Source scope

The source adds an optional browser-owned monthly observation, disabled without
private user/workspace/extension mapping. The native receiver has no HTTP or
credential-reading path. The adapter uses unsupported Notion web endpoints;
synthetic fixtures do not establish live compatibility.

| Requirement/scenario | Implementation | Verification |
| --- | --- | --- |
| Explicit identity, monthly scope, no credential forwarding | notion-extension/source.mjs; src/notion.rs | Synthetic identity, scope, field allowlist and wrong-origin tests |
| Zero, overage, unavailable and reset | src/notion.rs; State.js; AllowanceCard.qml | Rust normalisation/cache tests; State tests; production Qt monthly card test |
| Bounded frame, stale source, retirement | src/notion.rs | Native process oversize, EOF, held-open deadline, replay, retired owner; cache expiry/rebinding |
| Receipt-owned registration/removal | src/notion.rs; uninstall.sh | Temporary-directory repeat registration, conflict preservation, removal and symlink tests |
| Codex and native distribution preserved | main.rs; model.rs | Existing Rust, Pi, State, distribution and QML suites |

## Checks

- Locked/offline Rust build and Clippy with warnings denied passed.
- Production QML fixtures: 26 passed. The test exercises the actual State.js
  projection and caught a Qt date-format difference, corrected before review.
- Plugin manifest validation passed. No shipped Python, Docker or web runtime
  added; existing distribution audit covers the native source tree.
- OpenSpec strict validation passed.
- Full Rust and JS gate results and independent review are pending final run.

## Live acceptance outstanding

No extension installed or browser permission changed. No live Notion allowance
claimed. Chromium Profile 1 extension activation, private native registration,
comparison with Notion Usage, restart/expiry and installed fleet regression
checks remain required. No live CPU/RSS result is claimed. Canonical spec sync
and archive remain pending these checks.

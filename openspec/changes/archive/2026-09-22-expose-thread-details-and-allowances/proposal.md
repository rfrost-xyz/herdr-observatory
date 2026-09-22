# Proposal

## Why

Useful thread information is currently hidden behind hover, and the fleet panels underuse their space. The display also lacks a view of the operator’s two Codex account allowances and available usage reset passes.

## What Changes

- Show technical detail persistently in compact thread instruments; reserve hover for emphasis and click for a short local glitch.
- Make fleet instruments taller and quieter, and make both music lines clickable for their existing text effects.
- Split the footer into observations and Personal/Work Codex allowance panels, including weekly remaining, scheduled reset, plan, reset passes and earliest expiry.
- Bind allowances to the actual account rather than the machine, using supported read-only Codex data and explicit private account labels.

## Capabilities

### New Capabilities

- `account-allowances`: Bounded, read-only collection, explicit publication and truthful presentation of Codex account allowances.

### Modified Capabilities

- `activity-dashboard`: Persistent thread detail, subtle fleet instruments and manual music effects.
- `office-display`: Split footer and click-only decorative card glitches.

## Impact

Browser cards, music effects, footer, existing container collection and optional harness adapters, private configuration, tests and operational documentation. No additional installed host service or credential mount. Continue the existing dashboard branch and PR.

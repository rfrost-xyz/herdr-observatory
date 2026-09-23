# Proposal

## Why

The display shows a session-wide cache total but cannot show whether recent Codex turns are reusing context. Account token activity is absent, and the fixed browser layout is awkward in smaller Omarchy tiles. The connection icon is an approximation of the requested loading.dev component.

## What Changes

- Show a bounded recent cache-read share from distinct cumulative Codex usage samples, with model and compaction markers and clear unknown/reset handling.
- Read ChatGPT account token-activity summaries and daily buckets through the existing short-lived authenticated app-server probe, keeping them separate from local cache counters and within explicit Personal/Work account sharing.
- Preserve opaque turn and child association inside the local Codex adapter for future attribution, without publishing child identifiers or claiming a complete subagent total. Existing CLI sessions remain supported. The active-thread app-server usage notification remains a future source until Observatory owns the thread connection.
- Move the browser presentation to React 19, use the actual loading.dev Blocks component for connection loading, and make the display usable in narrower tiled browser windows while retaining the 720p and 1080p display layouts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `harness-telemetry`: Recent, deduplicated Codex cache samples and bounded opaque attribution.
- `account-allowances`: Read-only account token activity alongside allowance instruments.
- `office-display`: React 19 presentation and responsive tiled-window behaviour with the loading.dev connection indicator.

## Impact

Codex hook enrichment, account app-server probe, typed metadata, private account cache, browser presentation and build assets change. React 19 and loading-dev become pinned build dependencies; the deployed image serves local static output without a package manager or external CDN. No API-key billing integration, login change, host observer daemon or public receiver is introduced.

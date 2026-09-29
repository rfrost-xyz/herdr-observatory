# Design

## Context

Codex owns its authentication and provides bounded account RPCs. Notion’s CLI owns a workspace-scoped public API token; a direct read of the web allowance endpoint returned HTTP 401. Its supported catalogue has no personal monthly allowance endpoint.

## Goals / Non-Goals

Goals: monthly used percentage and exact reset date, explicit user/workspace binding, existing freshness semantics, browser-owned credentials. Non-goals: credit balances, purchases, billing changes, general browser automation, remote credential sharing, invented pacing.

## Decisions

An optional Manifest V3 extension uses the browser’s normal authenticated fetch to the fixed app.notion.com getSpaces and getCreditRateLimitStatus endpoints. It verifies the configured user/workspace through getSpaces, sets the explicit active-user header and forwards only normalised numeric values and the configured identity. A five-minute alarm refreshes while the browser runs. No cookies permission, content script, page scraping, externally connectable surface or exported session credential.

A one-message Rust native receiver validates the exact extension origin against private configuration, enforces a small frame and deadline, hashes the user/workspace binding and atomically saves a bounded observation under the existing owned state directory. The collector reads it independently of Codex refresh. Monthly values use dedicated optional fields; no weekly or calendar-start assumption is introduced. UI shows used percentage and reset date; pace remains unknown unless source period start is available (not implemented).

## Risks / Trade-offs

The web endpoint is unsupported and may change. Fail closed and keep the adapter isolated. Cross-origin extension fetch must be qualified in the user’s actual Chromium Profile 1 before claiming live support. Browser closure leads to stale/unavailable data after ten minutes. Native messaging registration is explicitly pinned to the installed extension ID; activation requires user approval.

## Migration Plan

Ship disabled by default. Review source and run synthetic gates, then register the bridge for the user-approved extension ID and configure the account privately. Verify live reading, logout/unavailable behaviour and restart. Keep the change active until live acceptance; do not archive based solely on fixtures. Remove only exact receipt-owned bridge files during rollback.

# Proposal

## Why

Anton lacks the user’s monthly Notion AI allowance and reset date. The official CLI login succeeds but its token is rejected by the browser allowance endpoint, so authentication must remain with the browser.

## What Changes

- Add an optional Notion monthly allowance card with used percentage and reset date.
- Add a narrowly scoped Chromium extension and bounded native messaging receiver. Browser authentication remains browser-owned; no cookie extraction or credentials in Anton.
- Require explicit user/workspace binding and preserve unavailable, stale and zero states.
- Document installation, permission approval, unsupported endpoint maintenance and removal.

## Capabilities

### New Capabilities

### Modified Capabilities

- `account-allowances`: Optional browser-owned Notion monthly observations alongside existing Codex allowances.

## Impact

Native Rust runtime, allowance presentation, synthetic tests and a new optional Chromium extension. No new Rust dependencies, service, listener, credit purchases or CLI credential access. Browser extension activation requires the user to approve the concrete extension permissions.

# Why

The user wants Notion monthly AI allowance and reset visibility alongside the
existing providers, with authentication handled outside the popover through the
same provider-owned account-source principles.

# What Changes

Add Notion only when an authenticated account source can supply its actual
allowance and reset date. No setup controls in the popover, browser extension,
cookie extraction or separately invented authentication system.

Implementation is blocked on a suitable source. The previous browser-extension
experiment was rejected and removed. The official mise-managed CLI and its login
remain in place.

# Capabilities

## New Capabilities

None.

## Modified Capabilities

- `account-allowances`: Notion monthly allowance through a provider-owned source.

# Impact

Future native account collection and passive monthly presentation. No accepted
runtime changes currently remain on this branch.

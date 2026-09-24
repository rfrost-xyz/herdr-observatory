# Proposal

## Why

The existing Observatory is a passive display. On an Omarchy desktop, a bar glance and a tiled companion window would make current Herdr work, Codex allowance, local inference and fleet health visible while Herdr remains the place to operate threads.

## What Changes

- Define the product and technical concept for an Omarchy bar plugin and a native companion window.
- Add a responsive, synthetic-data UI prototype covering the popover and three companion window sizes.
- Implement a user-owned Omarchy bar widget and a first companion window using the installed plugin contract and Observatory's read-only loopback state.
- Record source boundaries, unknown/stale behaviour and a phased implementation route.

The standalone HTML prototype remains illustrative. The QML surfaces read live state if Observatory is running on loopback. This change adds a reversible installer for the local Omarchy host and installs a copy for an operator trial. It does not alter Observatory's server/API or remote fleet configuration.

## Capabilities

### New Capabilities

- `omarchy-companion`: Read-only Omarchy bar, popover and tiled companion presentation of permitted Observatory state.

### Modified Capabilities

None. No existing display behaviour changes.

## Impact

Adds a user-owned plugin source directory with an Omarchy manifest, QML surfaces and explicit install/uninstall commands. It depends on the installed Omarchy shell/Qt Quick and the existing loopback Observatory service. The local bar layout gains one user-owned widget while installed; uninstall removes that widget and its owned files.

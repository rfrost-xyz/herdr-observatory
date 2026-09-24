# Herdr Observatory for Omarchy

This directory is the **plugin root**. Its `manifest.json` declares one
`bar-widget`, with `Panel.qml` as the entry point. `Panel.qml` uses Omarchy's
`Panel`, `WidgetButton` and `KeyboardPanel`, so the popover follows the bar's
current edge and monitor and shares its focus/dismissal behaviour.

`Companion.qml` is a separate Qt 6 Quick window. The bar launches it with the
installed `/usr/lib/qt6/bin/qml` runner. It is an ordinary Wayland toplevel:
Hyprland chooses its tile, workspace and monitor. Both surfaces use
`SnapshotStore.qml` and `State.js` to read and project only
`http://127.0.0.1:8789/api/state`. There is no Herdr control path.

## Source layout

| File | Role |
| --- | --- |
| `manifest.json` | Omarchy discovery and bar metadata |
| `Panel.qml` | Bar indicator and anchored popover |
| `Companion.qml` | Resizable Qt Quick toplevel candidate |
| `SnapshotStore.qml` | Bounded, timed loopback reads; clears state on loss |
| `State.js` | Shared, source-bound display projection |

Omarchy's `plugin add` clones a repository with a manifest **at its root**.
This project keeps the plugin in a subdirectory so it cannot be passed to
`omarchy plugin add` as a repository URL. A future distribution step can
publish this directory as its own repository. The installed shell also supports
a user-owned plugin directory at `~/.config/omarchy/plugins/herdr.observatory/`,
followed by `omarchy-shell shell rescanPlugins` and
`omarchy plugin enable herdr.observatory`. Installation and enabling are not
part of this change; check managed configuration first.

## Current data coverage

- Herdr thread state and fleet **reporting**, scoped to the existing permitted
  Observatory profile. A missing host makes counts partial.
- Only explicitly mapped Codex weekly allowance. Zero and unavailable remain
  different.
- The sampled ws-255 GPU utilisation, labelled as a device metric. Request
  counts, model residency and token throughput remain unavailable until a
  separate measured inference source is added.

The built-in Omarchy Agents panel is separate and remains available. This
plugin does not scrape its data or replace its provider usage collection.

## Verification

Run `node tests/test_omarchy_state.cjs` from the repository root for the
projection contract. `qmlformat` checks the QML syntax. A Qt 6 offscreen
launch of `Companion.qml` checks that the toplevel loads, but visual tiling,
shell integration and keyboard behaviour require an Omarchy graphical session.
The plugin adds no animation of its own; the installed shell controls the
popover transition.

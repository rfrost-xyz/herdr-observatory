# Proposal

## Why

Anton is now used as an Omarchy plugin. Keeping a Python runtime and a separately supported web dashboard adds processes, deployment paths and maintenance unrelated to that application.

## What Changes

- Make the installed plugin and its on-demand Rust peer the only supported application/runtime.
- Port remaining collection, validated session replay, allowances, identity, navigation and required harness reporting into Rust while preserving the QML popover.
- **BREAKING:** Remove the web application, HTTP server, Docker deployment, Work/music forwarding, cliamp bridge and dashboard-only host metrics.
- Replace Python-over-SSH probes and Docker allowance exporters with a narrowly owned peer executable over existing SSH, without a listener or daemon.
- Preserve private account mappings, concealment, exact session navigation, source timestamps, unknown/stale/zero distinctions, bounded checkpoints and guarded uninstall. Remove obsolete owned hook callbacks only after native collection parity is verified.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `omarchy-companion`: Rust-only plugin/peer ownership, native account refresh, reversible migration and no web forwarding.
- `harness-telemetry`: native collection and required Rust reporting replace image-owned adapters; retire web observation views and sharing obligations.
- `account-allowances`: native local/SSH reads replace web export and Docker sources; retain truthful bounded account state.
- `activity-dashboard`: retire the browser dashboard capability.
- `office-display`: retire the office/web/forwarding capability.

## Impact

The Rust crate, plugin packaging/QML process commands, harness integration and tests change. Legacy Python/web/build/deploy sources are removed only after synthetic parity evidence is captured. Existing remote web containers are left untouched; installing this plugin stops its forwarding. A separately marked `herdr.observatory-peer` executable is provisioned by the parent only after review. No Git/forge actions or remote service changes are included.

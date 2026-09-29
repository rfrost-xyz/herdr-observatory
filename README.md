# Anton

A native Omarchy menubar plugin for Herdr threads, session usage, subagent
completion and account allowances across your local fleet.

The popover follows the active Omarchy theme. Click a thread to open its exact
Herdr instance. Account emails can be concealed together, and sections/machines
can be collapsed without stopping collection.
With fleet discovery enabled, saved Herdr machines appear and disappear
automatically. New machines show “Setup needed” until their native peer is
explicitly provisioned.

## Architecture

QML owns one Rust collector. It reads local Herdr and invokes a small Rust peer
on configured SSH hosts. Both use bounded read-only operations. No Python,
Docker, HTTP server, web dashboard, music forwarding or background service is
required. Codex metrics come from validated native session records; Pi uses a
small extension which calls the native reporter.

See [installation, configuration and complete removal](omarchy/herdr.observatory/README.md).
Canonical behaviour is in `openspec/specs`; historical decisions are retained
in `openspec/changes/archive`.

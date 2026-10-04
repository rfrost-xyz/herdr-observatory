# Anton

A native Omarchy menubar plugin for Herdr threads, session usage, subagent
completion and Codex and Claude account allowances across your local fleet.

The popover follows the active Omarchy theme, which QML reads directly; the
collector carries no theme data. Click a thread, or focus it from the keyboard
and press Enter, to open its exact Herdr instance. Keyboard focus stays on the
same thread as others come and go, and clears when that thread is hidden. Press
`r`, middle-click the bar icon or send the IPC `refresh` command to ask the
collector for an immediate local sample; repeated requests are coalesced, and
SSH peers keep their configured cadence. Account emails can be concealed
together, and sections/machines can be collapsed without stopping collection.
With fleet discovery enabled, saved Herdr machines appear and disappear
automatically. New machines show “Setup needed” until their native peer is
explicitly provisioned.

## Architecture

QML owns one Rust collector. It reads local Herdr and invokes a small Rust peer
on configured SSH hosts. Both use bounded read-only operations. No Python,
Docker, HTTP server, web dashboard, music forwarding or background service is
required. Codex and Claude Code metrics come from validated native session
records; Pi uses a small extension which calls the native reporter. A small
local Claude Code mod reports the session's context window, so the Claude Code
context dial can show a percentage, and, from fresh evidence, the session's
5-hour and 7-day rate-limit windows. When attribution is allowed, the reporter
records those windows privately for the account `~/.claude.json` names, and a
mapped local Claude account then shows an allowance row beside the Codex ones.
Claude allowances are local-only and need an active Claude Code session in a
Herdr pane.

See [installation, configuration and complete removal](omarchy/herdr.observatory/README.md).
Canonical behaviour is in `openspec/specs`; historical decisions are retained
in `openspec/changes/archive`.

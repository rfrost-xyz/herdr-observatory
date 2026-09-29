# Design

## Context

The accepted runtime owns scheduling/state in Rust but retains a Python domain broker, Python reporters/navigation/account helpers and legacy web sources. The user now explicitly removes web forwarding and asks for a native plugin-only product. Existing dirty work and private installed state must be preserved until reviewed migration.

## Goals / Non-Goals

**Goals:** One plugin-owned Rust runtime with bounded local collection and an on-demand Rust peer; no Python/Docker runtime dependency; existing QML and source-truthful metrics; exact navigation and account identity; explicit ownership migration/uninstall.

**Non-Goals:** Web compatibility, a new service/listener, remote container removal, visual redesign, agent control, intrinsic/assembly optimisation, Git/forge actions.

## Decisions

1. Extend the existing crate into a reusable native library and CLI. Standard threads own current state and per-host scheduling; bounded Rust subprocess/socket operations replace the broker. Pinned SHA-256 and TOML crates replace Python hashing/theme parsing. No dynamic package installation.
2. Port validators and parsers against synthetic oracle fixtures before deleting legacy source. Preserve exact session/header/file identity, safe-integer/null distinctions, independent source times, typed completion/outcomes, complete-coverage compactions, turn intervals and private cursor bounds. Herdr native session IDs plus bounded session-root discovery eliminate redundant synchronous Observatory Codex callbacks. Pi retains its extension and native reporter.
3. One marked peer copy plus an owned retryable uninstall shell tail at `~/.local/share/herdr.observatory-peer/anton-runtime`, private configuration beside it and state under `~/.local/state/herdr.observatory-peer`. The generic `~/.local/share/anton` belongs to an unrelated app and is forbidden. Commands are on-demand `--probe`, `--allowances-probe`, `--identity-probe` and user-only `--focus`, with bounded JSON stdin. SSH remote command text is fixed; variable selectors are data. No listener, daemon or Python fallback. A missing/incompatible peer fails one host explicitly.
4. Local and peer account readers invoke only supported read-only Codex app-server RPCs under deadlines. Remote cache refresh is independent of threads. Account IDs are hashed; verified emails travel only on explicit identity reads to the private local identity file, never ordinary telemetry. Existing account mappings and concealment preferences remain.
5. Shared native file helpers enforce owner, no-follow, private modes, atomic writes and installed-owner retirement. Installer records exact owned payloads and integration commands; uninstall removes only still-owned callbacks/files and fails on conflicting/unknown files. Peer removal is separate and receipt-bound, with no deletion of external web services.
6. Remove Work/music collectors/publishers and dashboard-only metrics entirely. Explicit config migration drops known retired keys and legacy container-export selectors while preserving source targets, account mappings and unrelated legitimate plugin preferences. QML process commands switch to native subcommands; visual components stay unchanged.

## Risks / Trade-offs

- Large parser port can silently lose metrics. Capture synthetic fixtures across supported native record shapes, stale/malformed/zero cases and checkpoints; independently compare semantics before source removal.
- Peer provisioning adds an owned executable. Keep a versioned protocol, explicit receipt, atomic reviewed update and standalone removal contract.
- Rewriting integration paths can disturb harness configuration. Preflight symlinks/chezmoi/conflicting ownership, preserve unrelated entries and use private rollback before atomic edits.
- Removing old sources can erase useful tests. Port plugin guarantees into Rust/QML/JS first, retain historical OpenSpec archives as historical evidence, and delete only obsolete implementation/test assets.

## Migration Plan

Root first disables explicitly retired forwarding and records a plugin-only hybrid baseline. Implement native modules in bounded parallel ownership, establish fixture parity, then remove the obsolete application. Validate fmt/Clippy/Rust tests, JS/Pi and QML integration, native runtime dependency audit, install/retirement/uninstall, process EOF/deadline/restart and resource scope. Independent frozen-source review precedes parent-only peer provision and local installation. Root verifies both hosts, allowance refresh without active threads, exact navigation, no Python child/publication and warm restart. Synchronise changed/retired canonical specs and archive only after installed acceptance. Keep rollback private until acceptance, then clean owned temporary artefacts.

# Design

## Context
Herdr 0.9.1 on a Linux desktop and 0.9.0 on a WSL/container host both return JSON session snapshots. The Windows host SSH connection terminates inside its Linux container. Active Omarchy colours are in ~/.local/state/omarchy/current/theme/colors.toml. Additional hosts can be configured once SSH is available.

## Goals / Non-Goals
Goals: truthful passive visibility; configurable fleet hosts; fail-closed Work disclosure; live theme adoption.
Non-goals: agent control, terminal transcript collection, billing estimates, changing lock-screen policy, installing remote daemons or cloud publishing.

## Decisions
Use a dependency-free Python service and static HTML/CSS/JS. A local service can read sockets and launch SSH; hosted Workers cannot reach those private resources. Each host has an independent polling worker, using a Python probe sent over SSH stdin. No remote files are installed. Polling compatible snapshots is simpler than maintaining version-specific event subscriptions; history explicitly describes observed state transitions, not an exhaustive event log.

The probe returns only allowlisted agent metadata, a palette and /proc metrics. The parent assigns project visibility by explicit path roots and filters data before HTTP serialisation. Unclassified agents are Personal. Work mode cannot be upgraded through browser parameters. Bind only to loopback; reject unexpected Host/Origin headers. Configuration is trusted local operator input, never a browser-provided command.

Display CPU deltas, memory, filesystem usage, network rates and optional NVIDIA GPU metrics with scope labels. Container/WSL metrics are never labelled as native Windows totals. On collector failure clear current activity and mark offline; retain bounded, profile-filtered transition history. The browser independently expires stale data.

## Risks / Trade-offs
- Polling misses rapid transitions: label history as sampled observations.
- Metadata can include sensitive task titles: Work excludes unknown roots; raw output and full paths never reach the browser.
- A missing GPU tool is normal: display unavailable, never invent values.
- SSH access may fail: bounded timeouts and independent host workers preserve other hosts.

## Migration Plan
Run from checkout with a private JSON configuration. Stop the process to roll back. Windows uses a documented WSL launch command and its local browser; desktop startup is optional and not installed automatically.

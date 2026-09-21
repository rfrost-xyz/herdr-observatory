# Design

## Context
The existing PR remains unmerged; this change extends that same reviewable delivery. The workstation SSH route works from the laptop, but reverse SSH is refused. The workstation uses userspace Tailscale inside a host-networked container. Windows GUI access is not exposed through that shell.

## Goals / Non-Goals
Goals: a persistent Work service, a private Personal laptop service, theme continuity and a no-scroll 16:9 wall display.
Non-goals: public hosting, enabling inbound SSH on the laptop, terminal control, changing Windows lock/display policies.

## Decisions
The laptop publisher uses the existing authenticated SSH route to atomically write a bounded, versioned Work-only feed into a private workstation state directory. Data is allowlisted before transmission and again on receipt. Only selected source hosts are exported; personal agents and personal history never leave the laptop. The workstation combines its local probe with a file collector. Expired source timestamps clear laptop agents and metrics while retaining the last validated palette, including across restart.

The publisher runs with the Personal service but emits an independent Work projection. The office service always starts with --profile work and has no profile-switch API. Configuration stays outside Git. The workstation container reuses its installed runtime image with a Python entrypoint and a read-only home-volume mount; it has no Docker socket and no separate tailnet identity. Restart policy keeps it independent of the interactive terminal.

The display uses fixed viewport grid rows, bounded agent/history/machine counts and explicit page indicators. Automatic rotation covers additional agents; pause/previous/next allow Personal inspection. Global counters cover all permitted live agents, not just the displayed page. Reduced motion disables decorative animation. At 1280x720 and 1920x1080 no document or panel should scroll.

## Risks / Trade-offs
- Laptop disconnected: expire its feed within 30 seconds; retain theme and keep workstation collection alive.
- Duplicate file samples: preserve source timestamp and skip duplicate metrics/history ingestion.
- Windows browser cannot be launched via current container shell: provide a one-click PowerShell launcher and report the exact remaining desktop action if GUI access remains unavailable.
- Dense activity: show bounded pages, never shrink an unlimited list or silently hide overflow.

## Migration Plan
Deploy code as a versioned local release and configure private state. Start a loopback Work service on the workstation and restart the local Personal preview with publishing enabled. Verify live remote HTTP, filtering, theme delivery and service restart. Stop/remove only the named dashboard service to roll back; Herdr remains untouched.

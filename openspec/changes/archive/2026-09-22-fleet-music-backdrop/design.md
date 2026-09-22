# Design

## Context
See proposal.md. Both hosts run the same versioned image. The existing Work feed samples too slowly for music, and is deliberately limited to agent disclosure.

## Decisions
Read cliamp v2 IPC with two hardcoded read methods over a persistent Unix socket. Mount its directory through an optional Compose override so socket recreation works. This exposes integration authority, not a socket-level read permission. Allowlist title, artist, playback state, bounded finite spectrum bands and source timestamp only.

A separate persistent SSH process carries bounded NDJSON to the receiver module in the existing ws-255 image. An atomic ephemeral latest-sample file is read through an explicitly configured path. This avoids SSH connection creation per frame and does not broaden the Work agent feed. Publication is opt-in and the user explicitly selected sharing titles from iapetus. Music expires after three seconds independently of Herdr. The browser polls a small separate endpoint and never plays or captures audio.

At the user’s explicit request, adapt the actual Canvas2D field from pinned omacom/omarchy-site revision 2af2bcdc41c1eba20a2f4d6a98b9521f5d014dc8. Preserve its noise, ordered dithering, music columns and click stamps while replacing React/site-specific lifecycle and playback with the existing display and real cliamp samples. Retain source attribution without inventing an upstream licence. Use the synchronised palette, pointer glow and bounded click impulses. Real spectrum affects pixels; absence/paused/expired playback removes audio energy. The field remains behind cards; no text effects enter cards. Reduced motion and hidden views suppress animation.

## Risks / Trade-offs
- SSH outage: bounded nonblocking writes, process cleanup and reconnection; stale data expires.
- Socket/config contains private data: allowlisted IPC requests and response fields; no raw snapshot forwarding.
- Spotify desktop does not expose spectrum: document cliamp source support without claiming desktop Spotify capture.
- Background rendering cost: bounded grid, DPR and frame rate; opaque cards retain contrast.

## Migration Plan
Build one versioned image, load it on both hosts, add optional music configuration and the iapetus socket mount, then recreate existing Compose services. Verify real source samples and shared titles on both endpoints without recording private track data in the repository. Retain the previous image and configuration for rollback. Review, synchronise and archive this continuation of PR #1 without merging it.

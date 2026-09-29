# Design

## Context

See proposal.md. The existing QML component overlays pace and balance, and animates pace colour with explicit cancellation when hidden or reduced motion is enabled.

## Goals / Non-Goals

Retain the compact row height, provider/account identity, reset details, existing collector and privacy behaviour. This change changes only presentation.

## Decisions

- Use a pure State.js pace-band helper for explicit boundary coverage. Calculate percentage-point difference from existing valid readings, without a relative ratio.
- Use the active theme accent for the six-pixel balance fill and foreground for its number. Use muted-theme amber for the small deficit and full amber beyond five points; green and red use the respective semantic theme colours.
- Place a two-pixel interval beneath the main bar in a clipped six-pixel effect region, separated by a gap. Confine the halo and existing sparks to that region. Keep the neutral tick on the main track.
- Preserve 180ms pace-colour transitions and hidden/reduced-motion cancellation. At exact equality the zero-width strip has no fabricated interval.

## Risks / Trade-offs

Small intervals may be subpixel. Preserve honest geometry and the expected-position tick; exact values remain in the existing concise tooltip. Existing unknown-source guards continue to hide unavailable geometry.

## Migration Plan

Copy only changed plugin presentation files into the existing installation after review. Preserve private configuration. Keep temporary copies until installed acceptance, then remove task-owned files. No collector or remote changes.

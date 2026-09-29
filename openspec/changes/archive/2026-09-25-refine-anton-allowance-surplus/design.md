# Design

## Context

See proposal.md. Existing allowance geometry keeps a theme-accent fill and neutral expected-position tick. The signed pace and hatch build on this without widening the popover or changing collection.

## Decisions

- Prefer a hatch at main-track height for missing allowance. It communicates emptiness and remains legible at six pixels; a two-pixel hatch would be difficult to distinguish.
- Keep the lower thin strip only for positive surplus. Zero has no fabricated interval. Use static same-accent diagonal strokes for deficit, without animation or glow.
- Format remaining-minus-expected to one decimal with a leading plus for positive values and a typographic minus for negative values. Round symmetrically and normalise zero; use this same value for severity colour and positive-effect eligibility. Retain unrounded measured geometry.
- Use existing semantic green/amber/red thresholds for the small signed reading. Keep remaining percentage in foreground. Tooltip explains percentage points.
- Preserve hover-only effects, hidden/reduced-motion guards, global email concealment, reset details and compact row height.

## Risks / Trade-offs

Very small differences remain subpixel in the graph. The signed reading and exact comparison tooltip provide detail without widening the interval artificially.

## Migration Plan

Install only changed presentation files after focused verification and independent review. Capture the live popover, preserve configuration, synchronise the spec and remove task-owned temporary files. No collector or remote service changes.

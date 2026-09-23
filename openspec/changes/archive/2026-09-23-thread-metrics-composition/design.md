# Design

## Context

See proposal.md. Both the React renderer and the plain DOM fallback consume one shared browser view model. The supported eight-card desktop layout has narrow cards at 1280×720.

## Goals / Non-Goals

**Goals:** Separate four readable instruments in both renderers, preserve telemetry scope, and fit the supported displays.

**Non-Goals:** Change collector data, infer absent totals, or add another dial.

## Decisions

The shared view model emits context, input, output and cache readings as four instrument objects. Input and output use numeric cumulative totals when available; response figures carry visible response labels only when cumulative totals are absent. Real DOM text replaces CSS generated figures so both renderers, accessibility and browser sizing use the same values.

The grid uses four equal flexible columns. Context retains the sole dial. Cache percentage is calculated from the existing validated input composition.

## Risks / Trade-offs

Long values can crowd a 1280px card. Compact figures and responsive type limit their width; exact counts remain in accessible descriptions and titles. Verify the full eight-card viewports in Chromium.

## Migration Plan

Build the bundled renderer with the source change, run tests and viewport checks, deploy the exact merged revision to both profiles. Keep the previous image as rollback on each host.

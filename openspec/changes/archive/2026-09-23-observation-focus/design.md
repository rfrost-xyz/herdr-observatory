# Design

## Context

See proposal.md. The browser retains a 60-record in-memory observation buffer but currently renders only four rows. Both React and fallback DOM consume the same view model. The header already contains the title, song and state totals.

## Goals / Non-Goals

**Goals:** A readable single-line card footer, separate activity ribbon, scroll access to the bounded history and a focused observation view with the header retained.

**Non-Goals:** Persist observations beyond the current buffer or change event generation and filtering.

## Decisions

The shared view model supplies all bounded records to both renderers. The observation container owns vertical scrolling and follows new rows only when the user is already at its bottom. Existing animation eligibility remains limited to the latest four records.

Card metadata is rendered as one text element in host, Herdr pane, harness and model order. The full labelled detail is available in its title. Coverage and age details occupy a second footer row. Activity and tool move beneath the top bar as a full-width strip, leaving the native state icon alone.

The icon toggles an observation-focused layout under the existing header and requests browser fullscreen when available. An in-page expanded layout remains usable if the browser denies fullscreen. Escape or the same control restores the compact dashboard.

## Risks / Trade-offs

Older events can remain in view while new observations arrive. Auto-follow is conditional on the current scroll position. Narrow metadata can elide visually; its full text stays available as title and accessible card detail.

## Migration Plan

Build the React bundle, verify both render paths and supported viewports, deploy the exact merged revision to Work and Personal, and retain the prior image as rollback.

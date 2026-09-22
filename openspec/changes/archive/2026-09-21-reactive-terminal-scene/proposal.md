# Reactive terminal scene

## Why
Cards and panels still read as a dashboard. The user requires a fully rendered CLI-like scene reacting to meaningful Herdr changes.

## What Changes
Replace the dashboard DOM with one canvas terminal scene. Derive deduplicated impacts from sampled agent/source transitions, render a terminal transcript and live process listing, and propagate event-triggered waves through the scene.

## Capabilities
### Modified Capabilities
- office-display: reactive rendered terminal instead of cards.

## Impact
Browser assets, UI tests and README. Backend, privacy filters, Compose and source collection unchanged.

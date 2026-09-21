# Continuous heartbeats

## Why
One-second renders restart three-second machine sweeps. Active threads also need visible activity indicators.

## What Changes
Keep animation phase across refreshes and add compact working-thread heartbeats without changing viewport bounds.

## Capabilities
### Modified Capabilities
- office-display: continuous status heartbeats.

## Impact
Browser assets and UI regression tests only; retain Work disclosure and reduced-motion support.

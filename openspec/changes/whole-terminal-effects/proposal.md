# Whole-terminal effects

## Why
The user requests unbranded whole-terminal effects, complete nonrepeating playback, a configurable ten-second hold and theme-aware animation colours.

## What Changes
Animate a complete terminal capture with colour preserved. Run each animation through its natural final frame, then show the current live TUI for a ten-second default hold. Left/Right changes the hold in one-second steps; Page Up/Down pages threads. Exclude the preceding effect when selecting the next. Remove renderer branding from display labels.

## Capabilities
### Modified Capabilities
- office-display: whole-terminal complete playback and configurable hold.

## Impact
Library adapter, frame API, browser scheduler, tests, README and image. Existing profile filtering remains authoritative.

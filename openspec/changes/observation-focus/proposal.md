# Observation focus and thread detail

## Why

Thread metadata wraps into two small lines, and tool activity competes with the state icon. The recent-observation footer hides its bounded history and its Fullscreen control enlarges the whole dashboard instead of the observations.

## What Changes

- Put host, Herdr pane, harness and model on one metadata line in that order.
- Give activity and tool their own full-width strip beneath the project and state row.
- Make the bounded observation history scrollable with a mouse wheel and keyboard.
- Replace the Fullscreen word with an accessible icon that expands observations below the persistent title, music and state totals.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `activity-dashboard`: change thread card hierarchy and metadata order.
- `office-display`: expose bounded observation history and an expanded observation view.

## Impact

Browser renderers, shared UI logic, stylesheet, compiled bundle and UI tests. No feed schema or collector changes.

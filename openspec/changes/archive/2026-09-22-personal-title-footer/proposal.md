# Proposal

## Why
The activity header repeats information already present in thread cards. The music field competes with recent observations and currently uses a stronger green than the theme accent.

## What Changes
- Replace the generic title and observed-activity widget with a compact Rich mark rendered in Delta Corps Priest 1.
- Animate the title on click and every minute, using the existing browser text-effect engine with safe static fallback.
- Keep the visualiser outside an opaque, full-width observations footer and use a muted OS theme accent.
- Add meaningful event icons and colours; display only the theme name.

## Capabilities
### Modified Capabilities
- `office-display`: title effects, simplified header, footer separation and muted themed background.

## Impact
Browser modules, bounded title-only multiline renderer, assets, tests and README. Existing music, hooks, disclosure and container architecture remain applicable.

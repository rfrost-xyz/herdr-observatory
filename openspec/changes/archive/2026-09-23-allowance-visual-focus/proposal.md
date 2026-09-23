## Why

The deployed account cards make a large daily token count and a dense detail list prominent, even though the service does not report a token-denominated weekly quota. The weekly allowance and its pace are hard to compare at a glance.

## What changes

- Give the weekly percentage a full-width remaining-share bar.
- Show `Burn` and `Room` as two bars on one scale, with a concise pace cue.
- Keep the reset time visible and make passes compact. Remove daily token totals and sparklines from the cards while retaining the bounded account activity feed.
- Fit the cards in full-screen displays and narrow browser tiles.

## Impact

Only browser presentation, presentation tests and display documentation change. Account reads, privacy filtering and the allowance calculations remain in place.

## Why

Eight thread cards currently scroll and bury current activity in small text. The allowance change removed useful weekly pace and observed token activity along with an unreliable depletion forecast.

## What changes

- Redesign each thread as a fixed-height visual summary of native state, current activity, context, cache and token flow. Keep exact counts and provenance accessible.
- Fit up to eight cards without thread or document scrolling on 1280×720, 1920×1080 and corresponding 16:10 displays.
- Show the reported weekly allowance alongside a seven-day elapsed-time marker and a clearly labelled pace difference. Restore bounded reported daily token activity as a compact chart.
- Omit a session limit unless the account source actually supplies one. Keep depletion forecasts out of the display.

## Capabilities

### Modified capabilities

- `activity-dashboard`: glanceable visual cards and honest partial coverage.
- `office-display`: eight cards fit standard full-screen display sizes without scrolling.
- `account-allowances`: weekly pace and reported activity without fabricated quota or missing days.

## Impact

Browser presentation, tests and display documentation. Account collection and disclosure remain unchanged.

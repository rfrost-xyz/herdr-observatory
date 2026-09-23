## Why

Lifetime tokens and peak daily tokens describe past activity but do not help decide how much of the current weekly Codex allowance can be used before reset.

## What changes

- Keep the existing weekly percentage and reset time.
- Replace the two historical totals with an even-use allowance rate and average weekly allowance burn so far, both in percentage points per day.
- Leave rates unavailable when their source data is stale, incomplete or past the scheduled reset. Never convert token activity into a token-denominated quota.

## Impact

The browser account panels and their presentation tests change. The account probe, daily token sparkline, disclosure boundary and deployment interface stay the same.

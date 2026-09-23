## Calculation

For a fresh weekly allowance with a future reset, divide percentage points remaining by the fractional days until reset. Label the result `Room/day` and explain that it is an even-use guide. Recompute as time passes; after reset the value is unknown until a new allowance observation arrives.

For burn, divide the weekly percentage already used by the elapsed time in the seven-day window. Require at least one elapsed hour to avoid an unstable rate at the start of a window. Label it `Burn/day` and explain that it is an average so far, not a forecast. Reject reset timestamps outside the seven-day window. The daily token buckets may omit calendar dates, so they do not contribute to either allowance rate.

Both React and the fallback renderer use the same view calculation. They retain the existing stale and disconnected rules.

## Calculation

Use the existing seven-day window and fresh weekly remaining value. When at least one hour has elapsed, `burn = (100 - remaining) / elapsed_days` percentage points per day, and `runway_days = remaining / burn`. Display an approximate time when runway precedes the scheduled reset. Show `Beyond reset` when it reaches or exceeds reset, `0h left` for a reported zero, and an unknown mark when the source or window is invalid. This is a conditional projection from past average consumption, not a token balance or a guarantee.

## Presentation

Put the runway beside the large remaining percentage so their relationship is immediate. Keep Burn and Room on their current shared bar scale. Move the scheduled reset to the compact footer alongside passes and sample age. Preserve the same information in the React and fallback renderers, including accessible text.

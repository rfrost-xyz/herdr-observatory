## Verification

- The existing app-server account read provides a weekly `usedPercent`, seven-day window and reset time; its token activity has a separate daily-bucket shape. The live Personal and Work data omitted calendar dates, so absent token buckets were not filled with zeroes or used for an allowance rate.
- `python3 -m unittest discover -s tests -q`: 112 passed.
- All `tests/*.cjs` and `tests/*.mjs`: passed, including stale, expired and inconsistent weekly-window cases.
- `openspec validate --all --strict`: 5 passed, 0 failed. Existing long-requirement informational notices remain.
- The preview Docker image built and its bundled React output contained `Room/day` and `Burn/day` without the historical lifetime and peak rows.

The tested calculation uses only percentage remaining and the source's seven-day reset window. It does not infer a token-denominated quota.

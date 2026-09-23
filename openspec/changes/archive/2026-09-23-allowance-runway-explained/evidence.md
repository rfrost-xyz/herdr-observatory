## Verification

- Rendered synthetic 10% Personal and 99% Work samples in Chromium at 1280×720 and 520×700. The cards show runway beside remaining allowance, with Burn and Room bars and reset timing readable at both sizes.
- Account tests cover 10% remaining with about ten hours of estimated runway, beyond-reset, zero and unknown states, plus the fallback card structure.
- `python3 -m unittest discover -s tests -q`: 112 passed.
- All `tests/*.cjs` and `tests/*.mjs`: passed.
- `openspec validate --all --strict`: 5 passed, 0 failed. Existing long-requirement notices remain informational.
- Deployment verification is tracked separately from the repository change.

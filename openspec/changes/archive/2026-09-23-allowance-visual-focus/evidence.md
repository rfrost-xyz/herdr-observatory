## Verification

- Rendered synthetic Personal 10% and Work 99% account samples in Chromium at 520×700, 1280×720 and 1920×1080. The narrow tile stacks cards; the full display keeps them side by side. The weekly percentage and remaining bar lead each card, followed by Burn and Room bars on the same scale. No daily token count or sparkline appears.
- The account view test checks the rendered fallback card, low allowance, shared pace cue and absence of the daily token total.
- `python3 -m unittest discover -s tests -q`: 112 passed.
- All `tests/*.cjs` and `tests/*.mjs`: passed.
- `openspec validate --all --strict`: 5 passed, 0 failed. Existing long-requirement informational notices remain.
- The preview Docker image built and its bundled web assets contain the allowance bars and narrow-tile layout without a `Daily tokens` row.

The private account activity feed remains available. Only its account-card presentation changes.

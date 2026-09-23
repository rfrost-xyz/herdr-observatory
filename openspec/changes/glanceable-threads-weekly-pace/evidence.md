# Delivery evidence

## Thread presentation and office fit

- React and fallback cards use compact state, context and cache visuals. Exact source text remains in tooltips and accessible labels; absent usage has a distinct placeholder.
- A temporary eight-thread fixture rendered the actual React bundle and CSS in headless Chromium. At 1280×720, 1280×800, 1920×1080 and 1920×1200, all eight card `scrollHeight` values equalled their `clientHeight`, the thread grid did not scroll, and the document root fitted the viewport. The temporary fixture was removed.
- The narrow tile rules retain flowing cards and a scrollable viewport.

## Weekly and activity instruments

- Weekly pace compares reported allowance left with the time left in its seven-day window. An expired or inconsistent reset has no pace marker or inferred refill.
- The activity strip uses only reported dated daily buckets; missing dates do not create zero-valued bars. Token activity never defines an allowance.
- React and fallback account cards expose the same pace, time marker, activity and accessibility facts.

## Gates

- `python3 -m unittest discover -s tests -v`: 122 passed (with local socket permissions).
- `node --test tests/test_ui.cjs tests/test_wasm.mjs tests/test_background.mjs tests/test_title.mjs tests/test_music_title.mjs tests/test_pi_hooks.mjs tests/test_allowances.mjs`: 7 suites passed.
- `openspec validate --all --strict`: 5 items passed.
- Browser geometry and source-bound allowance cases checked as above.

Image smoke, independent review, archive and deployment evidence follow completion of those gates.

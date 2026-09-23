# Delivery evidence

## Requirement trace

| Requirement and scenarios | Implementation | Verification |
| --- | --- | --- |
| Honest allowance instruments: supported, incomplete, exhausted, missing or expired | `web/allowances.mjs`, `web/react-view.jsx` | `tests/test_allowances.mjs` weekly, missing and expired cases; four browser viewports |
| Account token activity: supported, partial, missing dates, invalid weekly window | `observatory/allowances_probe.py`, `observatory/allowances.py`, `web/allowances.mjs`, `web/react-view.jsx` | Python `test_allowances`; JavaScript daily bucket and unknown-state cases |
| Account token activity: office sharing | Existing `observatory/allowances.py` account mapping and disclosure path | Python `test_export_revalidates_current_account_mapping`, `test_feed_allowances_are_separate_opt_in_and_revalidated` |
| Glanceable activity: partial hook, valid telemetry, stale usage | `web/app.js`, `web/react-view.jsx`, `web/style.css` | UI regression suite, including partial response/cache case; eight-card browser checks |
| Glanceable activity: completed thread, checkout disclosure | Existing native-state and disclosure paths in `web/app.js` and `observatory/` | UI state-colour and count cases; Python work-filter and feed tests |
| Single-screen activity: eight threads, viewport resize | `web/style.css`, `web/app.js` | Browser geometry at 1280×720, 1280×800, 1920×1080, 1920×1200 |
| Single-screen activity: many agents, narrow tile | Existing `web/app.js` paging and narrow CSS rules | UI eight-card paging test; narrow CSS rules inspected |

## Thread presentation and office fit

- React and fallback cards use compact state, context and cache visuals. Exact source text remains in tooltips and accessible labels; absent usage has a distinct placeholder.
- A temporary eight-thread fixture rendered the actual React bundle and CSS in headless Chromium. At 1280×720, 1280×800, 1920×1080 and 1920×1200, all eight card `scrollHeight` values equalled their `clientHeight`, the thread grid did not scroll, and the document root fitted the viewport. The temporary fixture was removed.
- Independent review found a partial telemetry case with two response tiles. After assigning last-response cache its own slot, a second eight-card browser check at 1280×720 and 1920×1080 confirmed the tile rectangles do not overlap and no grid, card or document scrolling occurs.
- The narrow tile rules retain flowing cards and a scrollable viewport.

## Weekly and activity instruments

- Weekly pace compares reported allowance left with the time left in its seven-day window. Reserve and deficit are labelled in percentage points. An expired or inconsistent reset has no pace marker or inferred refill.
- The activity strip uses only reported dated daily buckets; missing dates do not create zero-valued bars. Token activity never defines an allowance.
- React and fallback account cards expose the same pace, time marker, activity and accessibility facts.

## Gates

- `python3 -m unittest discover -s tests -v`: 122 passed (with local socket permissions).
- `node --test tests/test_ui.cjs tests/test_wasm.mjs tests/test_background.mjs tests/test_title.mjs tests/test_music_title.mjs tests/test_pi_hooks.mjs tests/test_allowances.mjs`: 7 suites passed.
- `openspec validate --all --strict`: 5 items passed.
- A rebuilt Docker image passed a packaged UI and server import smoke test after the review fixes.
- Browser geometry and source-bound allowance cases checked as above.

Independent review identified the response-tile collision and percentage-point wording, both fixed and rechecked. Archive and deployment follow this evidence.

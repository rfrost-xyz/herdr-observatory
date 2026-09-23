# Delivery evidence

## Requirement trace

| Requirement and scenarios | Implementation | Verification |
| --- | --- | --- |
| Glanceable activity: partial hook and valid telemetry | `web/app.js` instrument selection, `web/react-view.jsx` and fallback card renderer, `web/style.css` | UI cases for three instruments, response-only scope, missing sample and tool summary; eight-card Chromium layout |
| Glanceable activity: stale usage, completed thread and checkout disclosure | Existing source-state and sanitisation paths in `web/app.js` and `observatory/` | UI stale/source-loss, Done state and checkout cases; Python feed disclosure tests |
| Honest allowance: supported, incomplete, exhausted, missing or expired | `web/allowances.mjs`, `web/react-view.jsx`, `web/style.css` | Allowance pace/unknown tests, fallback marker/gap/accessibility case and Chromium account-panel geometry |
| Account activity: supported, partial, missing dates, invalid window, office sharing | Existing bounded `observatory/allowances_probe.py` and disclosure path; full-width chart in both web renderers | Python allowance privacy/activity tests; JavaScript daily bucket and stale cases; Chromium chart width |
| Single-screen activity: eight threads, many agents, viewport resize, narrow tile | Three-column instrument layout in `web/style.css`; existing eight-agent paging in `web/app.js` | Browser geometry sweep; UI paging tests |

## Browser geometry

A temporary fixture used the actual React bundle and stylesheet with eight dense cards, one missing hook sample and two 30-day account charts. At 1280×720, 1280×800, 1920×1080 and 1920×1200 all cards and the thread grid fitted without scroll; the three instrument rectangles did not overlap; the document had no vertical or horizontal scroll. At 580×445 the narrow tile had no horizontal document scroll and its thread grid remained scrollable. The weekly bar and daily chart each spanned the account panel's inner width at every size. The fixture was removed.

## Gates and review

- Python suite: 122 passed.
- JavaScript suite: seven suites passed, including 83 UI and five account cases.
- Strict OpenSpec validation: five items passed.
- Docker preview image: packaged UI and server import passed.
- Independent review: pending.

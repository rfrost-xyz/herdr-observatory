# Delivery evidence

| Scenario | Implementation | Verification |
| --- | --- | --- |
| Status header and footer | React `web/react-view.jsx`, fallback `web/app.js`, and `web/style.css` | UI DOM placement and icon-only accessible state; Chromium eight-card geometry at four full-screen sizes |
| Valid, missing and response-only telemetry | Existing source-bound instruments in `web/app.js`, placed below the new header in both renderers | UI exact count, missing sample and response-only cases; image smoke |
| Stale usage and completed thread | Existing source-age and native-state paths with separate activity/tool placement | UI stale/source-loss and Done cases; Python feed tests |
| Checkout disclosure | Existing sanitised checkout leaf in the new branch box | UI hidden `.bare` case; Python disclosure tests |

## Gates

- Python: 122 passed.
- JavaScript: seven suites passed, including 83 UI cases.
- Strict OpenSpec: five items passed.
- Browser: eight dense cards at 1280×720, 1280×800, 1920×1080 and 1920×1200 fit without card, thread or document scroll. Header, metrics and footer do not overlap; 580×445 uses internal thread scroll without horizontal page scroll.
- Docker preview: packaged CSS, React bundle, fallback app and server import passed.
- Independent review: pending.

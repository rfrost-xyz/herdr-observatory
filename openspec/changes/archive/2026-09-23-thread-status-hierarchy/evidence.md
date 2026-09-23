# Delivery evidence

| Scenario | Implementation | Verification |
| --- | --- | --- |
| Valid context, cumulative tokens and cached share | `web/app.js` visible instruments; `web/react-view.jsx`, fallback DOM and `web/style.css` hierarchy | UI exact-count case; Chromium eight-card geometry and screenshot at four full-screen sizes |
| Partial hook, response-only and unknown | Existing source-bound selection and new shared status group in both renderers | UI partial/response/unknown cases and full suite |
| Stale usage, completed thread and checkout disclosure | Existing age, state and sanitisation paths with revised placement | UI age/Done/checkout cases; Python feed tests |

## Gates

- Python: 122 passed.
- JavaScript: seven suites passed, including 83 UI cases.
- Strict OpenSpec: five items passed.
- Browser: eight cards, no card, thread or document scroll, no instrument overlap or footer collision at 1280×720, 1280×800, 1920×1080 and 1920×1200. Narrow 580×445 tile keeps internal thread scroll and no document horizontal scroll. Input/output rows and cached percentage remain visible.
- Docker preview: packaged CSS, React bundle, app labels and server import passed.
- Independent review: clean at `88c3468d103ae2ac6d4938312ddac106da8c5dd1`; no actionable findings.

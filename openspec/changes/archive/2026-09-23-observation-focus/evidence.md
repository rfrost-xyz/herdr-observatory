# Verification evidence

- Thread metadata renders in one ordered line: host, Herdr pane, harness, model. Both the React and fallback renderers use the same footer structure. Chromium at 1280 × 720 and 1280 × 800 showed eight cards without document or thread scrolling, with no metadata overflow. The same geometry passed at 1920 × 1080 and 1920 × 1200.
- Activity and tool use a full-width row beneath the project and state header. Compact Chromium screenshots were visually inspected at 1280 × 720.
- The observations list renders the bounded 60-record history. A real Chromium mouse-wheel interaction scrolled it in compact and expanded views. Appending a record at the 60-record boundary retained the first visible record and its offset. Recent-only animation eligibility remains limited to four records.
- The icon button enters observation-focused native fullscreen and expands the list below the persistent title, song and thread-state header. A second click or Escape exits both modes. The expanded view has no document scroll at the four viewport sizes above.
- `npm run build:web`, 206 JavaScript tests, 122 Python tests, `openspec validate observation-focus --strict` and `git diff --check` passed on the final implementation.
- Independent review found a Page Up/Page Down conflict and a history-anchor shift. Both were corrected and regression-tested; renewed review found no remaining issue.

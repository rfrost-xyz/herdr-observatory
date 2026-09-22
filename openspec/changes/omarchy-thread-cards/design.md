## Design
Use browser-native DOM for the main interface with locally bundled mono/Nerd typography, crisp borders, small pixel accents and generous hierarchy inspired by https://omarchy.org. Eight cards in four columns and two rows receive the majority of the 16:9 screen. Keep title, project, Herdr state, host, native pane ID, harness and supported telemetry distinct and readable. Fleet metrics have full labels. Unavailable metrics remain explicit.

Preserve the existing observation, source-expiry and disclosure logic. A pure viewModel exposes readable values for rendering and tests. Maintain persistent DOM nodes updated with textContent. State/hook sequence arrivals produce decaying card impulses and a compact header equaliser; unchanged samples cannot retrigger them. No fabricated throughput or music playback. Pause, reduced motion, hidden tabs and unavailable sources suppress movement.

Map the current theme palette into CSS variables for background, foreground, surfaces, border and semantic accents, including light themes. Show the active theme name and its OS source without changing the OS or adding another sync service.

The recent-activity strip contains four bounded event lines. Reuse exact-width single-line WASM effects with full playback, fresh-event eligibility, 120-second default cooldown, 0–300 arrow adjustment and asynchronous cancellation guards. Its canvas belongs only to the current event, never any card. Keep paging, category and visual pause/fullscreen controls, with click alternatives.

## Validation
Retain observation/privacy/expiry and effect-lifecycle regression coverage. Add view-model tests for metric labels, unknowns, hook fields, activity decay, theme mapping and eight-card pages. Inspect DOM layout at 720p and 1080p, dark/light themes, busy and disconnected states; assert no document overflow and no canvas inside cards. Run all repository gates, independent review, release-image checks, then deploy and verify both hosts with rollback.

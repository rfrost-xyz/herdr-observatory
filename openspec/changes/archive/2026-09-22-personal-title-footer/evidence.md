# Delivery evidence

Runtime commit: `baa42fe`. Branch: `feat/activity-dashboard`, continuation of PR #1. No programme register exists.

| Requirement / task | Implementation | Verification |
| --- | --- | --- |
| Personal title artwork, 1.1 | title.mjs, title-specific factory in effects.mjs, restored glyph notice | Seven title tests cover minute cadence, complete playback, hidden/reduced cancellation, async/failure fallback and native cell styling; all 37 title effects complete at bounded dimensions with a settled final artwork frame |
| Header and event semantics, 1.2 | app.js, index.html | Removed activity widget/bookkeeping and OS theme suffix; UI regressions verify accessible title button, event icon/colour mapping and event-line bounds |
| Opaque footer, 1.3 | style.css, app.js | Browser 720p: footer 582..720, canvas bottom 582, eight cards client/scroll heights 177/177; 1080p: footer 912..1080, canvas bottom 912, cards 280/280; footer spans full viewport width, no document overflow |
| Muted theme accent, 1.3 | background.mjs | Palette test verifies all inks derive from accent even with unrelated green, bounded towards background for light/dark palettes and click/music peaks; orange fixture inspected in browser |
| Title interaction, 1.1 / 2.1 | title button and canvas | Browser click visibly animates only the title; event/card content remains readable; no browser console warnings/errors |
| Required gates, 2.1 | regression suite | 63 Python and 139 JavaScript tests passed (202 total), including all 37 event and all 37 title effects; JS syntax, strict OpenSpec and whitespace checks passed; 63 Python tests also pass in final image |
| Independent review, 2.1 | complete change | No code findings; stale README sentence corrected to distinguish title and event effects; final renewed approval received |
| Deployment, 2.2 | both Compose instances | Both healthy on herdr-observatory:baa42fe with unless-stopped; app.js, title.mjs, effects.mjs, background.mjs and style.css bytes match checkout on both hosts |
| Preserved data boundaries, 2.2 | existing services | Personal/Work profiles correct, themes equal, shared music available with matching title, all remote disclosed agents Work |
| Live UI, 2.2 | iapetus display | Rich button present, old widget absent, theme name has no suffix, music visible and canvas stops exactly at footer top; no document overflow at 1080p |
| Documentation and cleanup, 2.3 | README, specs, archive | README/provenance updated; synthetic QA server stopped, tab closed and viewport reset, temporary files removed; only current baa42fe and rollback 9826783 image tags retained on both hosts |

The previous image selection and private configuration remain in `.env.previous` and `config/config.json.previous`. No new daemon, host adapter, music transport or OS configuration was introduced. The local and remote applications still run independently from versioned images. The PR remains open and unmerged.

Validation limits: remote served assets and health were verified over SSH; no Windows desktop interaction was performed. Synthetic high-spectrum data tested muted accent rendering and full eight-card geometry; real source delivery and the live Personal UI were checked separately.

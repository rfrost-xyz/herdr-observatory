# Delivery evidence

Runtime: `9826783` (music/visual fleet `ddd17c4`, direct upstream renderer `9826783`). Branch: `feat/activity-dashboard`, continuation of PR #1. No programme register exists.

| Requirement / task | Implementation | Verification |
| --- | --- | --- |
| Single-screen activity, 1.1 | app.js, style.css | Eight synthetic cards fit at 1280x720 and 1920x1080, no document overflow or clipped card content; UI tests cover >8 default paging and manual hold |
| Readable fleet and identity, 1.1 | app.js, style.css | Percentage gauges distinguish unknown from zero; labelled Agent/Pane/Host, aria-labelled unavailable values; UI tests and browser |
| Simplified controls, 1.2 | app.js, index.html | No pause/timer controls or their key handlers; fixed internal 120 seconds; reduced-motion and source-loss tests |
| Interactive field, 1.3 | background.mjs, upstream notice | Direct pinned Omarchy noise/dither/spectrum/click adaptation; seven renderer tests include expiry, bounded grid, pointer/click and stationary reduced motion; browser with actual music and readable foreground |
| Optional music disclosure, 2.1 | music.py | Eight tests cover allowlist, framing/read-only methods, oversized numbers, expiry, receiver and unavailable paths; live cliamp returns ten measured bands |
| Independent transport, 2.2 | core.py, server.py, compose.music.yaml | Endpoint origin/method restrictions; narrow source mount; existing SSH and receiver module in same image, no new installed daemon |
| Live shared playback, 2.2 / 3.2 | both deployed containers | Both endpoints available/playing, titles and artists match without committing their values; spectrum changes across samples; receiver age 0.06s in final check |
| Theme and Work boundary, 3.2 | existing collector and feed | Personal/Work profiles correct, palettes equal, every remote visible agent category Work; all served app.js/background.mjs/style.css bytes match reviewed checkout |
| Review and checks, 3.1 | implementation and regression suite | 63 Python + 91 JavaScript tests passed, including all 37 WASM effects; JS syntax, strict OpenSpec validation and diff whitespace checks passed |
| Publication and cleanup, 3.3 | README, canonical spec, archive, PR #1 | Source attribution and operating/rollback instructions updated; temporary QA server, browser tab/viewport and research/test files removed; only current and rollback Observatory image tags retained on both hosts |

Independent review found oversized-number failure and reduced-motion pointer animation; both were fixed with regressions. Renewed review approved music integration, then the direct upstream renderer, then the final text-contrast backing. No actionable findings remain.

Both hosts run `herdr-observatory:9826783`, healthy with `unless-stopped`. The prior `c628ff5` image, `.env.previous` and `config/config.json.previous` are retained for rollback. Source configuration adds only an optional cliamp directory mount. Recipient stores the bounded latest sample in its existing private feeds volume. Intermediate `ddd17c4` and obsolete `6ad74b4` image tags removed on both engines.

Limitations: Spotify standalone playback is not captured. cliamp's measured spectrum is used, including playback through its own providers. The upstream silent music timeline and audio player are omitted; no audio crosses hosts. No synthetic beat timeline is generated. Ten captured bands interpolate to 32 visual bands. Background quick clicks are supported, without press-and-hold charge. No Windows desktop interaction or live Spotify-provider playback was tested.

# Acceptance evidence

Implementation d5f9b49; final console-spacing correction and deployed runtime 8283108. Both engines run image sha256:1af394af27b01a8ab3b26c65b12dbbd31b41ce15b98cb4c18bc156c9c5b066f6.

| Requirement / scenario | Implementation | Verification |
| --- | --- | --- |
| Living operations console / active operations | web/app.js cardEffects, activityPhase, snapshotConsole; web/style.css; index.html | SVG perimeter circuits, full-card scan planes and drifting textures replace ECG strips. Browser computed offsets change across 700, 1700 and 2500 ms despite redraws; nine machine/agent effects have distinct periods. Console test verifies escaping, bounded records, real capture timestamps and stale-source status |
| Living operations console / no live activity | visibleAgents/usable, conditional cardEffects, reduced-motion CSS | Node regressions remove effects for inactive and stale agents. Chromium forced reduced-motion reports animation-name none for all nine card circuits; global override covers scan, texture, core and cursor animations |
| Fixed screen and readable console | CSS grid and console character columns | Actual Chromium content viewports 1280x720 and 1920x1080 report no document, agent, machine or resource-card vertical overflow. Screenshots inspected; journal compacted to two rows and timestamp collision corrected. Final 1080p check confirms timestamp content fits its columns |

37 Python tests pass on the host and in the redesigned image; nine Node tests, JavaScript syntax, strict OpenSpec and diff checks pass. Independent review approved d5f9b49 and renewed approval for 8283108 with no actionable findings.

Live deployment: both containers healthy with identical final image, both sources online, Personal Work publication healthy, new console assets available from both servers, and personal-project exclusion retained in the Work API. Previous image/config remain available for rollback. Backend and collection boundaries are unchanged.

Root README describes card-level effects and the derived console. The displayed observe prompts are interface labels, not executed shell commands or raw terminal output. Decorative activity does not imply model token or tool-call throughput. Browser evidence uses synthetic fixtures; the physical Windows display is not independently verified. No programme register applies.

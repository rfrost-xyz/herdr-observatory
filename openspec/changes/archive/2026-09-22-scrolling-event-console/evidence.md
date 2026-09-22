# Verification evidence

Runtime commit: `ed536f9`.

| Requirement scenario | Implementation | Verification |
| --- | --- | --- |
| Fresh and duplicate observations | web/app.js shared scene and scrolling record queue; typed status/readiness/sequence observations | UI regressions cover baseline, duplicate samples, transitions, disconnect/expiry and interpolation bounds |
| Sparse milestone artwork | web/stamps.js; live-only DONE/INPUT stamps; attribution in web/vendor/STAMPS-NOTICE | Six-second expiry, thirty-second cooldown, current-source checks and effect capture exclusion tested |
| Animation accessibility and disclosure | Existing transcript, source freshness, pause/reduced-motion and complete WASM sessions retained | Python privacy suite; UI pause/reduced-motion/stale capture tests; all 37 WASM effects complete |
| Denser padded terminal | Shared 140x44 grid, responsive 20–40px side inset and 16px vertical inset | Chromium synthetic 1280x720 and 1920x1080: no document overflow; font 13.29/20.25px and side inset 32/40px; screenshot inspected with visible scrolling feed and DONE artwork |

42 Python tests and 69 Node tests passed, including all 37 WASM effects. JavaScript syntax, strict OpenSpec validation and diff whitespace checks passed. Reduced-motion browser check retained the feed and suppressed artwork. Browser QA used synthetic data and does not prove the physical Windows display setup.

Independent review approved after fixing temporary banner capture during long effects. Renewed review approved the final monotonic scroll-clock fix, with no actionable findings. Browser QA caught an RAF/performance clock mismatch; the renderer now uses the event clock and bounded interpolation.

README documents the layout, stamps, metadata and polling limitations. events.subscribe and agent.explain are documented opportunities, not implemented integrations. Existing Work filtering and Docker boundaries are unchanged.

## Deployment and cleanup

Packaged Python tests passed (42). Deployment verification covered both profiles, current browser assets, online sources, Work-category filtering and exclusion of personal-project data. Container hardening and restart policy remain unchanged.

One previous-image rollback was retained per deployment. The synthetic QA server was stopped and its browser profiles, screenshots and task temporary files removed. No host service or browser autostart was added. Host identifiers and operational deployment details are omitted from this public evidence record.

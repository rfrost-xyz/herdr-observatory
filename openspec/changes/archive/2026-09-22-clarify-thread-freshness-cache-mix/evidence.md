## Traceability

| Requirement | Implementation | Verification |
| --- | --- | --- |
| Independent hook/usage ages and historical activity | `web/app.js`, `web/style.css` | UI source-age and past-observation tests |
| Three-part cache balance and denominator | `web/app.js`, `web/style.css` | UI cache reconciliation and segment tests |
| Compact readability | `web/style.css` | Target viewport inspection if available; exact visual boundary recorded below |

## Gates
The full 107-test Python suite and all JavaScript suites pass. Syntax, whitespace and strict OpenSpec validation pass. Independent review found two cache balance edge cases, both corrected with regression tests. The in-app browser is unavailable in this environment, so the new 720p/1080p layout has not been visually inspected. The 720p CSS raises card labels and values, freshness labels can wrap, and historical activity uses shorter wording to limit vertical growth. A real viewport remains the acceptance boundary.

The `f2e732b` image was built from the committed Git archive, excluding the unrelated uncommitted installer edits, and its bundled Python suite passed with networking disabled. Both hosts run healthy containers from identical image `sha256:e0e10f2684a33e07e03629961f456611aad4c940180a22b0bd05e7a7992d786d`. Served JavaScript and CSS hashes match on both hosts. The Work endpoint reports only Work agents. Both hosts retain the previous `ab1fbb4` image and matching environment/config rollback copies. The adapter payloads did not change in this release.

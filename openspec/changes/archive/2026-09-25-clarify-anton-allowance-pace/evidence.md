# Acceptance evidence

## Scope and authority

Approved local allowance presentation refinement. Existing feature worktree remains uncommitted; no forge, remote service, collector or private configuration changes. Earlier dirty work was preserved.

## Traceability

| Requirement | Implementation | Verification |
| --- | --- | --- |
| Truthful current state | State.js allowancePaceBand; AllowanceCard.qml separate fill/strip/tick | 29 projection cases include exact zero/five/ten boundaries and unknown inputs; Qt verifies independent colours, interval positions, 6px/2px heights, zero/full balance and missing data |
| Restrained visual feedback | Clipped pacing region contains halo and BurnEffect; foreground percentage | Qt verifies parent clipping and tint, existing colour transition close/reduced-motion cancellation; installed hovered popover inspected |

## Gates

- 173 Python tests passed, including guarded installation/uninstall coverage.
- All seven required browser test files passed; web/app.js syntax passed.
- 29 projection cases and 13 Qt component tests passed.
- QML parsing, Omarchy plugin validation and git diff --check passed.
- Independent reviewer approved exact implementation with no findings and independently passed projection/Qt checks.
- Strict OpenSpec validation passed before synchronisation; final canonical validation recorded at archive.

## Installed acceptance

Copied only State.js, AllowanceCard.qml and README.md to the owned local plugin and restarted the shell. Byte parity verified. Actual popover shows the accent balance, foreground percentage, separate thinner deficit strip, neutral expected tick, concise comparison tooltip and confined hover effects. Both hosts report, all three threads have usage and both allowances are available. Latest shell log has no plugin errors. Initial bounded transcript replay remains unchanged.

## Completion

Canonical requirements synchronised with all prior scenarios retained and exact delta parity checked. Change archived after acceptance. Temporary source/installed backups and visual capture removed. No Git history or external publication performed.

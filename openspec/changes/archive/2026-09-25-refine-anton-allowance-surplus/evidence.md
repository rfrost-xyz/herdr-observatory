# Acceptance evidence

## Scope

Approved local allowance refinement in the existing uncommitted feature worktree. No collector, private configuration, remote service, Git history or forge changes.

## Requirement traceability

| Requirement | Implementation | Verification |
| --- | --- | --- |
| Truthful current state | State.js allowancePaceReading; AllowanceCard.qml signed value and diagonal hatch | Numerical signs, symmetric rounding, negative-zero suppression and shared severity tested; Qt checks hatch and solid-fill endpoints, original 6px height, missingness and narrow-row geometry |
| Restrained visual feedback | Positive-only lower strip, green halo and particles | Qt pointer hover exercises positive, deficit, zero, unavailable, hidden and reduced-motion transitions; main bar and remaining percentage retain their colours |

## Gates

- 173 Python tests passed, including installation/uninstall fixtures.
- 30 projection cases and 14 Qt tests passed.
- Seven browser test files and web/app.js syntax passed.
- QML parsing, plugin validation, strict OpenSpec validation and diff checks passed.
- Independent reviewer approved without findings and independently passed projection and Qt tests.

## Installed acceptance

Copied only State.js, AllowanceCard.qml and README.md and restarted the shell. Installed byte parity confirmed; no plugin errors in the latest shell log. Actual popover inspected: two deficits show clear static accent hatching to the expected tick, signed warning values beside foreground balances, and no deficit particles or glow. The positive hover path was verified with production QML components and synthetic fixtures. Both hosts and allowances remain available; all three threads report usage. Private identities were not saved in repository evidence.

## Completion

Canonical requirement and scenario parity verified. Archived after review and installed acceptance. Task-owned backups and capture removed. No external publication.

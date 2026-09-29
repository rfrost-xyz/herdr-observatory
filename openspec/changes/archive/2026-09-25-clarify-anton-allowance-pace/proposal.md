# Proposal

## Why

The current pace-coloured fill confuses remaining allowance with burn pace and makes every small deficit red. The approved design separates the two readings with clear, predictable thresholds.

## What Changes

- Keep a solid theme-accent remaining bar and foreground percentage.
- Put a thinner pacing interval below it with green, amber and red thresholds.
- Confine hover effects to the pacing strip and preserve the expected-position tick and concise comparison.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `omarchy-companion`: Separate allowance and pace presentation, thresholds and hover geometry.

## Impact

Local QML presentation, shared projection helper, focused fixtures and documentation only. Existing local worktree and installation; no collector, remote service, Git history or forge changes.

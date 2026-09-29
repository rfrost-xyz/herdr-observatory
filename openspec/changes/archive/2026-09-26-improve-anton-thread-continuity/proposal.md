## Why

Anton now has trustworthy fleet and usage readings, but a restart loses native replay progress, startup resembles a failed connection, and longer thread lists push allowances away. A small turn-duration reading and failed-child mark can add useful context without enlarging the thread metrics.

## What Changes

- Add an inline stopwatch for current or last turn wall-clock duration, with accumulated completed and aborted turn time on hover and explicit incomplete coverage.
- Distinguish connecting, connected and unreachable machines, and securely retain bounded numeric replay checkpoints between plugin runs.
- Keep the header and filters fixed, bound thread scrolling independently of allowances, and retain compact short-list layout and instant collapse.
- Add a small red failure notch to the existing subagent dial from validated outcome counts.
- Exercise production QML with synthetic dark/light, long-title, crowded, missing-value and low-allowance fixtures.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `omarchy-companion`: compact turn timing, explicit connection phases, accessible bounded scrolling and truthful subagent failure indication.
- `harness-telemetry`: validated wall-clock turn summaries and private restart-safe native replay checkpoints.

## Impact

Changes are limited to the local native collector, QML projection/components, installation ownership lists, tests and documentation. Existing Work publication, hook metadata and allowance design remain compatible. No new listener, daemon, database, remote installation, Docker change or repository publication is required.

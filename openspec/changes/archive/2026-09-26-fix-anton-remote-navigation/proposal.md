## Why

A reachable remote host can supply thread data while clicking its thread fails because the remote Herdr version does not support local machine API forwarding. Navigation must use the exact saved host/session without relying on that unsupported forwarding capability.

## What Changes

- Focus a remote pane over bounded SSH using the unique enabled saved machine profile and its session.
- Resolve the remote Herdr executable in the non-interactive SSH environment, including the established user-local installation path.
- Preserve explicit remote/session selectors through the terminal launcher, preventing a named launcher alias from replacing the requested command with bare Herdr.
- Keep one focus attempt, exact-host routing and the most recently selected matching terminal; report concise actionable failures.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `omarchy-companion`: remote navigation compatibility, exact routing and bounded failure reporting.

## Impact

Only the navigation helper, targeted tests and documentation change. No collector, allowance, UI layout, remote installation, service, Docker or forge change is required.

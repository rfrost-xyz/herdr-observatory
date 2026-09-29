## Context

Live investigation reproduced an unsupported remote machine API forwarding error through the saved Herdr machine profile. Direct SSH collection remained healthy. A plain non-interactive remote `herdr` command was also absent from PATH, while the existing user-local executable was available.

## Goals / Non-Goals

Restore explicit remote thread navigation using the exact saved profile/session, preserve last-selected window matching, avoid duplicate focus attempts and provide concise failures. Do not change agent input, lifecycle authority, collectors, remote services or desktop design.

## Decisions

- Resolve the same unique enabled saved profile used to identify the clicked host. Keep pane validation and explicit session binding.
- Use a bounded direct SSH focus operation for remote targets rather than invoking unsupported machine API forwarding first. Resolve the remote executable using established available locations and quote validated selectors safely.
- Keep local navigation local. Never retry against another host or fall back to Local on failure.
- Focus the exact pane before raising the most recently focused matching terminal or launching the matching remote view. Existing window-selection logic remains authoritative.
- Separate the terminal launcher route from the explicit Herdr command with its supported argument delimiter. Live command-line inspection showed that an unseparated `terminal herdr` route selected a named bare-Herdr launcher and discarded remote/session selectors.
- Surface a concise relevant failure without leaking raw command output or presenting a healthy collection feed as proof that navigation succeeded.

## Risks / Trade-offs

SSH collection and thread navigation have different remote executable and session requirements. Tests must cover a missing non-interactive PATH, exact session/pane forwarding, command quoting, one-attempt failure and unchanged window matching. A genuinely absent executable or unreachable host remains a clear error, requiring no remote mutation.

## Verification and Delivery

Root owns helper/tests, independent review, local installed update and actual remote click verification. This coordinator owns specifications/docs and final local archive. Record meaningful targeted gates and actual installed behaviour before canonical sync/archive. Preserve the dirty worktree and existing private configuration. No Git/forge or remote service operations are authorised.

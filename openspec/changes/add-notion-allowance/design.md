# Context

Codex allowance collection invokes bounded read-only account RPCs through the
already authenticated Codex process. Authentication remains outside the popover.
The same principle applies to Notion.

# Decisions

The popover is passive allowance presentation. Provider login is performed
separately using provider-supported authentication. No extension or setup card is
an acceptable substitute for the requested integration.

# Source Blocker

On 2026-09-29, installed ntn 0.23.10 exposes no monthly allowance command. Its
public API catalogue has no personal monthly allowance endpoint. The catalogue's
agent credit-limit mutation is unrelated and must not be used. A prior read of
the web usage endpoint with the CLI token returned HTTP 401. The official help
centre documents usage in Notion Settings, which does not establish a CLI/API
contract. Do not promise that CLI authentication alone provides allowance data.

# Next Acceptance Gate

Establish a read-only, account-bound allowance source usable through the existing
provider-owned authentication model. If unavailable, keep this change blocked;
do not invent readings, transfer credentials or add a new login system.

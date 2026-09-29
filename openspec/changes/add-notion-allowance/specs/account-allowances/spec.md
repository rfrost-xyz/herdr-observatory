## ADDED Requirements

### Requirement: Browser-owned Notion monthly allowance
The plugin SHALL optionally show the explicitly configured Notion user/workspace monthly AI allowance used percentage, remaining balance and source reset date. The browser SHALL retain authentication and send only allowlisted observations through a bounded local native messaging bridge. Anton SHALL NOT read browser cookies, CLI tokens, pages or transcripts. The extension SHALL request access only to app.notion.com and native messaging, with scheduled refresh. Purchased credits and spending controls SHALL remain out of scope.

#### Scenario: Valid monthly observation
- **WHEN** the configured browser account reports a valid monthly used/limit ratio and future reset
- **THEN** the Notion card shows monthly usage and reset date beside the existing providers without assuming a weekly or fixed 30-day pacing window.

#### Scenario: Invalid or unavailable source
- **WHEN** the session expires, identity differs, a response is malformed, a sample exceeds ten minutes, or its reset passes
- **THEN** Notion is unavailable, zero is never substituted, and existing Codex observations remain unaffected.

#### Scenario: Account isolation and retirement
- **WHEN** an unconfigured extension or account sends an observation, or the owned plugin is retired
- **THEN** the observation is rejected and cannot create a configured account or write state after retirement.

#### Scenario: Browser integration removal
- **WHEN** the optional integration is removed
- **THEN** only its receipt-owned host registration and receipt are removed, preserving unrelated browser settings, credentials and integrations.

## MODIFIED Requirements

### Requirement: Account-bound allowance observation
The native plugin SHALL expose only explicitly labelled accounts, deduplicate the same account across machines, and use the actual source account identity rather than the source host as account authority. Ordinary telemetry SHALL contain only labels, plan, bounded allowance values and timestamps, with no raw account identifiers, email, credentials or session material. Collection SHALL be read-only and SHALL NOT redeem resets, change login, mount credentials or install a persistent host process.

The Codex source SHALL be a bounded native read-only account RPC, available independently of threads. An optional Notion source SHALL use the browser-owned, explicitly bound monthly observation contract. Remote sources SHALL invoke the marked native peer over SSH without a Docker exporter or Python helper.

Transport sanitisation and presentation SHALL tolerate a source allowance timestamp up to one second ahead of local time, preserving the original timestamp and strict cache ordering. Larger future offsets and observations older than ten minutes SHALL be rejected. Weekly reset and reset-credit expiry checks SHALL continue to use actual local time without a grace period.

#### Scenario: Account moves machine
- **WHEN** an account is signed in on another configured host
- **THEN** the same configured Personal or Work label applies and duplicate observations produce one account panel.

#### Scenario: Unrecognised account
- **WHEN** a source reports an account not explicitly mapped in private configuration
- **THEN** its allowance data is absent from the popover and ordinary peer responses.

#### Scenario: Small peer clock offset
- **WHEN** a mapped native peer allowance is at most one second ahead of local time
- **THEN** it remains available with its original source time, while larger future offsets, stale samples and expired resets remain unavailable.

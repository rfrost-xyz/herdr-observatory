# Spec Delta

## MODIFIED Requirements

### Requirement: Account-bound allowance observation
The native plugin SHALL expose only explicitly labelled accounts, deduplicate the same account across machines, and use the actual source account identity rather than the source host as account authority. Ordinary telemetry SHALL contain only labels, plan, bounded allowance values and timestamps, with no raw account identifiers, email, credentials or session material. Collection SHALL be read-only and SHALL NOT redeem resets, change login, mount credentials or install a persistent host process.

The source SHALL be a bounded native read-only account RPC, available independently of threads. Remote sources SHALL invoke the marked native peer over SSH without a Docker exporter or Python helper.

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

### Requirement: Account token activity
The native plugin SHALL read supported ChatGPT-backed account token-activity summaries and daily buckets through its authenticated, read-only account source, bind them to explicit account mapping and keep them separate from local session cache usage. It SHALL export only bounded numeric totals, valid bucket dates, account labels and sample times. The native account state MAY retain bounded dated observations with account scope explicit. Missing dates SHALL NOT appear as zero or contribute to the sum. Activity SHALL NOT be converted into a remaining quota, cost or depletion forecast. Missing, unsupported, stale or malformed account usage SHALL remain unavailable without hiding a valid weekly allowance. Reads SHALL remain local to the plugin and explicitly configured peer transport; no web forwarding SHALL occur.

#### Scenario: Supported account activity
- **WHEN** a mapped account returns valid daily token buckets
- **THEN** the native account state retains only reported dates and their bounded numeric values, separately from weekly allowance pace.

#### Scenario: Partial or unsupported response
- **WHEN** activity buckets are absent or malformed
- **THEN** activity remains unavailable while a valid weekly balance remains visible.

#### Scenario: Missing dates
- **WHEN** reported dates have gaps
- **THEN** no omitted date is exported or added as zero.

#### Scenario: Expired or inconsistent weekly window
- **WHEN** the allowance is stale or its reset window is invalid
- **THEN** pace is unavailable without inventing a token balance.


#### Scenario: Office sharing
- **WHEN** an existing configuration still names a retired office publisher
- **THEN** explicit migration removes that forwarding configuration and account summaries remain local to the plugin, without modifying the old remote service.

## REMOVED Requirements

### Requirement: Explicit allowance sharing
**Reason**: The supported product is the local Omarchy plugin; web display and forwarding are retired.
**Migration**: Use the native plugin current-state view. Existing external web containers are left untouched, and this plugin no longer publishes to them.

### Requirement: Honest allowance instruments
**Reason**: The retired web footer and its fixed Personal/Work panels are superseded by the native popover presentation contract.
**Migration**: Use the Omarchy popover Truthful current state and Private account identity display requirements for allowance balance, pace, reset and concealment behaviour.

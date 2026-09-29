# Account allowances

## Purpose

Provide passive, account-bound visibility of Codex subscription allowances and reset passes across the existing private fleet.

## Requirements

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

### Requirement: Fleet-bound account sources
When fleet discovery is enabled, native remote allowance collection SHALL follow enabled saved machine profiles independently of whether their hosts have active threads. Explicit profile-bound source overrides SHALL retain private mappings and follow the profile's current target. A removed or disabled profile SHALL cease contributing an allowance source; an otherwise unreferenced target SHALL stop its probes and invalidate late results. Another enabled profile or explicit source for the same target SHALL retain that shared account worker. Label/session-only changes and unchanged discovery results SHALL preserve account collection cadence. Account identity SHALL remain authoritative: only explicitly mapped accounts SHALL appear, and observations of the same account on multiple machines SHALL produce one row. Discovery SHALL NOT create account mappings or expose account email in shared snapshots.

#### Scenario: Newly discovered empty machine
- **WHEN** an enabled saved machine has no threads and its native peer reports an already mapped account
- **THEN** its valid allowance is eligible independently of thread activity and is deduplicated with other readings of that account.

#### Scenario: Removed source has an in-flight result
- **WHEN** a profile is disabled, removed or changes target during an allowance request, leaving its former target without any active reference
- **THEN** the retired result cannot restore its old source, while unrelated valid account readings remain available.

#### Scenario: Stable shared account route
- **WHEN** only a profile label/session changes, the inventory repeats unchanged, or another enabled profile still uses the same target
- **THEN** the account worker and its refresh cadence remain unchanged.

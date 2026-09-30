# Spec Delta

## MODIFIED Requirements

### Requirement: Account-bound allowance observation
The native plugin SHALL expose only explicitly labelled accounts, deduplicate the same account across machines, and use the actual source account identity rather than the source host as account authority. Ordinary telemetry SHALL contain only labels, plan, source status, bounded allowance window values and timestamps, with no raw account identifiers, email, credentials or session material. Collection SHALL be read-only and SHALL NOT redeem resets, change login, mount credentials or install a persistent host process.

The source SHALL be a bounded native read-only account RPC, available independently of threads. Remote sources SHALL invoke the marked native peer over SSH without a Docker exporter or Python helper.

Transport sanitisation and presentation SHALL tolerate a source allowance timestamp up to one second ahead of local time, preserving the original timestamp and strict cache ordering. Larger future offsets and observations older than ten minutes SHALL be rejected. Window reset and reset-credit expiry checks SHALL continue to use actual local time without a grace period.

#### Scenario: Account moves machine
- **WHEN** an account is signed in on another configured host
- **THEN** the same configured Personal or Work label applies and duplicate observations produce one account panel.

#### Scenario: Unrecognised account
- **WHEN** a source reports an account not explicitly mapped in private configuration
- **THEN** its allowance data is absent from the popover and ordinary peer responses.

#### Scenario: Small peer clock offset
- **WHEN** a mapped native peer allowance is at most one second ahead of local time
- **THEN** it remains available with its original source time, while larger future offsets, stale samples and expired resets remain unavailable.

#### Scenario: Several mapped accounts across machines
- **WHEN** two mapped accounts are each observed on more than one configured source
- **THEN** the popover snapshot carries exactly one row per mapped account, each from its newest valid observation, in a stable order sorted by private account key.

### Requirement: Account token activity
The native plugin SHALL read supported ChatGPT-backed account token-activity summaries and daily buckets through its authenticated, read-only account source, bind them to explicit account mapping and keep them separate from local session cache usage. It SHALL export only bounded numeric totals, valid bucket dates, account labels and sample times. The native account state MAY retain bounded dated observations with account scope explicit, and peer allowance responses MAY carry them. The popover snapshot SHALL NOT carry token-activity summaries or daily buckets, because no presentation consumes them. Missing dates SHALL NOT appear as zero or contribute to the sum. Activity SHALL NOT be converted into a remaining quota, cost or depletion forecast. Missing, unsupported, stale or malformed account usage SHALL remain unavailable without hiding a valid allowance window. Reads SHALL remain local to the plugin and explicitly configured peer transport; no web forwarding SHALL occur.

#### Scenario: Supported account activity
- **WHEN** a mapped account returns valid daily token buckets
- **THEN** the native account state retains only reported dates and their bounded numeric values, separately from allowance pace.

#### Scenario: Partial or unsupported response
- **WHEN** activity buckets are absent or malformed
- **THEN** activity remains unavailable while a valid allowance window remains visible.

#### Scenario: Missing dates
- **WHEN** reported dates have gaps
- **THEN** no omitted date is exported or added as zero.

#### Scenario: Expired or inconsistent weekly window
- **WHEN** the allowance is stale or its reset window is invalid
- **THEN** pace is unavailable without inventing a token balance.

#### Scenario: Office sharing
- **WHEN** an existing configuration still names a retired office publisher
- **THEN** explicit migration removes that forwarding configuration and account summaries remain local to the plugin, without modifying the old remote service.

#### Scenario: Token activity stays off the popover wire
- **WHEN** a mapped account's cached or peer observation carries lifetime, peak or daily token activity
- **THEN** the popover snapshot row for that account contains none of those values, and its allowance window and reset values are unaffected.

## ADDED Requirements

### Requirement: Provider-neutral allowance rows
Each allowance row in the popover snapshot SHALL describe its account without naming provider-specific fields. It SHALL carry:
- stable identity: provider, provider label, account id and label;
- a source status of available, unavailable or auth_needed;
- an optional source-supplied status text, bounded to a single printable line of at most 80 characters, or null;
- plan, source sample time, reset-pass count and reset-pass expiry, each null when unknown;
- a list of at most eight allowance windows.

Each window SHALL carry a kind, a display label, a used percentage from 0 to 100 or null, a reset time in Unix seconds or null, a positive duration in seconds of at most 366 days, and whether it is the pacing window. Every field SHALL be present in every row, with null where unknown. An unavailable or auth_needed row SHALL carry no windows, and neither SHALL a row whose source observation is stale. A row SHALL be available only while a valid, mapped, fresh source observation exists.

The runtime SHALL select the pacing window by duration, never by field or list order. A Codex observation SHALL yield exactly one pacing window of kind `weekly`. That window SHALL be chosen as the single source window whose duration is 10080 minutes, and SHALL carry a duration of 604800 seconds. A window whose reset time has passed SHALL have both its used percentage and its reset time invalidated. Reset-pass expiry SHALL invalidate the reset-pass count without a refill. The reset-pass count SHALL be the source's native available count. Bounds and original source times SHALL be enforced when rows are built. The runtime SHALL NOT synthesise status text, windows or rows for a provider or account that has no configured mapping.

#### Scenario: Codex account with a weekly allowance
- **WHEN** a mapped Codex account reports 40% used of its 10080-minute window, resetting in the future, and one available reset pass
- **THEN** its row has status available, null status text, and one window of kind `weekly` with used percentage 40, that reset time, duration 604800 and pacing true, plus reset count 1.

#### Scenario: Window order does not select the pacing window
- **WHEN** a Codex source lists a 300-minute window first and the 10080-minute window second
- **THEN** the pacing window is the 10080-minute window, with its own used percentage and reset time.

#### Scenario: Past reset invalidates the balance
- **WHEN** a mapped observation's weekly reset time is at or before local time
- **THEN** the pacing window's used percentage and reset time are both null, while a valid reset-pass count remains.

#### Scenario: Mapped account without a current observation
- **WHEN** a configured account has no valid, fresh, mapped observation
- **THEN** its row has status unavailable, null status text and no windows, with its identity preserved.

#### Scenario: Malformed or oversized values
- **WHEN** an observation carries a non-integer or out-of-range used value, a negative reset count, more than the bounded number of rows, or unknown private fields
- **THEN** the affected values are null or the input is rejected, and no unknown field reaches the popover snapshot.

### Requirement: Legacy allowance source compatibility
The local runtime SHALL accept allowance observations in the shape emitted by installed peers' `--allowances-probe` and stored in the private allowance cache before this change, and SHALL convert them into provider-neutral rows. Extra fields in a peer response SHALL be ignored. The runtime's own `--allowances-probe` output and private cache format SHALL remain readable by runtimes that predate this change. The existing bounds, mapping, ten-minute staleness and one-second future tolerance SHALL apply to legacy input unchanged.

#### Scenario: Unchanged peer row
- **WHEN** an installed peer that has not been updated returns a mapped account row with `weekly_remaining`, `weekly_resets_at`, `reset_count`, token-activity fields and an extra unrelated field
- **THEN** the local snapshot shows that account as available, with an equivalent weekly pacing window and reset count, and none of the extra or token-activity fields.

#### Scenario: Cache written before the update
- **WHEN** the local runtime starts with an allowance cache written by the previous runtime version
- **THEN** its fresh mapped entries appear as available provider-neutral rows from the cache alone, even when every new account read fails, and stale entries appear as unavailable.

#### Scenario: Older local runtime reads a new peer
- **WHEN** a runtime that predates this change probes a peer running the new runtime
- **THEN** it receives the same row shape as before and continues to show that account's allowance.

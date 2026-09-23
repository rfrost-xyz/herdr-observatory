# Account allowances

## Purpose

Provide passive, account-bound visibility of Codex subscription allowances and reset passes across the existing private fleet.

## Requirements

### Requirement: Account-bound allowance observation
The service SHALL expose only explicitly labelled accounts, deduplicate the same account across machines, and use the actual source account identity rather than the source host as account authority. Browser output SHALL contain only labels, plan, bounded allowance values and timestamps, with no raw account identifiers, email, credentials or session material. Collection SHALL be read-only and SHALL NOT redeem resets, change login, mount credentials or install a persistent host process.

#### Scenario: Account moves machine
- **WHEN** an account is signed in on another configured host
- **THEN** the same configured Personal or Work label applies and duplicate observations produce one account panel.

#### Scenario: Unrecognised account
- **WHEN** a source reports an account not explicitly mapped in private configuration
- **THEN** its allowance data is absent from browser responses and publication.

### Requirement: Honest allowance instruments
The footer SHALL persistently show Personal and Work panels. Each SHALL lead with the reported weekly percentage remaining, a full-panel-width remaining-share bar, scheduled reset and sample age. When a fresh future reset falls inside the seven-day window, the same bar SHALL mark time remaining and visually distinguish the percentage-point gap as reserve, deficit or on pace. The exact pace meaning and difference SHALL remain accessible without requiring a long visible pace sentence. This comparison SHALL NOT imply future depletion time or a token quota. The panel SHALL NOT invent a session window. Available reset passes SHALL remain distinct, with expiry accessible. Missing or stale data SHALL remain unknown; passing a scheduled reset SHALL NOT fabricate a refreshed balance.

#### Scenario: Supported snapshot
- **WHEN** the current account source reports a valid weekly percentage and future reset in its seven-day window
- **THEN** the card uses its full width for remaining share and a time-remaining marker, with accessible pace meaning, reset timing, passes and sample age.

#### Scenario: Incomplete weekly snapshot
- **WHEN** the balance is current but reset timing is absent or outside its window
- **THEN** the reported balance remains visible and pace is unknown without a fabricated session limit or depletion estimate.

#### Scenario: Exhausted allowance
- **WHEN** the source reports zero weekly balance
- **THEN** zero remains distinct from unknown and the panel invents no pass or depletion time.

#### Scenario: Missing or expired data
- **WHEN** the source fails, the sample expires or a scheduled reset passes
- **THEN** balance and pace become unknown without a fabricated refill or pass.

### Requirement: Explicit allowance sharing
Account allowance sharing SHALL be separately configured from project disclosure and limited to selected labelled numeric account summaries over existing authenticated transport. Work project exclusions SHALL remain unchanged, including when both account allowances are intentionally shown on the office display.

#### Scenario: Office allowance display
- **WHEN** allowance sharing is enabled for Personal and Work accounts
- **THEN** both permitted summaries can appear on the office display without publishing Personal agents, project names or transcripts.

### Requirement: Account token activity
The service SHALL read supported ChatGPT-backed account token-activity summaries and daily buckets through its authenticated, read-only account source, bind them to explicit account mapping and keep them separate from local session cache usage. It SHALL export only bounded numeric totals, valid bucket dates, account labels and sample times. The panel MAY show a full-panel-width bounded daily-token histogram and sum for reported dates, with its observed-day count and account scope explicit. Missing dates SHALL NOT appear as zero or contribute to the sum. Activity SHALL NOT be converted into a remaining quota, cost or depletion forecast. Missing, unsupported, stale or malformed account usage SHALL remain unavailable without hiding a valid weekly allowance. Existing explicit sharing boundaries SHALL apply.

#### Scenario: Supported account activity
- **WHEN** a mapped account returns valid daily token buckets
- **THEN** the panel spans the available width with only reported dates, their numeric sum and observed-day count, separately from weekly allowance pace.

#### Scenario: Partial or unsupported response
- **WHEN** activity buckets are absent or malformed
- **THEN** activity remains unavailable while a valid weekly balance remains visible.

#### Scenario: Missing dates
- **WHEN** reported dates have gaps
- **THEN** no omitted date is rendered or added as zero.

#### Scenario: Expired or inconsistent weekly window
- **WHEN** the allowance is stale or its reset window is invalid
- **THEN** pace is unavailable without inventing a token balance.

#### Scenario: Office sharing
- **WHEN** sharing is enabled for both explicitly mapped accounts
- **THEN** the office display receives only bounded account summaries and no personal agent or raw account identity data.

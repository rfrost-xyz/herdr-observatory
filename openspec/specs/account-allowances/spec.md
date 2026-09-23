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
The footer SHALL persistently show Personal and Work panels with plan type, weekly percentage remaining, relative scheduled reset, available usage reset pass count and relative earliest expiry when reported. Missing data SHALL remain unknown, stale observations SHALL cease appearing current, and passing a scheduled reset SHALL NOT fabricate a refreshed balance. Reset passes SHALL be distinguished from the recurring weekly reset and purchased usage credits.

#### Scenario: Supported snapshot
- **WHEN** the current account source reports weekly allowance and reset passes
- **THEN** the panel shows independently labelled weekly remaining, reset time, pass count and next expiry with sample freshness.

#### Scenario: Missing or expired data
- **WHEN** fields are absent, the source fails or its freshness deadline passes
- **THEN** unavailable values remain explicit and the display does not invent zero passes, a refilled allowance or a new expiry.

### Requirement: Explicit allowance sharing
Account allowance sharing SHALL be separately configured from project disclosure and limited to selected labelled numeric account summaries over existing authenticated transport. Work project exclusions SHALL remain unchanged, including when both account allowances are intentionally shown on the office display.

#### Scenario: Office allowance display
- **WHEN** allowance sharing is enabled for Personal and Work accounts
- **THEN** both permitted summaries can appear on the office display without publishing Personal agents, project names or transcripts.

### Requirement: Account token activity
The service SHALL read supported ChatGPT-backed account token-activity summaries and daily buckets through its authenticated, read-only account source, bind them to the same explicit account mapping as allowances and show them separately from local session cache usage. It SHALL export only bounded numeric totals, valid bucket dates, account labels and sample times. The account panel SHALL show the weekly allowance percentage, an even-use allowance rate and the average allowance burn so far, both in percentage points per day. It SHALL derive rates only from a fresh allowance and a future reset within its seven-day window. It SHALL NOT convert token activity into an estimated token quota or infer zero usage on omitted daily bucket dates. Missing, unsupported, stale or malformed account usage SHALL remain unavailable without hiding valid allowance values or fabricating zero activity. Account activity SHALL follow the existing explicit sharing boundary.

#### Scenario: Supported account activity
- **WHEN** a mapped account returns a fresh weekly allowance with a future reset and at least one hour elapsed in its seven-day window
- **THEN** its panel shows weekly percentage left, an even-use allowance rate and average allowance burn so far with separate labels and source freshness.

#### Scenario: Partial or unsupported response
- **WHEN** usage buckets are absent, incomplete, malformed or rejected by the account service
- **THEN** token activity remains unavailable while rates derived from a valid weekly allowance continue to render.

#### Scenario: Expired or inconsistent weekly window
- **WHEN** the allowance is stale, the scheduled reset has passed or its timestamp falls outside the seven-day window
- **THEN** both derived allowance rates remain unavailable without inventing a reset or a token balance.

#### Scenario: Office sharing
- **WHEN** sharing is enabled for both explicitly mapped accounts
- **THEN** the office display receives only the bounded account activity summaries and no personal agent or account identity data.

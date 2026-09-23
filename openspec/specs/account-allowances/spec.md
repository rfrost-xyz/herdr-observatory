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
The footer SHALL persistently show Personal and Work panels. Each panel SHALL lead with the weekly percentage remaining, a visual remaining-share bar and a conditional runway estimate derived from that remaining percentage and observed average allowance burn. The estimate SHALL identify its average-burn assumption and SHALL remain unknown when the source or window is invalid. Burn and Room bars SHALL share one scale within that account. Their comparison SHALL remain available through colour and accessible text. The scheduled reset SHALL remain visible. Available reset passes SHALL be compact supporting information, with expiry and sample age accessible. Missing data SHALL remain unknown, stale observations SHALL cease appearing current, and passing a scheduled reset SHALL NOT fabricate a refreshed balance. Reset passes SHALL remain distinct from the recurring weekly reset and purchased usage credits.

#### Scenario: Supported snapshot
- **WHEN** the current account source reports a valid weekly allowance and reset window after at least one hour of use
- **THEN** the panel shows the reported percentage and share bar, estimated runway at the observed average burn, Burn and Room on a shared scale, their comparison and reset timing.

#### Scenario: Exhausted allowance
- **WHEN** the current account source reports zero weekly percentage remaining within a valid reset window
- **THEN** the panel shows zero remaining runway without inventing a token balance.

#### Scenario: Missing or expired data
- **WHEN** fields are absent, the source fails or its freshness deadline passes
- **THEN** unavailable values remain explicit and the display does not invent zero passes, a refilled allowance, a runway or a new expiry.

### Requirement: Explicit allowance sharing
Account allowance sharing SHALL be separately configured from project disclosure and limited to selected labelled numeric account summaries over existing authenticated transport. Work project exclusions SHALL remain unchanged, including when both account allowances are intentionally shown on the office display.

#### Scenario: Office allowance display
- **WHEN** allowance sharing is enabled for Personal and Work accounts
- **THEN** both permitted summaries can appear on the office display without publishing Personal agents, project names or transcripts.

### Requirement: Account token activity
The service SHALL read supported ChatGPT-backed account token-activity summaries and daily buckets through its authenticated, read-only account source, bind them to the same explicit account mapping as allowances and keep them separate from local session cache usage. It SHALL export only bounded numeric totals, valid bucket dates, account labels and sample times. The account panel SHALL show the weekly allowance percentage, an even-use allowance rate and average allowance burn so far, both in percentage points per day. It SHALL derive rates only from a fresh allowance and future reset within its seven-day window. The cards SHALL NOT present daily token totals or sparklines as an implied remaining token budget, convert token activity into an estimated token quota, or infer zero usage on omitted daily bucket dates. Missing, unsupported, stale or malformed account usage SHALL remain unavailable without hiding valid allowance values or fabricating zero activity. Account activity SHALL follow the existing explicit sharing boundary.

#### Scenario: Supported account activity
- **WHEN** a mapped account returns a fresh weekly allowance with a future reset and at least one hour elapsed in its seven-day window, plus valid daily token buckets
- **THEN** its card shows the weekly percentage and derived allowance rates, while bounded token activity remains in the permitted private feed.

#### Scenario: Partial or unsupported response
- **WHEN** usage buckets are absent, incomplete, malformed or rejected by the account service
- **THEN** token activity remains unavailable while rates derived from a valid weekly allowance continue to render.

#### Scenario: Expired or inconsistent weekly window
- **WHEN** the allowance is stale, the scheduled reset has passed or its timestamp falls outside the seven-day window
- **THEN** both derived allowance rates remain unavailable without inventing a reset or a token balance.

#### Scenario: Office sharing
- **WHEN** sharing is enabled for both explicitly mapped accounts
- **THEN** the office display receives only the bounded account activity summaries and no personal agent or account identity data.

# Spec Delta

## MODIFIED Requirements

### Requirement: Honest allowance instruments
The footer SHALL persistently show Personal and Work panels. Each panel SHALL lead with the weekly percentage remaining and a visual remaining-share bar, followed by Burn and Room bars on one scale within that account. A cue SHALL identify whether observed burn currently exceeds even-use room. The scheduled reset SHALL remain visible. Available reset passes SHALL be compact supporting information, with expiry and sample age accessible. Missing data SHALL remain unknown, stale observations SHALL cease appearing current, and passing a scheduled reset SHALL NOT fabricate a refreshed balance. Reset passes SHALL remain distinct from the recurring weekly reset and purchased usage credits.

#### Scenario: Supported snapshot
- **WHEN** the current account source reports a valid weekly allowance and reset window
- **THEN** the panel shows the reported percentage as a prominent number and full-width bar, Burn and Room on a shared scale, the comparison cue and reset timing.

#### Scenario: Missing or expired data
- **WHEN** fields are absent, the source fails or its freshness deadline passes
- **THEN** unavailable values remain explicit and the display does not invent zero passes, a refilled allowance or a new expiry.

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

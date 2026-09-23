# Spec delta

## MODIFIED Requirements

### Requirement: Honest allowance instruments
The footer SHALL persistently show Personal and Work panels. Each panel SHALL lead with the reported weekly percentage remaining and a visual remaining-share bar. The scheduled reset SHALL remain visible. Available reset passes SHALL be compact supporting information, with expiry and sample age accessible. The panels SHALL NOT present a projected depletion time, average burn or even-use room as an account limit or forecast. Missing data SHALL remain unknown, stale observations SHALL cease appearing current, and passing a scheduled reset SHALL NOT fabricate a refreshed balance. Reset passes SHALL remain distinct from the recurring weekly reset and purchased usage credits.

#### Scenario: Supported snapshot
- **WHEN** the current account source reports a valid weekly allowance and reset time
- **THEN** the panel shows the reported percentage, share bar, reset timing, observation age and available passes without a depletion prediction or pace bars.

#### Scenario: Exhausted allowance
- **WHEN** the current account source reports zero weekly percentage remaining
- **THEN** the panel shows zero remaining without inventing a token balance or projected time.

#### Scenario: Missing or expired data
- **WHEN** fields are absent, the source fails or its freshness deadline passes
- **THEN** unavailable values remain explicit and the display does not invent zero passes, a refilled allowance or a new expiry.

### Requirement: Account token activity
The service SHALL read supported ChatGPT-backed account token-activity summaries and daily buckets through its authenticated, read-only account source, bind them to the same explicit account mapping as allowances and keep them separate from local session cache usage. It SHALL export only bounded numeric totals, valid bucket dates, account labels and sample times. The account panel SHALL show the weekly allowance percentage without presenting daily token totals, token sparklines, derived allowance rates or a projected token quota. It SHALL NOT infer zero usage on omitted daily bucket dates. Missing, unsupported, stale or malformed account usage SHALL remain unavailable without hiding valid allowance values or fabricating zero activity. Account activity SHALL follow the existing explicit sharing boundary.

#### Scenario: Supported account activity
- **WHEN** a mapped account returns a fresh weekly allowance and valid daily token buckets
- **THEN** its card shows the weekly percentage while bounded token activity remains in the permitted private feed.

#### Scenario: Partial or unsupported response
- **WHEN** usage buckets are absent, incomplete, malformed or rejected by the account service
- **THEN** token activity remains unavailable while a valid weekly allowance continues to render.

#### Scenario: Expired or inconsistent weekly window
- **WHEN** the allowance is stale or the scheduled reset has passed
- **THEN** the allowance is unavailable without inventing a reset or a token balance.

#### Scenario: Office sharing
- **WHEN** sharing is enabled for both explicitly mapped accounts
- **THEN** the office display receives only the bounded account activity summaries and no personal agent or account identity data.

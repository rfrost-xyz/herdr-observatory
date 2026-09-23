# Spec Delta

## MODIFIED Requirements

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

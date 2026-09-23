# Spec Delta

## ADDED Requirements

### Requirement: Account token activity
The service SHALL read supported ChatGPT-backed account token-activity summaries and daily buckets through its authenticated, read-only account source, bind them to the same explicit account mapping as allowances and show them separately from local session cache usage. It SHALL export only bounded numeric totals, valid bucket dates, account labels and sample times. Missing, unsupported, stale or malformed account usage SHALL remain unavailable without hiding valid allowance values or fabricating zero activity. Account activity SHALL follow the existing explicit sharing boundary.

#### Scenario: Supported account activity
- **WHEN** a mapped account returns a valid lifetime total, peak daily total and daily token buckets
- **THEN** its panel shows those values as subscription token activity with source freshness and no cached/uncached claim.

#### Scenario: Partial or unsupported response
- **WHEN** usage fields are absent, malformed or rejected by the account service
- **THEN** those fields remain unavailable while independent allowance fields continue to render.

#### Scenario: Office sharing
- **WHEN** sharing is enabled for both explicitly mapped accounts
- **THEN** the office display receives only the bounded account activity summaries and no personal agent or account identity data.

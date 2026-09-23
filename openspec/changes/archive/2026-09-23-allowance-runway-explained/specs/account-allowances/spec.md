# Spec Delta

## MODIFIED Requirements

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

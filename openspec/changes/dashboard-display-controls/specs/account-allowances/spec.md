# Spec Delta

## MODIFIED Requirements

### Requirement: Honest allowance instruments
The footer SHALL persistently show Personal and Work panels. Each SHALL lead with the reported weekly percentage remaining without a redundant visible weekly-left label, a full-panel-width remaining-share bar, scheduled reset and sample age. When a fresh future reset falls inside the seven-day window, the same bar SHALL mark time remaining and visually distinguish the percentage-point gap as reserve, deficit or on pace. A smaller signed percentage-point variance from even weekly pace SHALL appear beside the percentage when both values are valid, green ahead and red behind. Unknown pace SHALL show no signed variance. The exact pace meaning and difference SHALL remain accessible without requiring a long visible pace sentence. This comparison SHALL NOT imply future depletion time or a token quota. The panel SHALL NOT invent a session window. Available reset passes SHALL remain distinct, with expiry accessible. Missing or stale data SHALL remain unknown; passing a scheduled reset SHALL NOT fabricate a refreshed balance.

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

#### Scenario: Signed pace variance
- **WHEN** remaining allowance and time to reset are valid
- **THEN** the card shows a signed variance beside the remaining percentage with the correct colour and accessible explanation; unknown pace shows none.

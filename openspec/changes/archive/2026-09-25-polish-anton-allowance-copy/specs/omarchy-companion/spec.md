## MODIFIED Requirements

### Requirement: Truthful current state
A failed or expired host sample SHALL be unavailable rather than contributing zero threads. Codex allowance SHALL show only mapped accounts and valid source-dated weekly values. Remaining allowance SHALL use a solid theme-accent bar of six logical pixels and a normal foreground percentage. A positive surplus SHALL use a separate two-logical-pixel strip below the balance spanning expected to actual remaining allowance. A deficit SHALL use a static diagonal hatch in the main bar accent spanning actual to expected remaining allowance at the main track height, visually distinct from solid remaining balance. A neutral tick SHALL mark expected remaining allowance. A smaller caption-sized signed percentage SHALL show remaining minus expected allowance to one decimal place. The concise single-line tooltip SHALL compare left and expected values using %, without repeating the signed value or spelling out percentage points. Its sign and threshold colour SHALL use the same rounded value, suppressing negative zero. Pace SHALL use the percentage-point difference on the allowance scale: green at or above expected, amber for deficits up to and including five points, stronger amber above five and below ten points, and red at ten points or more. Missing pacing data SHALL NOT produce a coloured strip. Valid session counters SHALL remain visible while their live thread is reported, with older values softened and their original age shown, rather than blanked after two minutes. Missing and disconnected data SHALL stay explicit. All mapped accounts SHALL show days/hours until reset and valid reset-pass counts. The concise hover SHALL compare remaining and expected balance directly; visible pace SHALL avoid an unbounded relative ratio near reset.

#### Scenario: One host stops reporting
- **WHEN** a previously reporting host becomes unavailable
- **THEN** its threads disappear, its active-thread count becomes unavailable and reporting remains labelled as partial.

#### Scenario: Pacing thresholds and independent balance
- **WHEN** expected allowance is 70% and remaining allowance is 70%, 66%, 65%, 64% or 60%
- **THEN** pace colours are green, amber, amber, stronger amber and red respectively, while the theme-accent balance fill and foreground percentage remain independent of pace.

#### Scenario: Deficit hatch and signed pace
- **WHEN** remaining allowance is 66% against 70% expected
- **THEN** the main fill ends at 66%, a same-accent hatch spans 66% to 70%, and an amber −4.0% reading appears beside the remaining percentage without particles or glow.

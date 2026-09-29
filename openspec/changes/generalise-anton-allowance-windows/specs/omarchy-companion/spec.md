# Spec Delta

## MODIFIED Requirements

### Requirement: Truthful current state
A failed or expired host sample SHALL be unavailable rather than contributing zero threads. Allowances SHALL show only mapped accounts and valid source-dated values of each account's designated pacing window. Remaining allowance SHALL be 100 minus the pacing window's used percentage, and expected remaining allowance SHALL be the time until that window resets as a percentage of its source-supplied duration; the popover SHALL NOT assume a window duration or a provider. Remaining allowance SHALL use a solid theme-accent bar of six logical pixels and a normal foreground percentage. A positive surplus SHALL use a separate two-logical-pixel strip below the balance spanning expected to actual remaining allowance. A deficit SHALL use a static diagonal hatch in the main bar accent spanning actual to expected remaining allowance at the main track height, visually distinct from solid remaining balance. A neutral tick SHALL mark expected remaining allowance. A smaller caption-sized signed percentage SHALL show remaining minus expected allowance to one decimal place. The concise single-line tooltip SHALL compare left and expected values using %, without repeating the signed value or spelling out percentage points. Its sign and threshold colour SHALL use the same rounded value, suppressing negative zero. Pace SHALL use the percentage-point difference on the allowance scale: green at or above expected, amber for deficits up to and including five points, stronger amber above five and below ten points, and red at ten points or more. Missing pacing data SHALL NOT produce a coloured strip. Valid session counters SHALL remain visible while their live thread is reported, with older values softened and their original age shown, rather than blanked after two minutes. Missing and disconnected data SHALL stay explicit. All mapped accounts SHALL show days/hours until their pacing window resets, in the same format for windows of any length, and valid reset-pass counts. The concise hover SHALL compare remaining and expected balance directly; visible pace SHALL avoid an unbounded relative ratio near reset.

#### Scenario: One host stops reporting
- **WHEN** a previously reporting host becomes unavailable
- **THEN** its threads disappear, its active-thread count becomes unavailable and reporting remains labelled as partial.

#### Scenario: Pacing thresholds and independent balance
- **WHEN** expected allowance is 70% and remaining allowance is 70%, 66%, 65%, 64% or 60%
- **THEN** pace colours are green, amber, amber, stronger amber and red respectively, while the theme-accent balance fill and foreground percentage remain independent of pace.

#### Scenario: Deficit hatch and signed pace
- **WHEN** remaining allowance is 66% against 70% expected
- **THEN** the main fill ends at 66%, a same-accent hatch spans 66% to 70%, and an amber −4.0% reading appears beside the remaining percentage without particles or glow.

#### Scenario: Long allowance window
- **WHEN** an available account's pacing window has a 30-day duration, 25% used and 15 days until reset
- **THEN** the balance shows 75%, expected remaining allowance is 50%, the signed pace reads +25.0% and the reset caption reads `15d 0h`, without any provider-specific presentation.

#### Scenario: Codex presentation preserved
- **WHEN** the existing Codex popover fixtures are rendered from provider-neutral rows carrying the same values
- **THEN** the rendered popover is byte-identical to the reference screenshots recorded before this change.


### Requirement: Provider and account collections
The native projection SHALL use stable provider/account identities for configured allowance rows and collapse preferences. Existing label-only mappings and private identity data SHALL remain compatible. Account expansion SHALL NOT broaden Work project disclosure or publish account email. Unsupported providers SHALL remain unavailable until an explicit adapter exists.

The popover SHALL project every allowance row with the same rules, whatever its provider: it SHALL NOT branch on a provider name, default a missing provider or duration, or synthesise a provider label other than the row's own provider id, which is the only fallback when the row's provider label is missing, empty, longer than 40 characters or contains a control character. A row without a valid provider, account identity, or a known status SHALL be treated as unavailable or omitted, never given invented values. Balance and pace SHALL come only from a row that is available, fresh and has exactly one valid pacing window; zero or several pacing windows SHALL leave balance and pace unknown. Window kinds the popover does not recognise SHALL be accepted and rendered through the same pacing-window rules. An unavailable or auth_needed row SHALL show its source-supplied status text in its hover and accessible description, or "Allowance unavailable" when none is supplied, and SHALL NOT show a balance, pace or expected-allowance marker. The popover SHALL NOT add a row, provider group or status text for a provider or account the snapshot does not contain. Saved concealment aliases for existing Codex Personal and Work accounts SHALL keep applying.

#### Scenario: More than two mapped accounts
- **WHEN** native configuration contains three valid mapped accounts
- **THEN** all appear under their provider with independent readings, shared concealment and no two-row limit.

#### Scenario: Another provider with a monthly window
- **WHEN** the snapshot contains a synthetic non-Codex provider row with a 30-day pacing window alongside Codex rows
- **THEN** it appears as its own provider group under its source-supplied provider label and renders through the same allowance card, with no provider-specific presentation code.

#### Scenario: Account needs authentication
- **WHEN** a row has status auth_needed and status text "Sign in required"
- **THEN** its card shows no balance or pace, and its hover and accessible description read "Sign in required".

#### Scenario: Unavailable without source text
- **WHEN** a row has status unavailable and null status text
- **THEN** its hover reads "Allowance unavailable".

#### Scenario: Absent provider
- **WHEN** the snapshot contains rows for only one provider
- **THEN** the popover shows only that provider's group and no placeholder row for any other provider.

#### Scenario: Ambiguous pacing window
- **WHEN** an available row has no window marked as pacing, or more than one
- **THEN** its balance, pace and expected-allowance marker are unknown, and the list order of its windows is not used to choose one.

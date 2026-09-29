## ADDED Requirements

### Requirement: Provider-owned Notion allowance observation
The plugin SHALL show explicitly mapped Notion monthly AI usage and its actual
reset date only from a verified read-only account source. Provider authentication
SHALL remain outside the popover and under the provider's ownership. The popover
SHALL NOT require a browser-extension setup flow or expose login controls.
Unavailable allowance data SHALL NOT become a zero, guessed reset or fabricated
account observation. Purchased credits SHALL remain outside this feature.

#### Scenario: Authenticated monthly source
- **WHEN** an already authenticated provider source reports a verified account's monthly allowance and reset
- **THEN** the popover shows its actual usage and reset without requiring a second authentication system.

#### Scenario: Source lacks allowance support
- **WHEN** authentication succeeds but the provider does not expose allowance data
- **THEN** the integration remains unavailable and does not substitute browser setup or invented readings.

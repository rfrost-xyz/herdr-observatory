# Office display delta

## ADDED Requirements

### Requirement: Living operations console
The display SHALL animate the surfaces and perimeters of working machine and agent cards with distinct persistent phases, replacing literal heartbeat strips. It SHALL provide a bounded terminal-style presentation of actual sampled state, labelled as derived observations rather than raw command output. It SHALL retain fixed viewport bounds, escaping and reduced-motion support.

#### Scenario: Active operations
- **WHEN** permitted agents are working
- **THEN** their cards animate as a whole while readable task and technical data remain stable; console entries retain real capture timestamps.

#### Scenario: No live activity
- **WHEN** sources become unavailable or reduced motion is selected
- **THEN** active FX stop without fabricating work or new console observations.

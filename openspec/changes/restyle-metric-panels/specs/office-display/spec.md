## MODIFIED Requirements

### Requirement: Readable fleet and theme integration
Fleet information SHALL use clear labels for processor, memory, graphics, storage, network traffic and sample age, retaining host identity and Online/Offline state. Processor, memory, graphics and storage SHALL have bounded percentage gauges; unknown values SHALL be visually distinct from zero. Thread identity SHALL label its harness, native pane ID and host. The synchronised OS theme SHALL colour the entire display, including surfaces, borders and status accents, with readable light and dark presentations and no visible theme-name label.

#### Scenario: Missing fleet metrics
- **WHEN** a source or metric is unavailable
- **THEN** its label remains understandable and the value is explicitly unavailable, never inferred as zero.

#### Scenario: Theme update
- **WHEN** a new valid OS palette arrives
- **THEN** cards, header, fleet and activity effects adopt it without modifying OS configuration or adding a service.

#### Scenario: Hook-driven card detail
- **WHEN** fresh supported hook telemetry arrives
- **THEN** the card shows the available phase, tool, model and usage/context information, briefly reacts once, and preserves Herdr's lifecycle state as authority.

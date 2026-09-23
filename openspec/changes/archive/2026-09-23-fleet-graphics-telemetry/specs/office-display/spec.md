# Spec Delta

## MODIFIED Requirements

### Requirement: Reproducible supervised deployment
Both profiles SHALL run from versioned application images under Compose with health checks, bounded logs and restart policies, independently of development checkouts. Runtime mounts SHALL be limited to explicit configuration, Herdr integration, optional music socket, palette, feed and opt-in aggregate graphics sources; the application SHALL NOT mount the Docker socket, host process filesystem or an entire home directory.

#### Scenario: Restart and update
- **WHEN** the dashboard container or Docker engine restarts
- **THEN** the configured profile resumes without an interactive development session, and a documented previous image can be used for rollback.

#### Scenario: Optional graphics integration
- **WHEN** an operator enables a host-specific graphics source
- **THEN** the dashboard retains its user, capabilities and network exposure; any privileged GPU monitor has no host process or home mount and publishes only an aggregate.

### Requirement: Readable fleet and theme integration
Fleet information SHALL use clear labels for processor, memory, graphics, storage, network traffic and sample age, retaining host identity and Online/Offline state. Processor, memory, graphics and storage SHALL have bounded percentage gauges; unknown values SHALL be visually distinct from zero. A measured graphics percentage SHALL explain its source and scope. Thread identity SHALL label its harness, native pane ID and host. The synchronised OS theme SHALL colour the entire display, including surfaces, borders and status accents, with readable light and dark presentations and no visible theme-name label.

#### Scenario: Missing fleet metrics
- **WHEN** a source or metric is unavailable
- **THEN** its label remains understandable and the value is explicitly unavailable, never inferred as zero.

#### Scenario: Theme update
- **WHEN** a new valid OS palette arrives
- **THEN** cards, header, fleet and activity effects adopt it without modifying OS configuration or adding a service.

#### Scenario: Hook-driven card detail
- **WHEN** fresh supported hook telemetry arrives
- **THEN** the card shows the available phase, tool, model and usage/context information, briefly reacts once, and preserves Herdr's lifecycle state as authority.

#### Scenario: Measured graphics scope
- **WHEN** the graphics gauge has a valid reading
- **THEN** its accessible description distinguishes visible NVIDIA device utilisation from the busiest Intel Xe device engine.

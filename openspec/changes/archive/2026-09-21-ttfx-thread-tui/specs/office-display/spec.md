## MODIFIED Requirements

### Requirement: Reactive rendered terminal
The display SHALL render a fixed character-cell TUI with clearly readable thread states and a separate CLI feed using the pinned ttfx text-effects library. It SHALL NOT render decorative circles or background geometry. Only current profile-filtered source observations SHALL reach the renderer. Frame generation and playback SHALL be bounded, deduplicated by source capture, and fall back to plain text on errors. Reduced motion and pause SHALL keep plain observations readable while current thread state continues updating.

#### Scenario: Agent milestone
- **WHEN** an agent appears, changes status or disappears from a fresh source
- **THEN** the TUI updates the current state and records the observed change once, without claiming unsampled tool execution.

#### Scenario: Source unavailable
- **WHEN** a source or browser connection is lost
- **THEN** the terminal records loss once, removes stale agents from the live listing and does not report them as completed.

#### Scenario: Accessible one-screen rendering
- **WHEN** the scene runs at 720p or 1080p, or reduced motion is requested
- **THEN** the terminal remains bounded to one screen with an accessible text equivalent and reduced motion suppresses dynamic effects.

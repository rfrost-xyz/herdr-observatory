## MODIFIED Requirements

### Requirement: Reactive rendered terminal
The display SHALL show an unbranded live thread TUI between whole-terminal text effects. Effects SHALL use current theme colours, complete naturally without replacement by routine updates, and never repeat consecutively. A ten-second default hold SHALL begin after completion and be adjustable with Left/Right in one-second increments. Incomplete generation SHALL fall back to the live TUI. Profile filtering, accessibility and reduced-motion support SHALL remain.

#### Scenario: Agent milestone
- **WHEN** an agent appears, changes status or disappears from a fresh source
- **THEN** the TUI updates the current state and records the observed change once, without claiming unsampled tool execution.

#### Scenario: Source unavailable
- **WHEN** a source or browser connection is lost
- **THEN** the terminal records loss once, removes stale agents from the live listing and does not report them as completed.

#### Scenario: Accessible one-screen rendering
- **WHEN** the scene runs at 720p or 1080p, or reduced motion is requested
- **THEN** the terminal remains bounded to one screen with an accessible text equivalent and reduced motion suppresses dynamic effects.

#### Scenario: Complete playback and hold
- **WHEN** fresh data arrives during an effect
- **THEN** playback continues through the final frame, followed by the configured hold before a different effect begins.

#### Scenario: Theme and controls
- **WHEN** the user presses Left or Right
- **THEN** the hold changes by one second and the unbranded terminal retains theme-derived coloured effects across its full contents.

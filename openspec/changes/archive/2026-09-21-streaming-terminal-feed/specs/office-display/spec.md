## MODIFIED Requirements

### Requirement: Reactive rendered terminal
The display SHALL be a single rendered terminal scene rather than dashboard cards or charts. Meaningful sampled Herdr agent and source transitions SHALL create deduplicated terminal observations and scene-wide reactions. Repeated browser polls of the same capture SHALL NOT create records. Each fresh source capture SHALL provide explicitly labelled sampled observations with lower-energy feed reactions, even when agent state is unchanged. These SHALL NOT imply new work milestones. The bounded feed SHALL discard queued stale or disconnected observations. Current thread state SHALL remain readable without glitch distortion while the surrounding stream ripples, tears and sheds outgoing glyphs. Initial state SHALL establish a baseline without replaying historical impacts.

#### Scenario: Agent milestone
- **WHEN** an agent appears, changes status or disappears from a fresh source
- **THEN** the terminal records the observed change and the rendered scene reacts once, without claiming unsampled tool execution.

#### Scenario: Source unavailable
- **WHEN** a source or browser connection is lost
- **THEN** the terminal records loss once, removes stale agents from the live listing and does not report them as completed.

#### Scenario: Accessible one-screen rendering
- **WHEN** the scene runs at 720p or 1080p, or reduced motion is requested
- **THEN** the terminal remains bounded to one screen with an accessible text equivalent and reduced motion suppresses dynamic effects.

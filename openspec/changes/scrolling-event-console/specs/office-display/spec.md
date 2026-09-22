## ADDED Requirements

### Requirement: Scrolling observation console
The live terminal SHALL scroll timestamped, bounded observations as fresh samples and agent transitions arrive. Duplicate samples SHALL NOT create duplicate transitions. Status stamps SHALL distinguish observed state changes from raw events or tool execution. The current thread list SHALL remain separate and readable. Readiness, launch flags and revision/sequence changes SHALL use only sanitised available metadata.

#### Scenario: Fresh and duplicate observations
- **WHEN** an agent changes between idle, working, blocked or done and a duplicate snapshot follows
- **THEN** one timestamped transition is appended and the thread state updates; fresh sample rows scroll in, while duplicates add no transitions.

#### Scenario: Sparse milestone artwork
- **WHEN** a fresh done or blocked transition is observed
- **THEN** a brief themed Delta Corps Priest 1 stamp may appear without covering thread rows, with a cooldown preventing repeated large banners; ordinary observations use normal terminal text.

#### Scenario: Animation accessibility and disclosure
- **WHEN** motion is paused, reduced motion is requested, a source expires, or a whole-terminal effect is playing
- **THEN** no fabricated activity is added, effects retain their completion semantics, obsolete pending stamps are discarded, and current disclosed state remains accessible.

#### Scenario: Denser padded terminal
- **WHEN** the browser renders at 720p or 1080p
- **THEN** live text and effects share a denser grid with modest left/right padding, fit without document scrolling and retain keyboard controls after resizing.

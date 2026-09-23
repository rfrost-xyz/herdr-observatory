# Spec Delta

## MODIFIED Requirements

### Requirement: Scrolling observation console
The activity strip SHALL scroll timestamped, bounded observations only as notable agent lifecycle/status and source-availability changes are observed. Duplicate samples SHALL NOT create duplicate transitions. Status stamps SHALL distinguish observed state changes from raw events or tool execution. The current thread cards SHALL remain separate and readable. Readiness, launch flags and revision/sequence changes SHALL use only sanitised available metadata in the thread cards. The activity strip SHALL sit inside an opaque full-width footer split horizontally between observations and account allowances and excluded from the music visualiser and show a compact view of recent events while retaining up to 60 bounded records for mouse-wheel and keyboard scrolling, with semantic icons and theme-derived colours, each on one bounded line containing observation time, project, Herdr state, available thread number and pertinent status update without inventing activity. Unavailable fields SHALL be marked unavailable.

#### Scenario: Fresh and duplicate observations
- **WHEN** an agent changes between idle, working, blocked or done and a duplicate snapshot follows
- **THEN** one timestamped transition is appended and the thread state updates; routine samples and metadata counters add no log rows, while notable changes scroll in once.

#### Scenario: Sparse milestone artwork
- **WHEN** a fresh done or blocked transition is observed
- **THEN** the event stays on one line without decorative project-name artwork, and may receive an in-place text effect when eligible.

#### Scenario: Animation accessibility and disclosure
- **WHEN** reduced motion is requested, a source expires, or an activity-line effect is playing
- **THEN** no fabricated activity is added, effects stay outside thread cards and retain their completion semantics, obsolete pending effect starts are discarded, and current disclosed state remains accessible.

#### Scenario: Denser padded terminal
- **WHEN** the browser renders at 720p or 1080p
- **THEN** thread cards and the activity strip retain modest side padding, fit without document scrolling and retain keyboard controls after resizing.

#### Scenario: Browse recent history
- **WHEN** the observation footer contains more rows than fit in its compact area
- **THEN** the mouse wheel and keyboard scroll its bounded history, and incoming records follow at the bottom only while the user is already there.

#### Scenario: Expand observations
- **WHEN** the icon-only observation control is activated
- **THEN** observations fill the screen below the retained title, music and thread-state totals while fleet, cards and allowance panels are hidden; the control exposes its state and can restore the dashboard.


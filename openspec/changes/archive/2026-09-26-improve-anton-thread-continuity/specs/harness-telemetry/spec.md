## ADDED Requirements

### Requirement: Native turn wall-clock summaries
The plugin-owned native reader MAY derive a bounded per-session turn summary from saved `task_started`, `task_complete` and `turn_aborted` events with exact validated turn association. Supported Unix-second start and completion bounds SHALL define elapsed wall time, including waits inside the turn and excluding inter-turn idle gaps. Native `duration_ms` SHALL NOT replace missing timestamp bounds or silently change this time scope. Valid completed and aborted intervals SHALL contribute once to accumulated finished-turn time. Duplicate, conflicting, malformed, out-of-order or incomplete evidence SHALL NOT fabricate zeroes, double-count turns or claim complete accumulated coverage. The current or last valid interval MAY remain independently available when complete accumulated coverage is unknown. Public native output SHALL contain only allowlisted numeric timing, coverage, outcome and source-freshness fields; hashed private associations SHALL remain bounded. Native cursor retention and timing observation validation MAY tolerate at most one second of transport clock skew while preserving original timestamps. Larger future values and expired readings SHALL remain invalid. Existing usage and allowance source-time validation, hook metadata and Work publication schemas SHALL remain unchanged.

#### Scenario: Wait within a turn and gap between turns
- **WHEN** saved bounds establish two finished turns separated by idle time and one turn includes a wait
- **THEN** accumulated time sums the two wall-clock intervals once, includes the wait and excludes the idle gap.

#### Scenario: Aborted, duplicated or incomplete events
- **WHEN** a valid turn abort is repeated or older records do not establish complete coverage
- **THEN** the abort contributes at most once, and incomplete accumulated time stays unknown even if an independently valid last duration can be shown.

#### Scenario: Native-only timing and freshness
- **WHEN** a successful exact-bound native read reaches the current source end
- **THEN** its observation time may advance for current elapsed display, while checkpoint load, pipe heartbeats and Work publication do not manufacture a fresh timing reading or expand the existing Work schema.

### Requirement: Private native replay checkpoints
The plugin-owned collector MAY retain restart checkpoints locally for configured local and SSH native readers. Checkpoints SHALL be versioned, owner-only, atomically written and globally bounded to 32 sessions, 256 KiB and 24 hours. They SHALL contain only hashed associations, allowlisted numeric or lifecycle parser state, source cursor identity and original timestamps, never raw native identifiers, source paths or transcript content. Reuse SHALL validate exact session/header binding, file identity, consumed source identity, expiry, truncation and replacement before continuing. Corrupt, unsafe, incompatible or ambiguous state SHALL be discarded for bounded fresh replay. A loaded checkpoint SHALL NOT itself become a current measurement. Writes SHALL be throttled to meaningful cursor progress, with bounded periodic retention refresh and an owner-shutdown flush where possible. Guarded plugin uninstallation SHALL retire its owned checkpoint file and writer lock before runtime removal, preventing in-flight writes, delayed startup and final shutdown flush from recreating them while preserving unrelated state. The same ownership retirement SHALL prevent late collector binding-registry and already-loaded reporter or allowance-receiver state writes. Unsafe or busy retirement SHALL fail visibly within a bounded wait.

#### Scenario: Warm collector restart
- **WHEN** an owner restart finds a valid bounded checkpoint and revalidates its exact unchanged source binding
- **THEN** native replay continues from retained progress and publishes current values only after successful source validation and catch-up.

#### Scenario: Replaced, expired or malicious checkpoint
- **WHEN** a saved cursor is expired, oversized, malformed, owned by someone else, symlinked or inconsistent with its session or source
- **THEN** it is refused without exposing raw content, treating its timestamps as fresh or reusing another session's measurements.

#### Scenario: Unchanged polling and uninstall
- **WHEN** repeated successful polls advance no meaningful parser state
- **THEN** the collector does not rewrite the checkpoint on every poll, and guarded uninstallation removes the owned checkpoint when the plugin is removed.

#### Scenario: Uninstall races a checkpoint writer
- **WHEN** uninstall overlaps an in-flight checkpoint write or an old owner subsequently attempts a final flush or delayed startup
- **THEN** successful retirement leaves no owned checkpoint or writer lock, late attempts cannot recreate them, and unrelated files remain intact.

#### Scenario: Small remote clock offset
- **WHEN** a validated remote observation and cursor are at most one second ahead of local time
- **THEN** native timing and checkpoint retention accept the bounded offset without rewriting original source timestamps, while larger future offsets remain invalid and usage or allowance freshness rules stay unchanged.

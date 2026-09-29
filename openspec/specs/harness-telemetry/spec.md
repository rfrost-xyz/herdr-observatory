# Harness telemetry

## Purpose
Provide bounded harness activity metadata shared by native Herdr and the Observatory display without collecting private transcript content.

## Requirements

### Requirement: Supplementary harness reports
The installed Pi adapter SHALL report supported tool, model, phase and compaction events as expiring Herdr presentation metadata, without changing lifecycle authority, session references or sending agent input. Reports SHALL be limited to the matching native session in the configured local Herdr instance.

#### Scenario: Matching session
- **WHEN** a supported event occurs in the identified Herdr pane
- **THEN** Herdr exposes supplementary activity metadata and a concise display label while preserving its semantic state.

#### Scenario: Unavailable or replaced session
- **WHEN** the native helper or Herdr is unavailable, the adapter runs outside Herdr, or its native session no longer matches
- **THEN** reporting fails silently within a bounded time without blocking or changing the harness operation.

### Requirement: Truthful and private telemetry
The system SHALL export only allowlisted metadata, omit raw arguments, output, prompts and reasoning content, reject future or mismatched telemetry, and leave unsupported metrics unavailable. Older session-bound values SHALL retain their original source times. Context estimates SHALL be distinguished from reported usage; usage SHALL name its scope and retain its source timestamp.

#### Scenario: Supported and unsupported metrics
- **WHEN** a harness supplies supported recent usage through its extension or verified hook-time numeric enrichment
- **THEN** the display shows supported last-response counters and labelled context estimates, while absent counters remain unknown and newer activity does not renew old usage.

#### Scenario: Personal activity
- **WHEN** native metadata contains arguments, paths, transcript content or unknown fields
- **THEN** those fields are absent from the popover state and peer telemetry response.

### Requirement: Minimal adapter lifecycle
Telemetry processing and required install payloads SHALL ship with the native plugin or its marked native peer. No Python or Docker process SHALL be required. Redundant plugin Codex callbacks SHALL be removed only when proven owned; native Herdr integrations SHALL remain untouched. Installation and removal SHALL be idempotent, preserve unrelated/native integrations and require no additional persistent service or network listener.

#### Scenario: Repeat installation and removal
- **WHEN** an operator installs twice and later removes the adapters
- **THEN** there is one owned integration per harness and removal leaves unrelated hooks and extensions intact.

### Requirement: Hook-time structured usage enrichment
The native Codex reader SHALL enrich collection from supported numeric usage records in the exact local session established by Herdr, without synchronous Codex callbacks or a daemon. It SHALL verify the file belongs to the user, reject symlink traversal, restrict reads to the configured session directory, verify the session header and bound file reads and execution. Only allowlisted numeric fields and their source time SHALL leave the adapter; transcript text, arguments, file paths and child identities SHALL NOT be forwarded. Pi SHALL report supported live extension usage and seed it from the current session branch when available on reload. Missing counters SHALL remain unknown, and an older usage source SHALL not be made fresh by a newer hook.

#### Scenario: Supported Codex usage
- **WHEN** a matching bounded session file contains a supported token_count record
- **THEN** the parent-bound reporter exposes last-response input/output/cache counters, reported context window and a labelled context estimate with the record's timestamp.

#### Scenario: Unsafe or incompatible source
- **WHEN** the file escapes the session root, is a symlink, mismatches identity, lacks a supported record, reports a future time or is malformed
- **THEN** numeric enrichment is omitted while native Herdr state remains available.

#### Scenario: Pi reload
- **WHEN** Pi reloads the extension in a session whose active branch contains timestamped assistant usage
- **THEN** supported last-response usage is available without waiting for another assistant response and without reading transcript files.

### Requirement: Scoped cumulative harness metrics
Native collection and required harness adapters SHALL expose supported cumulative input/output and cached/uncached token counters independently of last-response usage, preserving their source timestamp and harness scope. Codex context percentage SHALL follow the verified installed harness calculation when its required inputs exist; Pi SHALL use its supported context API. Cache hits/misses SHALL be labelled as token quantities rather than request counts. Compaction totals SHALL be exported only with complete trustworthy coverage; truncated or incompatible sources SHALL remain unknown. Existing session identity, file bounds, numeric allowlisting, privacy and source-time validation SHALL apply to all added fields. Bound presentation metadata SHALL remain until replaced or the pane closes, with observations older than two minutes labelled last known rather than silently removed. A native Codex sample without valid new usage SHALL retain a previous valid sample only for the same bound session, preserving its original source time until a newer valid sample arrives. Reports SHALL fit the native metadata key/value limits and publish one coherent numeric sample atomically, while accepting valid previous-version samples during migration.

#### Scenario: Cumulative versus last-response usage
- **WHEN** a supported Codex record contains cumulative and last-response counters
- **THEN** both retain their scopes and the display can show cumulative totals rather than substituting the last response.

#### Scenario: Complete Pi session
- **WHEN** Pi supplies a supported complete bounded session entry collection
- **THEN** numeric assistant usage and compaction entries contribute to scoped session totals without exposing message content.

#### Scenario: Incomplete or invalid accounting
- **WHEN** bounded reads cannot establish full compaction or cumulative coverage, counters are malformed or a new hook has no recent supported usage
- **THEN** affected totals remain unknown when no prior valid sample exists, and a newer collection does not manufacture freshness or zeroes.

#### Scenario: Native report limits and migration
- **WHEN** an expanded numeric sample is reported beside retained previous-version metadata
- **THEN** the native report stays within its key/value limits and the reader selects one complete version without mixing old and new counters.

#### Scenario: Intermittent Codex usage read
- **WHEN** a native Codex collection has no supported usage after an earlier valid sample for the same bound session
- **THEN** the report retains that sample and its original usage-source time until a newer valid sample arrives or the session changes.

### Requirement: Plugin-owned native enrichment
A native desktop collector MAY follow validated session-bound Codex records on local and configured SSH sources independently of hook events. Remote reads SHALL run ephemerally over the existing authenticated transport with opaque replay cursors retained locally, using an explicitly installed plugin-owned native peer without a resident service. It SHALL retain exact session/header checks, owner and symlink restrictions, configured session roots, bounded file reads, bounded caches and original source timestamps. It SHALL emit only allowlisted numeric and lifecycle summaries and keep transcript content, raw child identity and source paths private. Native hooks SHALL avoid repeated transcript enrichment on their synchronous critical path; Codex collection SHALL NOT depend on an Observatory hook callback.

#### Scenario: Native completion without a hook
- **WHEN** a bound parent on a local or configured SSH source receives a supported native child completion while waiting
- **THEN** collector-owned enrichment updates the sanitised completion summary without requiring a subsequent parent hook.

#### Scenario: Replaced or unsafe source
- **WHEN** a pane session changes, a source is untrusted, replay is incomplete or the owner exits
- **THEN** the collector refuses cross-session enrichment, preserves unknown values and stops with its plugin owner.

### Requirement: Bounded native outcome summaries
Supported native lifecycle evidence MAY expose a coherent bounded partition of completed, running, interrupted, failed and unknown children with an original source timestamp. Counts SHALL sum to the validated total and remain within the existing metadata key/value limits. Older total/done-only reports SHALL remain compatible. Hook start/stop counts SHALL NOT establish an outcome partition. The same numeric and source-time validation SHALL apply at each peer boundary.

#### Scenario: Failed or interrupted child
- **WHEN** typed native records distinguish failure or interruption from running and completed
- **THEN** the summary retains those outcomes without counting them as successful completion or publishing raw child details.

#### Scenario: Invalid or older partition
- **WHEN** outcome counts are incomplete, exceed bounds, disagree with total/done or are absent on an older report
- **THEN** no invented outcome partition is displayed and independently valid legacy completion remains usable.

### Requirement: Native turn wall-clock summaries
The plugin-owned native reader MAY derive a bounded per-session turn summary from saved `task_started`, `task_complete` and `turn_aborted` events with exact validated turn association. Supported Unix-second start and completion bounds SHALL define elapsed wall time, including waits inside the turn and excluding inter-turn idle gaps. Native `duration_ms` SHALL NOT replace missing timestamp bounds or silently change this time scope. Valid completed and aborted intervals SHALL contribute once to accumulated finished-turn time. Duplicate, conflicting, malformed, out-of-order or incomplete evidence SHALL NOT fabricate zeroes, double-count turns or claim complete accumulated coverage. The current or last valid interval MAY remain independently available when complete accumulated coverage is unknown. Public native output SHALL contain only allowlisted numeric timing, coverage, outcome and source-freshness fields; hashed private associations SHALL remain bounded. Native cursor retention and timing observation validation MAY tolerate at most one second of transport clock skew while preserving original timestamps. Larger future values and expired readings SHALL remain invalid. Usage source-time validation and hook metadata constraints SHALL remain unchanged. Allowance validation SHALL follow its independently specified bounded transport policy.

#### Scenario: Wait within a turn and gap between turns
- **WHEN** saved bounds establish two finished turns separated by idle time and one turn includes a wait
- **THEN** accumulated time sums the two wall-clock intervals once, includes the wait and excludes the idle gap.

#### Scenario: Aborted, duplicated or incomplete events
- **WHEN** a valid turn abort is repeated or older records do not establish complete coverage
- **THEN** the abort contributes at most once, and incomplete accumulated time stays unknown even if an independently valid last duration can be shown.

#### Scenario: Native-only timing and freshness
- **WHEN** a successful exact-bound native read reaches the current source end
- **THEN** its observation time may advance for current elapsed display, while checkpoint load, pipe heartbeats do not manufacture a fresh timing reading or export private timing identity.

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
- **THEN** native timing and checkpoint retention accept the bounded offset without rewriting original source timestamps, while larger future offsets remain invalid, usage freshness stays strict and allowances follow their independently specified transport tolerance.

# Delta for harness-telemetry

## ADDED Requirements

### Requirement: Native Claude Code transcript replay
The plugin-owned native reader SHALL enrich a Claude Code pane only when Herdr reports agent `claude` with a `herdr:claude` session of kind `id` whose value is a safe identifier. It SHALL bind that id to exactly one `<projects-root>/<entry>/<id>.jsonl`, where the projects root is `$CLAUDE_CONFIG_DIR/projects` from the collector's own environment or `~/.claude/projects`. Zero, several or budget-truncated matches (a scan cut short by the shared probe deadline is a deadline skip instead, and changes neither the follower's binding nor its retained sample; without a current binding the thread's telemetry is unknown for that pass), a predecessor successor proven by a bounded scan, ownership or symlink failure, a header whose `sessionId` differs, or any later record carrying a different `sessionId` SHALL leave the thread's native telemetry unknown, for the rest of that binding in the case of a later record. Positive bindings SHALL be rediscovered at a bounded cadence. Records carrying `forkedFrom` SHALL be treated as inherited history and SHALL feed no metric. Subagent, tool-result, memory and account files SHALL NOT be read during thread replay.

Assistant usage SHALL be deduplicated by response: only an assistant record with a different `message.id` closes the open group, a later line of the open group replaces its counted contribution, and a reopened recently closed group SHALL make totals unknown. Totals SHALL be published only after replay from the header has caught up. Cumulative input SHALL include cache reads and writes, while source `input_tokens` remain uncached input. Client-generated `<synthetic>` records SHALL NOT count, missing counters SHALL make totals unknown, and context occupancy SHALL follow the provider's own last-response rule. The context window and percentage SHALL remain omitted when no compliant window source exists. Child completion SHALL come only from structured launch, resume and task-notification records with bounded hashed associations, and turn timing SHALL come from validated prompt, abort and turn-end record timestamps, with silent or ambiguous evidence leaving accumulated coverage unknown. Once a turn may still be running without evidence of its end, no interval for that turn SHALL be published as current or last until a turn-end record or abort; the previous last valid interval MAY remain.

#### Scenario: Bound Claude session
- **WHEN** Herdr reports a Claude pane whose id matches exactly one owned transcript with a matching header
- **THEN** the popover receives deduplicated cumulative and last-response usage, compactions, child outcomes and turn timing for that session with original source times and `claude-transcript` provenance, and no context percentage.

#### Scenario: Split and interleaved responses
- **WHEN** one response is written as several assistant lines sharing a message id, with tool results between them
- **THEN** its usage is counted once, using the response's final line.

#### Scenario: Ambiguous, forked or predecessor binding
- **WHEN** the id matches several files, a scan is truncated, a successor transcript names the bound id, or a fork copied the parent's records
- **THEN** no native measurement is shown for the ambiguous binding and copied history is never counted in two sessions.

#### Scenario: Unsupported record shapes
- **WHEN** an oversized, malformed or unrecognised record of a relevant type cannot be classified
- **THEN** only the dependent totals, children, compactions or turn coverage become unknown and no zero is invented.


## MODIFIED Requirements

### Requirement: Hook-time structured usage enrichment
The native Codex and Claude Code readers SHALL enrich collection from supported numeric usage records in the exact local session established by Herdr, without synchronous harness callbacks or a daemon. It SHALL verify the file belongs to the user, reject symlink traversal, restrict reads to the configured session directory, verify the session header and bound file reads and execution. Only allowlisted numeric fields and their source time SHALL leave the adapter; transcript text, arguments, file paths and child identities SHALL NOT be forwarded. Pi SHALL report supported live extension usage and seed it from the current session branch when available on reload. Missing counters SHALL remain unknown, and an older usage source SHALL not be made fresh by a newer hook.

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
Native collection and required harness adapters SHALL expose supported cumulative input/output and cached/uncached token counters independently of last-response usage, preserving their source timestamp and harness scope. Codex context percentage SHALL follow the verified installed harness calculation when its required inputs exist; Pi SHALL use its supported context API. Cache hits/misses SHALL be labelled as token quantities rather than request counts. Compaction totals SHALL be exported only with complete trustworthy coverage; truncated or incompatible sources SHALL remain unknown. Existing session identity, file bounds, numeric allowlisting, privacy and source-time validation SHALL apply to all added fields. Bound presentation metadata SHALL remain until replaced or the pane closes, with observations older than two minutes labelled last known rather than silently removed. A native Codex or Claude Code sample without valid new usage SHALL retain a previous valid sample only for the same bound session, preserving its original source time until a newer valid sample arrives. For Claude Code, the retained sample SHALL be re-emitted only for an incomplete replay of a bound, identity-checked file, locally and for peer threads, and SHALL be dropped on binding failure, replacement, ambiguity, session change or when a peer sample is rejected by revalidation. Reports SHALL fit the native metadata key/value limits and publish one coherent numeric sample atomically, while accepting valid previous-version samples during migration.

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

#### Scenario: Intermittent Claude Code replay
- **WHEN** a bound, identity-checked Claude Code replay has not caught up on a local or peer pass after an earlier valid sample for the same session
- **THEN** that sample is re-emitted with its original usage-source time, and it is dropped instead when the binding becomes ambiguous, unsafe, replaced or changed.

### Requirement: Plugin-owned native enrichment
A native desktop collector MAY follow validated session-bound Codex and Claude Code records on local and configured SSH sources independently of hook events. Remote reads SHALL run ephemerally over the existing authenticated transport with opaque replay cursors retained locally, using an explicitly installed plugin-owned native peer without a resident service. It SHALL retain exact session/header checks, owner and symlink restrictions, configured session roots, bounded file reads, bounded caches and original source timestamps. It SHALL emit only allowlisted numeric and lifecycle summaries and keep transcript content, raw child identity and source paths private. Native hooks SHALL avoid repeated transcript enrichment on their synchronous critical path; Codex and Claude Code collection SHALL NOT depend on an Observatory hook callback. Peers SHALL return Claude Code telemetry within the existing probe and envelope, so older and newer collectors and peers interoperate.

#### Scenario: Native completion without a hook
- **WHEN** a bound parent on a local or configured SSH source receives a supported native child completion while waiting
- **THEN** collector-owned enrichment updates the sanitised completion summary without requiring a subsequent parent hook.

#### Scenario: Replaced or unsafe source
- **WHEN** a pane session changes, a source is untrusted, replay is incomplete or the owner exits
- **THEN** the collector refuses cross-session enrichment, preserves unknown values and stops with its plugin owner.

### Requirement: Bounded native outcome summaries
Supported native lifecycle evidence MAY expose a coherent bounded partition of completed, running, interrupted, failed and unknown children with an original source timestamp. Counts SHALL sum to the validated total and remain within the existing metadata key/value limits. Older total/done-only reports SHALL remain compatible. Hook start/stop counts SHALL NOT establish an outcome partition. Claude Code launch, resume and task-notification records are typed native lifecycle evidence; unrecognised statuses SHALL count as unknown within the partition. The same numeric and source-time validation SHALL apply at each peer boundary.

#### Scenario: Failed or interrupted child
- **WHEN** typed native records distinguish failure or interruption from running and completed
- **THEN** the summary retains those outcomes without counting them as successful completion or publishing raw child details.

#### Scenario: Invalid or older partition
- **WHEN** outcome counts are incomplete, exceed bounds, disagree with total/done or are absent on an older report
- **THEN** no invented outcome partition is displayed and independently valid legacy completion remains usable.

### Requirement: Native turn wall-clock summaries
The plugin-owned native reader MAY derive a bounded per-session turn summary from saved Codex `task_started`, `task_complete` and `turn_aborted` events with exact validated turn association, or from Claude Code turn-trigger, abort and `turn_duration` record timestamps with validated per-record association. Claude Code input queued into a running turn SHALL join it only with queue evidence; a silent turn end without a turn-end record SHALL make accumulated coverage unknown. Supported Unix-second start and completion bounds SHALL define elapsed wall time, including waits inside the turn and excluding inter-turn idle gaps. Native `duration_ms` or `durationMs` SHALL NOT replace missing timestamp bounds or silently change this time scope. A Claude Code `durationMs` MAY only reject an interval: an interval whose saved start differs by more than two seconds from the start the record's timestamp less `durationMs` implies, or whose record lacks a valid `durationMs`, SHALL NOT be recorded or totalled and SHALL make accumulated coverage unknown. A Claude Code aborted interval SHALL make accumulated coverage unknown and MAY be published as the last interval only when its turn was dated. A Claude Code current turn SHALL be published only from the start of the session file, or after such a checked end, with no background work pending or running. Valid completed and aborted intervals SHALL contribute once to accumulated finished-turn time. Duplicate, conflicting, malformed, out-of-order or incomplete evidence SHALL NOT fabricate zeroes, double-count turns or claim complete accumulated coverage. The current or last valid interval MAY remain independently available when complete accumulated coverage is unknown. Public native output SHALL contain only allowlisted numeric timing, coverage, outcome and source-freshness fields; hashed private associations SHALL remain bounded. Native cursor retention and timing observation validation MAY tolerate at most one second of transport clock skew; the local revalidation of peer telemetry, turn timing and usage source times MAY tolerate one second of transport clock skew plus the peer's bounded Herdr snapshot timeout after its `sampled_at`, measured from the later of local now and `sampled_at` while preserving original timestamps. Larger future values and expired readings SHALL remain invalid. Usage source-time validation and hook metadata constraints SHALL remain unchanged. Allowance validation SHALL follow its independently specified bounded transport policy.

#### Scenario: Wait within a turn and gap between turns
- **WHEN** saved bounds establish two finished turns separated by idle time and one turn includes a wait
- **THEN** accumulated time sums the two wall-clock intervals once, includes the wait and excludes the idle gap.

#### Scenario: Aborted, duplicated or incomplete events
- **WHEN** a valid turn abort is repeated or older records do not establish complete coverage
- **THEN** the abort contributes at most once, and incomplete accumulated time stays unknown even if an independently valid last duration can be shown.

#### Scenario: Native-only timing and freshness
- **WHEN** a successful exact-bound native read reaches the current source end
- **THEN** its observation time may advance for current elapsed display, while checkpoint load, pipe heartbeats do not manufacture a fresh timing reading or export private timing identity.

#### Scenario: Claude Code duration disagreement
- **WHEN** a Claude Code `turn_duration` disagrees with the saved bounds by more than two seconds or lacks `durationMs`
- **THEN** no interval is recorded or totalled, the previous valid interval remains last, and accumulated coverage is unknown.

### Requirement: Private native replay checkpoints
The plugin-owned collector MAY retain restart checkpoints locally for configured local and SSH native readers. Checkpoints SHALL be versioned, owner-only, atomically written and globally bounded to 32 sessions, 256 KiB and 24 hours. They SHALL contain only hashed associations, allowlisted numeric or lifecycle parser state, source cursor identity and original timestamps, never raw native identifiers, source paths or transcript content. Claude Code rows SHALL use harness-namespaced hashed keys and keep their parser state in one required versioned block, which MAY include a bounded allowlisted model identifier; a row whose block is absent or invalid SHALL be replayed fresh. A Claude Code row that would exceed the byte bound MAY first shed its accumulated finished-turn intervals, keeping its source position and making accumulated coverage unknown, before it is dropped. Reuse SHALL validate exact session/header binding, file identity, consumed source identity, expiry, truncation and replacement before continuing. Corrupt, unsafe, incompatible or ambiguous state SHALL be discarded for bounded fresh replay. A loaded checkpoint SHALL NOT itself become a current measurement. Writes SHALL be throttled to meaningful cursor progress, with bounded periodic retention refresh and an owner-shutdown flush where possible. Guarded plugin uninstallation SHALL retire its owned checkpoint file and writer lock before runtime removal, preventing in-flight writes, delayed startup and final shutdown flush from recreating them while preserving unrelated state. The same ownership retirement SHALL prevent late collector binding-registry and already-loaded reporter or allowance-receiver state writes. Unsafe or busy retirement SHALL fail visibly within a bounded wait.

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

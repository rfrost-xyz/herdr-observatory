# Harness telemetry

## Purpose
Provide bounded harness activity metadata shared by native Herdr and the Observatory display without collecting private transcript content.

## Requirements

### Requirement: Supplementary harness reports
The installed Pi adapter SHALL report supported tool, model, phase and compaction events as expiring Herdr presentation metadata, without changing lifecycle authority, session references or sending agent input. Reports SHALL be limited to the matching native session in the configured local Herdr instance. The Claude Code mod, installed only by the local plugin and never on a peer, SHALL report only the Claude Code context window, without a display label and under the same lifecycle, session and input restrictions.

#### Scenario: Matching session
- **WHEN** a supported Pi event occurs in the identified Herdr pane
- **THEN** Herdr exposes supplementary activity metadata and a concise display label while preserving its semantic state.

#### Scenario: Matching Claude Code session
- **WHEN** the Claude Code mod reports a window for the session bound to the identified Herdr pane
- **THEN** the pane's metadata carries the window only, and the pane's display label and semantic state are unchanged.

#### Scenario: Unavailable or replaced session
- **WHEN** the native helper or Herdr is unavailable, the adapter runs outside Herdr, or its native session no longer matches
- **THEN** reporting fails silently within a bounded time without blocking or changing the harness operation.

#### Scenario: Claude Code mod outside a bound pane
- **WHEN** the Claude Code mod observes a window outside Herdr, without a pane, before Herdr binds the pane to the reported session id, or after the session changed
- **THEN** no metadata is written, the event continues unchanged, and a later event in the bound session may report again.

### Requirement: Truthful and private telemetry
The system SHALL export only allowlisted metadata, omit raw arguments, output, prompts and reasoning content, reject future or mismatched telemetry, and leave unsupported metrics unavailable. Older session-bound values SHALL retain their original source times. Context estimates SHALL be distinguished from reported usage; usage SHALL name its scope and retain its source timestamp.

#### Scenario: Supported and unsupported metrics
- **WHEN** a harness supplies supported recent usage through its extension or verified hook-time numeric enrichment
- **THEN** the display shows supported last-response counters and labelled context estimates, while absent counters remain unknown and newer activity does not renew old usage.

#### Scenario: Personal activity
- **WHEN** native metadata contains arguments, paths, transcript content or unknown fields
- **THEN** those fields are absent from the popover state and peer telemetry response.

### Requirement: Minimal adapter lifecycle
Telemetry processing and required install payloads SHALL ship with the native plugin or its marked native peer. No Python or Docker process SHALL be required. Redundant plugin Codex callbacks SHALL be removed only when proven owned; native Herdr integrations SHALL remain untouched. The Claude Code mod SHALL be the one owned Claude Code integration: its files SHALL be recorded with their hashes in the plugin's hook receipt, installed only into a target proven absent or owned and not managed by chezmoi, mise dotfiles (a history entry or a `[dotfiles]` declaration in any mode; the `[dotfiles]` declarations SHALL be read with `mise config get -f`, which renders no template-mode dotfile source, while mise loads the user's configuration, including `[env]`, as any mise command does) or a Git repository with a real repository marker (a `.git` symlink counts as one), never installed on a peer, refreshed so that an interruption at any step, including one that leaves the installer's own temporary file behind in a mod directory the installer writes files into, leaves a state that a retry or removal accepts, and removed only while every recorded file still matches a hash the installer wrote and no path component to it is a symlink. Installers that write the hook receipt SHALL be serialised, and one that cannot obtain the receipt within a bounded time SHALL refuse without writing. A removal refused for a changed file SHALL leave the plugin, its integrations and its receipt in place so that removal can be retried; a recorded mod directory SHALL be kept only when it still holds entries, and a removal that cannot remove one for any other reason SHALL fail with the receipt entry kept. Installation and removal SHALL be idempotent, preserve unrelated/native integrations and require no additional persistent service or network listener.

#### Scenario: Repeat installation and removal
- **WHEN** an operator installs twice and later removes the adapters
- **THEN** there is one owned integration per harness and removal leaves unrelated hooks and extensions intact.

#### Scenario: Interrupted mod refresh
- **WHEN** a refresh of the mod is interrupted after its receipt or some of its files were written, possibly leaving the installer's temporary file in a mod directory
- **THEN** a later install or removal succeeds without refusing the files the interrupted refresh wrote or kept, deletes the leftover temporary file only from the mod directories that receive file writes, and a removal leaves no mod directory behind.

#### Scenario: Modified or unowned Claude Code mod
- **WHEN** the mod directory exists without a receipt entry, contains an unrecorded file at install, is managed configuration, or a recorded file was changed before removal
- **THEN** the installer refuses and preserves the files and receipt, while the Pi extension and the plugin installation remain unaffected.

#### Scenario: Claude Code mod declared in mise dotfiles
- **WHEN** `mise` is on `PATH` and a `[dotfiles]` declaration in any mode (for example a copy of `~/.claude/skills`) equals, contains or is inside the mod directory, even though history tracks nothing there, or the declarations cannot be read or parsed
- **THEN** the installer refuses and changes nothing, having read the declarations with `mise config get -f`, which renders no template-mode dotfile source (mise loads the user's configuration, including `[env]`, as any mise command does), while a declaration of an unrelated sibling such as `~/.claude/skills/other` does not refuse.

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
Native collection and required harness adapters SHALL expose supported cumulative input/output and cached/uncached token counters independently of last-response usage, preserving their source timestamp and harness scope. Codex context percentage SHALL follow the verified installed harness calculation when its required inputs exist; Pi SHALL use its supported context API. Claude Code context percentage SHALL be the native replay context as a percentage of the window, rounded half up, from a bound Claude Code reporter sample for the same session, without the Codex reserve, and SHALL remain unknown when either input is unknown; a reported window smaller than the replay context SHALL leave the window and percentage unknown without discarding the replay context. Cache hits/misses SHALL be labelled as token quantities rather than request counts. Compaction totals SHALL be exported only with complete trustworthy coverage; truncated or incompatible sources SHALL remain unknown. Existing session identity, file bounds, numeric allowlisting, privacy and source-time validation SHALL apply to all added fields. Bound presentation metadata SHALL remain until replaced or the pane closes, with observations older than two minutes labelled last known rather than silently removed. A native Codex or Claude Code sample without valid new usage SHALL retain a previous valid sample only for the same bound session, preserving its original source time until a newer valid sample arrives. For Claude Code, the retained sample SHALL be re-emitted only for an incomplete replay of a bound, identity-checked file, locally and for peer threads, and SHALL be dropped on binding failure, replacement, ambiguity, session change or when a peer sample is rejected by revalidation. Reports SHALL fit the native metadata key/value limits and publish one coherent numeric sample atomically, while accepting valid previous-version samples during migration.

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

#### Scenario: Claude Code context percentage
- **WHEN** a caught-up bound Claude Code replay reports context 150,000 and the pane carries a reporter window of 200,000 bound to the same session
- **THEN** the sample shows window 200,000 and context percentage 75, a context of 1 in a window of 200 shows 1, a context equal to the window shows 100, and a window bound to another session, a smaller window than the context, or an unknown context leaves the percentage unknown rather than zero.

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
The plugin-owned native reader MAY derive a bounded per-session turn summary from saved Codex `task_started`, `task_complete` and `turn_aborted` events with exact validated turn association, or from Claude Code turn-trigger, abort and `turn_duration` record timestamps with validated per-record association. Claude Code input queued into a running turn SHALL join it only with queue evidence; a silent turn end without a turn-end record SHALL make accumulated coverage unknown. Supported Unix-second start and completion bounds SHALL define elapsed wall time, including waits inside the turn and excluding inter-turn idle gaps. Native `duration_ms` or `durationMs` SHALL NOT replace missing timestamp bounds or silently change this time scope. A Claude Code `durationMs` MAY only reject an interval: an interval whose saved start differs by more than two seconds from the start the record's timestamp less `durationMs` implies, or whose record lacks a valid `durationMs`, SHALL NOT be recorded or totalled and SHALL make accumulated coverage unknown. A Claude Code aborted interval SHALL make accumulated coverage unknown and MAY be published as the last interval only when its turn was dated. A Claude Code current turn SHALL be published only from the start of the session file, or after such a checked end reporting no background agents or workflows pending, and SHALL NOT be published after a background launch, resume, or background-starting local command that the transcript records in a structured field, until the next such end. Valid completed and aborted intervals SHALL contribute once to accumulated finished-turn time. Duplicate, conflicting, malformed, out-of-order or incomplete evidence SHALL NOT fabricate zeroes, double-count turns or claim complete accumulated coverage. The current or last valid interval MAY remain independently available when complete accumulated coverage is unknown. Public native output SHALL contain only allowlisted numeric timing, coverage, outcome and source-freshness fields; hashed private associations SHALL remain bounded. Native cursor retention and timing observation validation MAY tolerate at most one second of transport clock skew; the local revalidation of peer telemetry, turn timing and usage source times MAY tolerate one second of transport clock skew plus the peer's bounded Herdr snapshot timeout after its `sampled_at`, measured from the later of local now and `sampled_at` while preserving original timestamps. Larger future values and expired readings SHALL remain invalid. Usage source-time validation and hook metadata constraints SHALL remain unchanged. Allowance validation SHALL follow its independently specified bounded transport policy.

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

### Requirement: Native Claude Code transcript replay
The plugin-owned native reader SHALL enrich a Claude Code pane only when Herdr reports agent `claude` with a `herdr:claude` session of kind `id` whose value is a safe identifier. It SHALL bind that id to exactly one `<projects-root>/<entry>/<id>.jsonl`, where the projects root is `$CLAUDE_CONFIG_DIR/projects` from the collector's own environment or `~/.claude/projects`. Zero, several or budget-truncated matches (a scan cut short by the shared probe deadline is a deadline skip instead, and changes neither the follower's binding nor its retained sample; without a current binding the thread's telemetry is unknown for that pass), a predecessor successor proven by a bounded scan, ownership or symlink failure, a header whose `sessionId` differs, or any later record carrying a different `sessionId` SHALL leave the thread's native telemetry unknown, for the rest of that binding in the case of a later record. Positive bindings SHALL be rediscovered at a bounded cadence. Records carrying `forkedFrom` SHALL be treated as inherited history and SHALL feed no metric. Subagent, tool-result, memory and account files SHALL NOT be read during thread replay.

Assistant usage SHALL be deduplicated by response: only an assistant record with a different `message.id` closes the open group, a later line of the open group replaces its counted contribution, and a reopened recently closed group SHALL make totals unknown. Totals SHALL be published only after replay from the header has caught up. Cumulative input SHALL include cache reads and writes, while source `input_tokens` remain uncached input. Client-generated `<synthetic>` records SHALL NOT count, missing counters SHALL make totals unknown, and context occupancy SHALL follow the provider's own last-response rule. The context window SHALL come only from a bound local Claude Code reporter sample for the pane's current session, and the window and percentage SHALL remain omitted when no such sample exists. Peer Claude Code samples SHALL carry no window or percentage, and a peer SHALL collect its Claude Code samples without reading reporter metadata, so that a peer pane's report never changes the peer's output. Child completion SHALL come only from structured launch, resume and task-notification records with bounded hashed associations, and turn timing SHALL come from validated prompt, abort and turn-end record timestamps, with silent or ambiguous evidence leaving accumulated coverage unknown. Once a turn may still be running without evidence of its end, no interval for that turn SHALL be published as current or last until a turn-end record or abort; the previous last valid interval MAY remain.

#### Scenario: Bound Claude session
- **WHEN** Herdr reports a Claude pane whose id matches exactly one owned transcript with a matching header
- **THEN** the popover receives deduplicated cumulative and last-response usage, compactions, child outcomes and turn timing for that session with original source times and `claude-transcript` provenance, and a context window and percentage only when a bound local reporter sample supplies the window.

#### Scenario: Split and interleaved responses
- **WHEN** one response is written as several assistant lines sharing a message id, with tool results between them
- **THEN** its usage is counted once, using the response's final line.

#### Scenario: Ambiguous, forked or predecessor binding
- **WHEN** the id matches several files, a scan is truncated, a successor transcript names the bound id, or a fork copied the parent's records
- **THEN** no native measurement is shown for the ambiguous binding and copied history is never counted in two sessions.

#### Scenario: Unsupported record shapes
- **WHEN** an oversized, malformed or unrecognised record of a relevant type cannot be classified
- **THEN** only the dependent totals, children, compactions or turn coverage become unknown and no zero is invented.

### Requirement: Claude Code context window reporter
The plugin MAY install a Claude Code mod as a personal skills-directory plugin that reports the context window Claude Code uses for its current session. The mod SHALL observe events only, SHALL always pass each event on unchanged, SHALL never throw into or wait on the harness for the reporter process, and SHALL run the installed native reporter only inside a Herdr pane, with the pane, a strictly increasing epoch-based sequence, the Claude Code session id and a positive bounded integer window as arguments only, without setting the process-run standard input, environment or working-directory options. The mod SHALL start a reporter run only within a hook invocation, SHALL skip a sample while a recent run is in flight rather than holding it for later, and SHALL handle every rejection of the promises it creates. The mod SHALL skip a report when its clock does not read epoch milliseconds. The reporter SHALL NOT depend on its working directory and SHALL fail closed when it cannot resolve absolute home and state paths. It SHALL NOT send the model, prompts, messages, costs, rate limits or account data. The native reporter SHALL require the plugin owner, a receipt recording the mod, exactly one configured local host and a Herdr pane whose agent is `claude` with a `herdr:claude` session of kind `id` equal to the reported id, and SHALL write one bound versioned metadata report for agent `claude` carrying only the window, with no display label, usage source time or totals. An unchanged bound window SHALL NOT be rewritten. Any failure SHALL write no metadata; without an earlier bound report for the same session the window stays unknown, and a stale window never hides replay context.

#### Scenario: First turn in a bound Claude Code session
- **WHEN** a Claude Code session that loaded the mod completes a turn in a Herdr pane bound to its session id
- **THEN** the pane receives one bound window report, and the local popover shows the context window and percentage for that thread.

#### Scenario: Mods unavailable or reporter refused
- **WHEN** mods are disabled, the mod is not loaded, the runtime or its mod receipt is missing, or Herdr reports another agent or session
- **THEN** the harness continues unchanged, no metadata is written, and the thread's window and percentage stay unknown unless an earlier bound report for the same session supplied them.

#### Scenario: Remote Claude Code thread
- **WHEN** a peer returns a Claude Code thread
- **THEN** its window and percentage are unknown in the local popover, whatever metadata the peer carries, and the peer envelope is unchanged.

#### Scenario: Unsafe arguments
- **WHEN** the reporter receives an invalid pane, an unsafe session id, a sequence beyond the bound, or a zero, signed, non-decimal or oversized window
- **THEN** it exits without reading configuration, connecting to Herdr or writing metadata.

#### Scenario: Reporter without a usable environment
- **WHEN** the reporter runs with no usable home or a relative state location in its environment or arguments, from an arbitrary working directory
- **THEN** it ignores a relative environment state location, resolves the home from the user account, refuses a relative explicit state location or an unresolvable home, and never creates a file relative to its working directory.

#### Scenario: Report process that never finishes
- **WHEN** a reporter run started by the mod has not settled after a bounded time
- **THEN** events inside that time start no run and are not replayed later, a later event may start a new run, and the late result of the earlier run does not change the mod's state.

#### Scenario: Mod API failure during a report
- **WHEN** the mod's clock or process-run call throws or rejects while it starts or awaits a reporter run
- **THEN** the event continues unchanged, no unhandled rejection reaches the harness, and a later event can still report.

# Delta for harness-telemetry

## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Minimal adapter lifecycle
Telemetry processing and required install payloads SHALL ship with the native plugin or its marked native peer. No Python or Docker process SHALL be required. Redundant plugin Codex callbacks SHALL be removed only when proven owned; native Herdr integrations SHALL remain untouched. The Claude Code mod SHALL be the one owned Claude Code integration: its files SHALL be recorded with their hashes in the plugin's hook receipt, installed only into a target proven absent or owned and not managed by chezmoi, mise dotfiles or a Git repository with a real repository marker (a `.git` symlink counts as one), never installed on a peer, refreshed so that an interruption at any step, including one that leaves the installer's own temporary file behind in a mod directory, leaves a state that a retry or removal accepts, and removed only while every recorded file still matches a hash the installer wrote and no path component to it is a symlink. Installers that write the hook receipt SHALL be serialised, and one that cannot obtain the receipt within a bounded time SHALL refuse without writing. A removal refused for a changed file SHALL leave the plugin, its integrations and its receipt in place so that removal can be retried. Installation and removal SHALL be idempotent, preserve unrelated/native integrations and require no additional persistent service or network listener.

#### Scenario: Repeat installation and removal
- **WHEN** an operator installs twice and later removes the adapters
- **THEN** there is one owned integration per harness and removal leaves unrelated hooks and extensions intact.

#### Scenario: Interrupted mod refresh
- **WHEN** a refresh of the mod is interrupted after its receipt or some of its files were written, possibly leaving the installer's temporary file in a mod directory
- **THEN** a later install or removal succeeds without refusing the files the interrupted refresh wrote or kept, deletes the leftover temporary file, and a removal leaves no mod directory behind.

#### Scenario: Modified or unowned Claude Code mod
- **WHEN** the mod directory exists without a receipt entry, contains an unrecorded file at install, is managed configuration, or a recorded file was changed before removal
- **THEN** the installer refuses and preserves the files and receipt, while the Pi extension and the plugin installation remain unaffected.

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

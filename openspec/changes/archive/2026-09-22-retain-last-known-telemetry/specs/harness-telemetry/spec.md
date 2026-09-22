## MODIFIED Requirements

### Requirement: Truthful and private telemetry
The system SHALL export only allowlisted metadata, omit raw arguments, output, prompts and reasoning content, enforce Work disclosure before publishing, reject future or mismatched telemetry, and leave unsupported metrics unavailable. Older session-bound values SHALL retain their original source times. Context estimates SHALL be distinguished from reported usage; usage SHALL name its scope and retain its source timestamp.

#### Scenario: Supported and unsupported metrics
- **WHEN** a harness supplies supported recent usage through its extension or verified hook-time numeric enrichment
- **THEN** the display shows supported last-response counters and labelled context estimates, while absent counters remain unknown and newer activity does not renew old usage.

#### Scenario: Personal activity
- **WHEN** telemetry belongs to a personal or unclassified project
- **THEN** it is absent from the Work browser and Work feed, even if metadata contains unexpected fields.

### Requirement: Hook-time structured usage enrichment
The existing Codex adapter SHALL enrich hooks from supported numeric usage records in the exact local session file named by the hook, without a daemon or session-directory container mount. It SHALL verify the file belongs to the user, reject symlink traversal, restrict reads to the configured session directory, verify the session header and bound file reads and execution. Only allowlisted numeric fields and their source time SHALL leave the adapter; transcript text, arguments, file paths and child identities SHALL NOT be forwarded. Pi SHALL report supported live extension usage and seed it from the current session branch when available on reload. Missing counters SHALL remain unknown, and an older usage source SHALL not be made fresh by a newer hook.

#### Scenario: Supported Codex usage
- **WHEN** a matching bounded session file contains a supported token_count record
- **THEN** the parent-bound reporter exposes last-response input/output/cache counters, reported context window and a labelled context estimate with the record's timestamp.

#### Scenario: Unsafe or incompatible source
- **WHEN** the file escapes the session root, is a symlink, mismatches identity, lacks a supported record, reports a future time or is malformed
- **THEN** numeric enrichment is omitted while ordinary hook reporting remains best-effort and silent.

#### Scenario: Pi reload
- **WHEN** Pi reloads the extension in a session whose active branch contains timestamped assistant usage
- **THEN** supported last-response usage is available without waiting for another assistant response and without reading transcript files.

### Requirement: Scoped cumulative harness metrics
Hook adapters SHALL expose supported cumulative input/output and cached/uncached token counters independently of last-response usage, preserving their source timestamp and harness scope. Codex context percentage SHALL follow the verified installed harness calculation when its required inputs exist; Pi SHALL use its supported context API. Cache hits/misses SHALL be labelled as token quantities rather than request counts. Compaction totals SHALL be exported only with complete trustworthy coverage; truncated or incompatible sources SHALL remain unknown. Existing session identity, file bounds, numeric allowlisting, privacy and source-time validation SHALL apply to all added fields. Bound presentation metadata SHALL remain until replaced or the pane closes, with observations older than two minutes labelled last known rather than silently removed. Reports SHALL fit the native metadata key/value limits and publish one coherent numeric sample atomically, while accepting valid previous-version samples during migration.

#### Scenario: Cumulative versus last-response usage
- **WHEN** a supported Codex record contains cumulative and last-response counters
- **THEN** both retain their scopes and the display can show cumulative totals rather than substituting the last response.

#### Scenario: Complete Pi session
- **WHEN** Pi supplies a supported complete bounded session entry collection
- **THEN** numeric assistant usage and compaction entries contribute to scoped session totals without exposing message content.

#### Scenario: Incomplete or invalid accounting
- **WHEN** bounded reads cannot establish full compaction or cumulative coverage, counters are malformed or a new hook has no recent supported usage
- **THEN** affected totals remain unknown and a newer hook does not manufacture freshness or zeroes.

#### Scenario: Native report limits and migration
- **WHEN** an expanded numeric sample is reported beside retained previous-version metadata
- **THEN** the native report stays within its key/value limits and the reader selects one complete version without mixing old and new counters.

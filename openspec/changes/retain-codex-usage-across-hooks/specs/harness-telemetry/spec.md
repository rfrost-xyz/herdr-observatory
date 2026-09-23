## MODIFIED Requirements

### Requirement: Scoped cumulative harness metrics
Hook adapters SHALL expose supported cumulative input/output and cached/uncached token counters independently of last-response usage, preserving their source timestamp and harness scope. Codex context percentage SHALL follow the verified installed harness calculation when its required inputs exist; Pi SHALL use its supported context API. Cache hits/misses SHALL be labelled as token quantities rather than request counts. Compaction totals SHALL be exported only with complete trustworthy coverage; truncated or incompatible sources SHALL remain unknown. Existing session identity, file bounds, numeric allowlisting, privacy and source-time validation SHALL apply to all added fields. Bound presentation metadata SHALL remain until replaced or the pane closes, with observations older than two minutes labelled last known rather than silently removed. A Codex hook without valid usage SHALL retain a previous valid sample only for the same bound session, preserving its original source time until a newer valid sample arrives. Reports SHALL fit the native metadata key/value limits and publish one coherent numeric sample atomically, while accepting valid previous-version samples during migration.

#### Scenario: Cumulative versus last-response usage
- **WHEN** a supported Codex record contains cumulative and last-response counters
- **THEN** both retain their scopes and the display can show cumulative totals rather than substituting the last response.

#### Scenario: Complete Pi session
- **WHEN** Pi supplies a supported complete bounded session entry collection
- **THEN** numeric assistant usage and compaction entries contribute to scoped session totals without exposing message content.

#### Scenario: Incomplete or invalid accounting
- **WHEN** bounded reads cannot establish full compaction or cumulative coverage, counters are malformed or a new hook has no recent supported usage
- **THEN** affected totals remain unknown when no prior valid sample exists, and a newer hook does not manufacture freshness or zeroes.

#### Scenario: Native report limits and migration
- **WHEN** an expanded numeric sample is reported beside retained previous-version metadata
- **THEN** the native report stays within its key/value limits and the reader selects one complete version without mixing old and new counters.

#### Scenario: Intermittent Codex usage read
- **WHEN** a Codex hook has no supported usage after an earlier valid sample for the same bound session
- **THEN** the report retains that sample and its original usage-source time until a newer valid sample arrives or the session changes.

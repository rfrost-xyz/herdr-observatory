## ADDED Requirements

### Requirement: Plugin-owned native enrichment
A native desktop collector MAY follow validated session-bound Codex records on local and configured SSH sources independently of hook events. Remote reads SHALL run ephemerally over the existing authenticated transport with opaque replay cursors retained locally, without remote installation or persistent state. It SHALL retain exact session/header checks, owner and symlink restrictions, configured session roots, bounded file reads, bounded caches and original source timestamps. It SHALL emit only allowlisted numeric and lifecycle summaries and keep transcript content, raw child identity and source paths private. Native hooks SHALL avoid repeated transcript enrichment on their synchronous critical path; existing web/remote hook enrichment SHALL remain compatible.

#### Scenario: Native completion without a hook
- **WHEN** a bound parent on a local or configured SSH source receives a supported native child completion while waiting
- **THEN** collector-owned enrichment updates the sanitised completion summary without requiring a subsequent parent hook.

#### Scenario: Replaced or unsafe source
- **WHEN** a pane session changes, a source is untrusted, replay is incomplete or the owner exits
- **THEN** the collector refuses cross-session enrichment, preserves unknown values and stops with its plugin owner.

### Requirement: Bounded native outcome summaries
Supported native lifecycle evidence MAY expose a coherent bounded partition of completed, running, interrupted, failed and unknown children with an original source timestamp. Counts SHALL sum to the validated total and remain within the existing metadata key/value limits. Older total/done-only reports SHALL remain compatible. Hook start/stop counts SHALL NOT establish an outcome partition. Work revalidation SHALL apply unchanged.

#### Scenario: Failed or interrupted child
- **WHEN** typed native records distinguish failure or interruption from running and completed
- **THEN** the summary retains those outcomes without counting them as successful completion or publishing raw child details.

#### Scenario: Invalid or older partition
- **WHEN** outcome counts are incomplete, exceed bounds, disagree with total/done or are absent on an older report
- **THEN** no invented outcome partition is displayed and independently valid legacy completion remains usable.

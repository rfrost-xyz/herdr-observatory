## ADDED Requirements

### Requirement: Hook-time structured usage enrichment
The existing Codex adapter SHALL enrich hooks from supported numeric usage records in the exact local session file named by the hook, without a daemon or session-directory container mount. It SHALL verify the file belongs to the user, reject symlink traversal, restrict reads to the configured session directory, verify the session header and bound file reads and execution. Only allowlisted numeric fields and their source time SHALL leave the adapter; transcript text, arguments, file paths and child identities SHALL NOT be forwarded. Pi SHALL report supported live extension usage and seed it from the current session branch when available on reload. Missing counters SHALL remain unknown, and an older usage source SHALL not be made fresh by a newer hook.

#### Scenario: Supported Codex usage
- **WHEN** a matching bounded session file contains a recent supported token_count record
- **THEN** the parent-bound reporter exposes last-response input/output/cache counters, reported context window and a labelled context estimate with the record's timestamp.

#### Scenario: Unsafe, stale or incompatible source
- **WHEN** the file escapes the session root, is a symlink, mismatches identity, lacks a supported record, is stale or malformed
- **THEN** numeric enrichment is omitted while ordinary hook reporting remains best-effort and silent.

#### Scenario: Pi reload
- **WHEN** Pi reloads the extension in a session whose active branch contains recent assistant usage
- **THEN** supported last-response usage is available without waiting for another assistant response and without reading transcript files.

## MODIFIED Requirements

### Requirement: Truthful and private telemetry
The system SHALL export only allowlisted metadata, omit raw arguments, output, prompts and reasoning content, enforce Work disclosure before publishing, reject stale or mismatched telemetry, and leave unsupported metrics unavailable. Context estimates SHALL be distinguished from reported usage; usage SHALL name its scope and retain its source timestamp.

#### Scenario: Supported and unsupported metrics
- **WHEN** a harness supplies supported recent usage through its extension or verified hook-time numeric enrichment
- **THEN** the display shows supported last-response counters and labelled context estimates, while absent counters remain unknown and newer activity does not renew old usage.

#### Scenario: Personal activity
- **WHEN** telemetry belongs to a personal or unclassified project
- **THEN** it is absent from the Work browser and Work feed, even if metadata contains unexpected fields.

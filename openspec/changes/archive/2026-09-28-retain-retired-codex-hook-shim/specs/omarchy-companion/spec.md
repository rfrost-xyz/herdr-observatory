## ADDED Requirements

### Requirement: Safe retirement of cached Codex hooks
When native migration retires an owned Codex callback, it SHALL preserve an inert compatibility helper at the exact former command path for already-running callers. The helper SHALL consume stdin without interpreting, storing or publishing it, emit no output and exit successfully. It SHALL NOT restore hook registration, telemetry collection, Python execution, forwarding or a persistent process. The native installation SHALL record the exact helper path and payload hash in its existing ownership receipt and retain it until explicit uninstall.

#### Scenario: Existing session keeps its old command
- **WHEN** a Codex session created before migration invokes the retired command after current hook registration is removed
- **THEN** the receipt-owned helper succeeds without affecting the session, producing output or collecting data.

#### Scenario: Recover an already-migrated installation
- **WHEN** the old helper is absent and an owned backup proves its exact prior command under the current verified native installation
- **THEN** recovery creates only the inert helper and its ownership metadata, preserving current hooks, Pi ownership and running processes.

#### Scenario: Repeat migration or conflicting path
- **WHEN** repair encounters its matching receipt-owned helper, or an unknown file, changed payload, symlink or conflicting owner
- **THEN** the matching helper remains idempotent, while conflicts are preserved and rejected before unrelated integration changes.

#### Scenario: Explicit uninstall
- **WHEN** the operator uninstalls the native plugin or peer
- **THEN** only the unchanged receipt-owned compatibility helper is removed, unknown content is preserved and ownership conflicts retain enough state for a safe retry.

# Tasks

## 1. Safe compatibility ownership

- [x] 1.1 Preserve the inert helper during owned migration and support proof-based recovery of an absent helper; verify empty/large input, idempotency, receipt integrity and missing-proof/conflict/symlink failure fixtures.
- [x] 1.2 Extend receipt-bound uninstall to validate compatibility and Pi before mutation, remove only owned bytes and preserve unknown content; verify local/peer uninstall, changed-hash and retry fixtures.
- [x] 1.3 Document the cached-session lifetime and explicit recovery/removal command; verify the documented CLI against synthetic already-migrated state.

## 2. Acceptance

- [x] 2.1 Pass focused/full native lifecycle gates, format, Clippy, existing JavaScript/distribution checks and strict OpenSpec validation; obtain independent frozen-source review with all findings resolved.
- [x] 2.2 Record parent-owned live recovery, unchanged hooks/processes and reviewed installed payload hashes; synchronise canonical behaviour, archive and clean owned temporary artefacts.

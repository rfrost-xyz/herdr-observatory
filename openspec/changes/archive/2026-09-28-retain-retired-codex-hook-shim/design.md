# Design

## Context

The native installer removes exact owned Codex callbacks, backs up the former hooks JSON, installs Pi reporting and records its ownership in `.hooks-receipt.json`. Existing Codex processes retain an old shell command even when current hooks configuration no longer contains it. The former helper may already be absent.

## Goals / Non-Goals

**Goals:** quiet cached calls without restarting user processes; preserve native-only collection; make compatibility ownership and explicit removal reliable.

**Non-goals:** restoring callbacks, reading hook JSON payloads, publishing/collecting metrics, introducing a daemon or detecting when every old process has exited.

## Decisions

- Keep an inert `codex.sh` at the exact historic user-owned path. Its fixed bytes are `#!/bin/sh\n# herdr-observatory retired Codex hook v1\ncat >/dev/null 2>/dev/null || :\nexit 0\n`. It drains stdin, emits nothing and exits zero. The invoking Codex process owns EOF/deadline; no timeout dependency or persistent process is added.
- Extend the existing version-one hook receipt with `compatibility: {path, sha256}`. Path must equal the one derived from the current home and hash must match the exact bundled payload. Preserve the existing runtime/extension ownership fields. No separate unknown receipt file is introduced.
- Future migration replaces only a proven-owned old shell helper with the inert payload while removing actual callbacks and Python helpers. Already-migrated recovery requires an owned regular backup containing the exact historic callback command, current native owner/runtime/Pi receipt proof, and an absent legacy helper. A matching existing receipt-owned shim is idempotent. Unknown regular files, changed bytes, symlinks and other owners are preserved by failing closed.
- A dedicated `--repair-retired-hooks` command performs only the already-migrated recovery, without rewriting Pi or current hooks configuration. Normal `--install-hooks` performs the same compatibility ownership validation during future migration and preserves it on repeat installation.
- Validate owned real directory ancestors before creation and use private directories/files. Retain a recoverable receipt or remove only the just-created exact payload if a write fails. Preflight conflicts before mutating unrelated Pi or current hook configuration.
- Explicit uninstall validates both the Pi extension and compatibility helper before removing either. Remove only an exact receipt-bound shim, then empty owned compatibility directories when safe. Unrelated files remain. Retain compatibility until uninstall; do not remove it merely because current hooks configuration is empty.

## Risks / Trade-offs

- Cached sessions outlive migration → keep a tiny inert helper until explicit uninstall.
- A changed helper belongs to the user → refuse deletion or replacement and retain the receipt for resolution.
- Partial repair/install failure → rollback only newly created proven bytes or keep ownership evidence for an idempotent retry.
- A previous release ignores compatibility metadata on uninstall → deliver the native receipt-aware uninstall path before final acceptance.

## Migration Plan

Root may install the independently reviewed inert payload immediately after verifying exact backup-command evidence, current owner/runtime/Pi hash and absent path, atomically recording compatibility metadata. No user process or collector restart is required for that recovery. Source implementation then validates repeat install and full uninstall with synthetic fixtures. Root deploys the reviewed native binary only after source review, verifies existing cached calls and hashes, and preserves all unrelated hooks and private state. Canonical sync/archive follow explicit acceptance.

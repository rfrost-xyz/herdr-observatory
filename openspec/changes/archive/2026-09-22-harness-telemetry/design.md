# Design

## Context
The existing image polls Herdr snapshots locally and over SSH, and publishes a filtered Work feed. Herdr protocol 22 supports display-only metadata, named tokens, expiry and sequence rejection. Codex 0.154/0.155 hooks expose tool/model/compaction events; Pi 0.85/0.86 extensions also expose usage and estimated context.

## Goals / Non-Goals
Use Herdr's ephemeral metadata store and existing collection routes. Do not add a daemon, database, HTTP intake, transcript reader, raw text export or lifecycle authority. No claim of lossless tracing or exact current context for Codex.

## Decisions
- A small Codex shell adapter and Pi extension invoke a short-lived Python reporter through the existing Docker CLI. Parsing, validation, socket selection and metadata construction reside in the image. A new listener would introduce authentication and another store without benefiting sampled display.
- The reporter only uses the single configured local socket. It verifies pane/native session identity, serialises writes with a bounded lock and uses event sequences. A session digest binds tokens for consumers; native identifiers never leave the probe. All owned fields refresh or clear together with a 120-second TTL.
- Use namespaced tokens plus a guarded concise display_agent label, avoiding edits to managed Herdr sidebar configuration. Preserve native session/state integrations. Native Pi integration is a prerequisite; install its official payload where absent.
- Probe decodes only current matching tokens; core and Work feed validate the resulting typed object again. Browser activity remains sampled latest-event observation, with no replay or inference of missing events.
- Codex tool responses are discarded. Pi exports only typed usage counters and phase markers, never reasoning text or content. Context is explicitly estimated; usage is last assistant response, not session totals.
- Installer ships with the image, merges only owned Codex commands and writes a separate Pi extension. Installation refuses conflicting owned paths, offers uninstall, and preserves unrelated entries. Existing sessions need restart/reload to pick up configuration.

## Risks / Trade-offs
- Short tools can complete between polls → show latest observed event, document sampling.
- Long silent tools outlive TTL → report unavailable rather than fabricate heartbeat.
- Docker exec adds process overhead → bounded synchronous Codex timeout and bounded serial Pi queue; no per-token subprocesses.
- Native session identity may arrive after SessionStart → later events retry naturally; never seize native authority.
- Herdr does not atomically guard tokens by native session → check immediately before writing and validate digest again on reads; expiry bounds native display remnants.

## Migration Plan
Build/test image, deploy on both authorised hosts retaining one rollback, install adapters from the image and verify native integration prerequisites and safe end-to-end metadata. Document new-session activation. Remove task-only artifacts and superseded image after health and rollback checks. Rollback uninstalls only owned adapters and selects retained image.

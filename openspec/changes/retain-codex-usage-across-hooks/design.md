## Context

The host adapter reads usage from the exact hook-named session file. The image reporter receives that optional sample, checks the live Herdr pane binding and writes one atomic version-2 metadata report. Herdr retains only the latest report, so an empty usage payload currently clears a previously valid sample.

## Goals / Non-Goals

**Goals:** Keep the last valid Codex sample and its source age when a later hook lacks usage, while continuing to report the latest event and phase.

**Non-goals:** Read a different session file, synthesize counters, change the Codex adapter or retain data after a pane changes session.

## Decisions

Decode and validate the pane's existing Observatory metadata under the reporter's lock after confirming the native session binding. Reuse its numeric fields only when the incoming Codex sample is absent or older. The report still writes all version-2 groups atomically. This uses Herdr's bounded metadata rather than adding a host cache or daemon.

## Risks / Trade-offs

- A session may show older usage for longer. Preserve `usage_seq` so the card labels it last known after two minutes.
- A pane can be reused by another session. The existing binding check and encoded binding prevent its previous sample crossing that boundary.
- A malformed old report must not become a fallback. Decode through the existing validated reader and require the Codex usage source.

## Migration Plan

Build one reviewed image from the merge commit and deploy it to both hosts, retaining the currently healthy image as rollback. The installed hook payloads do not change. After acceptance, remove only older, unreferenced Observatory images and the merged worktree.

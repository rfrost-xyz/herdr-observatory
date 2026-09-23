# Design

## Context

See proposal.md. Herdr's Codex adapter already receives start and stop hooks and serialises one parent-bound metadata report per hook. Its numeric wire v2 has 13 keys, below Herdr's 16-key per-report limit. The display has React and imperative renderers.

## Goals / Non-Goals

**Goals:** Keep a compact, session-bound observation across later hooks and expose it in both renderers without adding collection authority.

**Non-Goals:** A lossless child roster, success status, child content or a progress percentage.

## Decisions

- Use a separate bounded `obs_children` value containing start count, stop count and latest subagent-hook time. The numeric usage groups and their source age remain untouched. A validated UserPromptSubmit establishes zero counts. Later hooks carry the summary forward under the existing lock and session binding. Missing or malformed prior data stays unknown.
- Count hook observations, not distinct child identities. The current adapter strips child IDs before reporting, and the user wants a compact observation. A hook can be missed or repeated, so the UI always says “observed”. Stops are labelled stops, not completions.
- Show a separate compact summary row beneath current activity, naming observed starts and stops in visible text, with an accessible explanation and last-known age. This keeps the state icon authoritative and avoids interpreting a ratio as completion.

## Risks / Trade-offs

- Missed, duplicate or resumed hooks can change event counts without reflecting a child roster. The observed-event label and absence of a live count make that limitation explicit.
- Adding one metadata key requires checking native limits and old v1/v2 reports. Missing keys stay unavailable; numeric v2 ordering remains unchanged.
- Card height can grow. Check the four supported desktop geometries and narrow tiled viewport.

## Migration Plan

Build one versioned image for both hosts, preserve current and rollback images and config, reinstall the image-supplied Codex adapter, restart/reload Codex sessions as needed, then verify filtered API and browser behaviour. Roll back via the previous image and adapter payload if needed.

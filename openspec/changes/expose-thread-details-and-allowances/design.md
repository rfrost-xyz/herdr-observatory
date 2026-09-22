# Design

## Context

The existing image owns collection, rendering and authenticated Work publication. Host adapters are short-lived hook handlers. Iapetus has no reachable SSH server, so an image-initiated Codex read on that host cannot use the existing SSH route. Codex 0.155.1 exposes account-bound allowance and reset-pass data through read-only app-server RPC.

## Goals / Non-Goals

Keep account identity independent of machine location and keep secrets outside the image. Preserve the single-screen composition and truthful metric scopes. No login management, reset redemption, extra host daemon or general-purpose credential access is introduced.

## Decisions

- Existing Codex hooks run a bounded, throttled one-shot account read and pass an allowlisted summary to the image. Idle data ages visibly and expires; a documented one-shot refresh uses the same adapter. This avoids enabling an SSH service on iapetus or mounting authentication files.
- Hash account identity at source and map it to explicit labels in private configuration. Export no account hashes to browser output. Collect remote cached summaries through the existing SSH route and share selected summaries separately from project disclosure.
- Keep weekly windows selected by their reported duration, not their primary/secondary position. Preserve native reset-pass count separately from weekly reset time; unknown credit details remain unknown.
- Use a dedicated footer renderer with bounded Personal/Work panels and relative time labels. Persistent thread instruments replace overlays. Click glitches are decorative and never record fake activity.

## Risks / Trade-offs

- Hook-only collection does not refresh an idle account continuously. Show source age, expire old data and document manual refresh.
- Optional upstream fields can be absent. Validate independently and show unknown values.
- Dense data at 720p can clip. Check all eight cards and both footer panels at 720p and 1080p before deployment.

## Migration Plan

Build and test one immutable image, update both existing Compose deployments, install the image-owned hook payloads, configure private account mapping and authorised allowance sharing, and verify live source and browser output. Preserve the immediate previous image and configuration for rollback.

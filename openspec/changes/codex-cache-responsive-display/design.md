# Design

## Context

The installed Codex reporter is a short-lived hook process. It reads one bounded rollout tail and sends numerical metadata into the matching Herdr pane. A separate short-lived authenticated app-server process already reads account limits. The browser is a local static page with imperative DOM updates, browser-native Canvas effects and fixed full-screen geometry. See proposal.md for the user-facing motivation.

## Goals / Non-Goals

**Goals:** Preserve existing CLI sessions and Work filtering, deliver meaningful recent cache and account activity, and make the browser view fit tiled windows with the chosen loading.dev component.

**Non-Goals:** Change Codex prompt caching policy, diagnose individual cache misses, replace the Codex CLI with an Observatory-owned app-server thread client, expose raw rollout content, or infer a complete subagent roster.

## Decisions

### Codex source and recent samples

Keep hook-time rollout enrichment for CLI threads. `thread/tokenUsage/updated` is documented for the app-server connection handling an active thread; launching a second short-lived app-server does not subscribe to an existing CLI thread. The hook adapter will emit one bounded numerical sample and opaque association derived from validated hook identifiers. The reporter will deduplicate by session plus usage sequence, calculate differences only from increasing cumulative counters, and hold a short bounded rolling series. Model and compaction observations are markers, not miss explanations. Session and counter discontinuities clear the baseline. No last-response count is added per hook.

Only aggregate recent counters and markers enter presentation metadata. Opaque child and turn references remain on the source side of the reporting boundary; the adapter exposes a small validation seam for a future independently measured child source, without treating SubagentStop itself as a usage sample.

### Account activity

Extend the existing app-server exchange with `account/usage/read` after rate limits, within one shared timeout and bounded response. Accept summary integers and a capped, ordered list of valid ISO calendar dates and non-negative token counts. The account hash from the rate-limit response remains the authority for joining the two reads. A usage error leaves allowance data intact. The existing account mapping, ten-minute expiry and explicit sharing path apply to new fields. The UI labels activity as account-wide ChatGPT subscription tokens, not local cache reuse or API billing.

### React presentation and build

Use React 19 and a pinned `loading-dev` package as build-time dependencies. Render the display shell and data-driven fleet, thread, observation and account panels with React. Keep the event-effect, title, music and pixel canvases as imperative renderer instances attached through stable refs/elements, so a data update does not recreate native/WASM sessions. Build a self-contained local JavaScript asset during image construction; no CDN, Node process or package install runs in the deployed container. Retain current data reconciliation, source-expiry and disclosure rules.

Responsive CSS reflows the fleet and footer at narrower widths and allows bounded internal scrolling for thread and account panels. The established full-screen layouts retain the current composition. React is a presentation choice, while CSS and viewport checks establish responsiveness. Compare bundle size and initial render time against the previous static page before claiming performance improvement.

## Risks / Trade-offs

- [Rollout format changes] -> Keep bounded parsing and unknown fallback; an app-server thread client can replace the source later without changing the numerical presentation contract.
- [Duplicated hooks or counter reset] -> Deduplicate by session and source time, require monotonic cumulative counters and clear baselines on discontinuity.
- [Account activity accidentally disclosed] -> Reuse explicit hashed-account mapping and sharing validation; validate each exported field and date.
- [React rerenders disturb canvas playback] -> Keep stable canvas elements and lifecycle tests for completion, resize, reduced motion and source loss.
- [Extra bundle and JavaScript work] -> Pin and bundle locally, measure asset size and browser timings, and report regressions honestly.
- [Open PR #2 may merge first] -> Rebase this independent branch onto current main and renew checks and review before publication.

## Migration Plan

Build and verify the new image on the branch. Keep the installed hook payload and previous versioned image available, then install the updated adapter through its idempotent installer and deploy the new image to both hosts. Verify account and cache source labels, responsive views and Work disclosure. Roll back by selecting the prior image and prior adapter payload if live acceptance fails. Archive and publish after local gates and independent review; merge only with explicit authorisation.

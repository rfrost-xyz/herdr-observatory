# Design

## Context
The current server runs a native three-effect adapter and downloads complete ANSI animations. The website provides a 37-effect WASM Session API with typed cell buffers.

## Goals / Non-Goals
Generate incremental frames locally while retaining server-side disclosure. No Herdr control or raw terminal capture.

## Decisions
Vendor the exact website module and bundle with SHA-256 hashes, origin URLs and upstream licence; no runtime CDN dependency. Derive the catalogue from the binary. Use a shuffled bag and preserve the existing complete-frame scheduler and hold. Read cell symbols, foreground, background and flags directly. Keep filtered terminal layout generation on the server, remove the obsolete frame endpoint, native helper and Rust build. Serve only explicit static paths with application/wasm and a narrowly scoped wasm-unsafe-eval CSP.

## Risks / Trade-offs
Browser CPU and effect duration vary: test every effect to natural completion on a full terminal fixture. Runtime errors free the session and retain the live TUI. Pause/reduced motion/hidden tabs freeze progress; source loss cancels obsolete captures. Upstream binary provenance is the public site artifact, not an asserted reproducible build from a guessed branch.

## Migration Plan
Build a versioned image, deploy through existing Compose on both hosts and retain the previous image for rollback. Verify Work disclosure and same-origin assets. Update README and CI; archive after independent review.

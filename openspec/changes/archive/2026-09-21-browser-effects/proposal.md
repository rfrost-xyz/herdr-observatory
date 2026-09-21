# Proposal

## Why
Only three effects are enabled and server-generated frame downloads are wasteful. Use the browser renderer deployed by Omarchy and expose its complete catalogue.

## What Changes
- Vendor pinned WASM and JavaScript with checksums and licence provenance.
- Generate frames locally from filtered terminal snapshots, rotating all effects without consecutive repeats.
- Preserve complete playback, themed colour, hold controls, privacy and reduced motion.
- Remove server frame generation and Rust adapter build; update images, tests and README.

## Capabilities
### New Capabilities
None.
### Modified Capabilities
- `office-display`: browser-local full-catalogue rendering.

## Impact
Browser renderer, HTTP assets/CSP, Docker build, CI and both deployed profiles. The obsolete frame endpoint is removed.

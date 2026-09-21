# Design
## Context
The canvas fills the viewport but font measurement and 2.5% margins shrink terminal content.
## Goals / Non-Goals
Fill the viewport without changing the 120×36 data canvas or thread pagination.
## Decisions
Use eight-pixel edges and derive cell width and row height directly from viewport dimensions. Position live glyphs individually, using the same geometry as WASM cells. Bound glyph width to its cell. Pin CSS canvas to the viewport using dynamic viewport height.
## Risks / Trade-offs
Non-widescreen windows alter glyph spacing; verify viewport edges at 720p, 1080p and a narrower window. No disclosure changes.

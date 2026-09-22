# Design
## Context
The browser composes a 140x44 character grid for both live canvas and effect captures. Its ASCII bars, capitalised labels and unbundled system font dominate its appearance.
## Goals / Non-Goals
Make the existing terminal feel like a modern TUI. Preserve sampling truth, controls, grid geometry and Work privacy. No new data transport or host desktop configuration.
## Decisions
Bundle the installed JetBrainsMono Nerd Font regular face with its licence and checksum. Serving a complete face avoids a glyph subset that would fail for future labels; a local web font gives Windows the same rendering without a font installation. Load explicitly through the browser font API, retaining readable labels and Unicode fallback when unavailable. Render live and effect cells with the same font.
Use rounded rules, understated section titles, glyph state indicators, compact aligned columns and theme-derived row accents. Keep text-based composition for whole-scene effect compatibility; glyphs supplement explicit state labels. Existing motion remains event-driven; do not add decorative activity.
## Risks / Trade-offs
Font transfer is larger than a small subset → local serving, preload, no external requests; pin checksum and retain upstream licence. Font load failure → keep readable words and fall back to ordinary symbols. Colour alone can obscure meaning → retain state labels and accessible transcript. Unicode glyph width can disturb effects → use code-point-safe truncation and test through the existing WASM renderer.
## Migration Plan
Build both Docker profiles from reviewed source; check font delivery, privacy and health. Retain the previous runtime image for rollback and remove task QA artifacts.

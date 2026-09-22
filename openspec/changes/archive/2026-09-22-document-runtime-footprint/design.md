# Design

## Context
Existing runtime behaviour is unchanged. The README is the operator guide and AGENTS.md is the implementation/deployment map for maintainers.

## Decisions
Use source inspection plus read-only live process/mount checks. Measure whole-container CPU/memory over thirty seconds and separately measure a temporary image-contained music reader, with publication disabled. State omitted costs and avoid attributing the total container to music alone. Publish no live track or project data. Keep host requirements distinct from image contents and optional integrations.

## Risks
Short measurements vary with workload and exclude browser/player cost. Present them as dated observations, not a performance guarantee. Documentation must point to live configuration and code rather than imply historic image tags are permanent authority.

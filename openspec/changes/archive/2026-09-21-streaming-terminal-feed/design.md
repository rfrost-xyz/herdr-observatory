# Design

Deduplicate source captures by host sampled_at, not browser polls. Queue at most 64 records, at most 24 pane observations per capture. Replace older pending observations from the same host, discard stale/offline/disconnected data before emission, and drain at a bounded cadence. Every record retains observation time and identifies source capture time. Lower-energy SAMPLE/PANE effects differ from milestone effects. These are sampled observations, not native Herdr events or inferred tool activity.

Persistent canvas renders smoothly moving feed field, expanding ripples, bounded horizontal tearing and outgoing glyph fragments. Current thread rows retain readable unbroken text and explicit WORKING/INPUT/DONE labels. Reduced motion and pause suppress effects without stopping current state. No raw terminal output, new backend endpoint or disclosure change.

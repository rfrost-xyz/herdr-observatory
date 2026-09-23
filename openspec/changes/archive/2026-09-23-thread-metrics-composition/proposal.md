# Thread metrics composition

## Why

The combined input/output tile compresses two large figures into a narrow block. The user-supplied sketch calls for four clear readings that fill the card width without collisions.

## What Changes

- Give context, session input, session output and cache-read share separate visual regions.
- Render input and output as real text instead of CSS-generated content.
- Increase the project heading's prominence while preserving the status header and compact footer.
- Keep response-only and unknown measurements explicitly scoped.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `activity-dashboard`: specify four distinct thread readings and source scope.
- `office-display`: require the four readings to fit the supported eight-card viewports.

## Impact

Browser renderers, stylesheet, compiled React bundle, UI checks and viewport verification. Telemetry collection and API remain unchanged.

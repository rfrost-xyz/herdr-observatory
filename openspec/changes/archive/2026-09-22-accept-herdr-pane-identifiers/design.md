# Design

## Decisions
Pane IDs are bounded opaque API identifiers, not decimal numbers. Accept ASCII identifier punctuation and letters/digits, then ask Herdr to resolve the exact identifier. Keep the native-session and source guards unchanged.

## Verification
Exercise alphanumeric IDs through the full report function and reject empty/oversized/control-character input. Deploy both images and verify real hook-driven metadata on the restarted session, not just a manually supplied diagnostic event.

# Spec Delta

## ADDED Requirements

### Requirement: Tiled browser presentation
The browser display SHALL remain usable in narrower and shorter Omarchy tiling windows as well as the established 1280x720 and 1920x1080 Windows and Omarchy layouts. It SHALL preserve readable fleet, thread and account information through reflow or bounded internal scrolling, keep the current-state text accessible, and stop decorative motion for hidden or reduced-motion views. Its connection-loading indicator SHALL use the selected Blocks component and SHALL not animate when the connection is established or reduced motion is requested. Browser assets SHALL load locally without a runtime package manager or external CDN.

#### Scenario: Narrow tile
- **WHEN** the browser is resized to a narrow tiled viewport
- **THEN** controls, fleet values, thread cards and account activity remain reachable without horizontal document clipping.

#### Scenario: Full display
- **WHEN** the browser uses a 720p or 1080p full display
- **THEN** the existing one-screen composition, card motion and bounded event effects remain functional.

#### Scenario: Connection and accessibility
- **WHEN** the connection is pending, established or lost, or reduced motion is requested
- **THEN** the loading indicator follows that state, text names the connection state and reduced motion suppresses the animation.

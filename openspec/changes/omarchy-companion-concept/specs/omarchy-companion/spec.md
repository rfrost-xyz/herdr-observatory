## ADDED Requirements

### Requirement: Read-only Omarchy companion
The application SHALL provide a manifest-backed Omarchy bar widget with an anchored popover and a separate ordinary Wayland companion window. Both SHALL read only the existing loopback Observatory state and SHALL NOT attach to, send input to, or change the lifecycle of a Herdr thread. The native UI SHALL use only permitted Observatory projections and SHALL NOT read session files, credentials, SSH keys or raw terminal output.

#### Scenario: Bar to companion
- **WHEN** the operator opens the bar widget and chooses the companion action
- **THEN** the popover shows a bounded current overview and an ordinary tiled window opens without a forced floating rule.

### Requirement: Truthful current state
Thread counts SHALL be scoped to currently reporting hosts and labelled with their meaning. A failed or expired host sample SHALL be shown as unavailable rather than contributing zero threads. Codex allowance SHALL show only mapped accounts and valid, source-dated weekly values. GPU utilisation SHALL be labelled as device utilisation and SHALL NOT be presented as inference requests or token throughput. Missing, stale, unsupported or disconnected data SHALL stay explicit.

#### Scenario: One host stops reporting
- **WHEN** a previously reporting host becomes unavailable
- **THEN** its threads cease to appear, the fleet row says unavailable and the remaining count is labelled as partial.

#### Scenario: Inference has no request source
- **WHEN** GPU utilisation is available but no request-level inference source exists
- **THEN** device utilisation may appear, while inference throughput is explicitly unavailable.

### Requirement: Desktop fit
The popover SHALL use the installed Omarchy panel lifecycle and keyboard dismissal. The companion SHALL let Hyprland place and resize its ordinary window, with readable content at narrow, half and wide tile widths. Both surfaces SHALL expose text labels for status and add no custom animation. The installed shell controls its own panel transitions.

#### Scenario: Narrow tile
- **WHEN** Hyprland narrows the companion window
- **THEN** the content forms a scrolling single column without clipping thread status or source age.

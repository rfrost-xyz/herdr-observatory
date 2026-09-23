# Spec delta

## MODIFIED Requirements

### Requirement: Omarchy display
The dashboard SHALL adopt valid active Omarchy palette changes without modifying desktop configuration and provide a usable fallback, responsive layout, keyboard controls and reduced-motion support. Thread identity, activity, metric and provenance modules SHALL remain reachable when their content exceeds a display row.

#### Scenario: Theme changes
- **WHEN** the selected theme source publishes a different valid palette
- **THEN** the next successful refresh updates dashboard colours; missing or malformed colours use safe defaults.

#### Scenario: Short display viewport
- **WHEN** the viewport cannot fit two full rows of thread details
- **THEN** thread card rows grow to contain their modules and the thread area scrolls to reach them without clipping the footer.

#### Scenario: Narrow tiled viewport
- **WHEN** a browser tile cannot fit the desktop column count
- **THEN** the layout reduces columns, sizes rows from their modules and keeps the cards reachable without horizontal document clipping.

#### Scenario: Initial connection
- **WHEN** the browser is waiting for its first state sample
- **THEN** the connection label has a reduced-motion-aware block loading indicator that disappears when that wait ends; measured percentages remain determinate gauges.

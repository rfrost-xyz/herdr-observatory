# Spec delta

## MODIFIED Requirements

### Requirement: Single-screen activity
The display SHALL fit 1280×720 and 1920×1080 16:9 viewports, and 1280×800 and 1920×1200 16:10 viewports, without document or thread-grid scrolling for up to eight permitted threads. Thread cards SHALL remain fully contained in a four-column, two-row page and occupy the majority of the viewport with a compact title, fleet strip and recent-activity area. Narrow tiled browsers MAY scroll their own content. The count SHALL include all permitted live panes.

#### Scenario: Eight visible threads
- **WHEN** a page contains eight permitted threads with maximum supported card content
- **THEN** all eight cards and their current state, activity and visual telemetry fit on screen without clipping or scrolling.

#### Scenario: Many agents
- **WHEN** more agents exist than fit on one page
- **THEN** the total includes all permitted live agents and labelled eight-thread pages rotate every 15 seconds by default, with keyboard manual navigation that holds the selected page until rotation resumes.

#### Scenario: Viewport resize
- **WHEN** the browser resizes between supported full-screen proportions
- **THEN** card composition and bounded activity effects resize without document or thread-grid scrolling.

#### Scenario: Narrow browser tile
- **WHEN** browser width is below full-screen display width
- **THEN** cards reduce columns and stay reachable without horizontal document clipping.

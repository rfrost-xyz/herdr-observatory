# Thread state header

## Why

The current card splits project, branch, state and activity across short rows. The state badge is small and metadata occupies the heading rather than a quiet footer.

## What changes

- Add a state-coloured top bar with project and branch on the left and a large native-state icon on the right.
- Place the latest activity and reported tool under the icon while keeping their observation scope distinct from state.
- Move harness, host, pane and model to the bottom left opposite compaction and age detail.
- Keep the one-dial telemetry layout and the existing source-bound unknown states.

## Impact

Presentation changes to the two browser renderers, stylesheet and focused UI tests. No feed or telemetry schema change.

# Design

## Decisions
Capture only the CLI's 24 content rows for effects and clip playback to that rectangle. Render live machine/thread rows and controls on every frame throughout playback. Keep complete effect playback, all 37 effects, non-repetition and the 120-second hold.

Working rows have independently phased text spinners and subtle moving highlights. Observed state or telemetry sequence changes add a brief highlight; duplicate polls do not retrigger it. Idle/done/unknown rows stay settled. Paused, reduced-motion and stale/disconnected views suppress motion. Use Working, Blocked, Done, Idle and Unknown in rendered and accessible state text.

## Verification
Test region capture/clipping and continued thread/controls rendering during effects. Test independent row timing, duplicate observations, stale/offline/reduced-motion behaviour and canonical state words. Validate the complete effect catalogue with CLI-sized captures and deploy both images.

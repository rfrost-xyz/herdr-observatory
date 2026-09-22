# Design

A persistent canvas renders the entire CLI-like surface: terminal transcript, live process listing and prompt, without cards/charts/panels. Real state changes create timestamped records and bounded impacts: agent discovered/removed, start/finish/input/idle/unknown, host loss/recovery and browser connection loss/recovery. First snapshot seeds a baseline without an impact storm. Repeated snapshots, revised titles/revisions and time passing do not produce synthetic work events. Host loss does not imply agent completion.

Each impact launches a radial displacement wave through a projected grid and terminal glyphs, a short phosphor wash and settling text motion; event colour encodes status. Ongoing ambient cursor/screen scan is distinct from event reactions. Effects are bounded, avoid rapid full-screen flashes and become static under reduced-motion preferences. Render at device pixel ratio capped at two, cap animation frame rate at thirty and pause scheduling work in hidden tabs.

The transcript and process listing adapt to viewport dimensions and truncate long strings without scrolling. Canvas draws text rather than HTML. An accessible textual equivalent exposes current state and recent observations. Keyboard shortcuts pause visual effects or enter fullscreen. No new terminal-content access or shell execution.

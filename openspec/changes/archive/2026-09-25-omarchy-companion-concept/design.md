# Omarchy companion concept

## Decision

Use **Quickshell/QML for the Omarchy bar widget and popover**. Keep the tiled companion a separate, ordinary Wayland toplevel. **Qt Quick is the provisional main-window choice**, because the installed Omarchy shell already uses Qt/QML and it offers a mature accelerated scene graph. A **Rust/Slint main window** is a credible alternative, especially if a smaller idle footprint or an independent Rust client is more valuable than sharing QML patterns with the popover. A Rust language choice alone does not determine rendering performance: Qt Quick and Slint use different renderers and backends. Select the main-window toolkit after a small real-Hyprland comparison, not from language-level speed claims.

Both clients should read the existing Observatory service as their read-only data source and keep process lifetimes independent. Share data semantics and visual tokens; share QML components only if Qt Quick wins. Do not add a Rust collector to the first client: it would duplicate an already running collector without solving a measured bottleneck. Revisit Rust for collection or parsing only if a profile identifies hot work or distribution needs justify it.

The [Rails World 2026 keynote](https://www.youtube.com/watch?v=vDjW_dRyKXY&t=1103s) makes the native-client case after the linked 18:23 segment (roughly 24–27 minutes) and proposes Rust for HEY's backend at roughly 29–33 minutes. That is an example, not a benchmark for this application. Observatory already has a Python backend and a loopback API. Its collection cost and system boundaries need measurement before a rewrite could be justified.

For this mostly read-only companion, visible freshness is currently bounded by Observatory's sampling and browser polling, not by intensive client computation. Rust could reduce CPU or memory if a measured collector, parser or aggregation path is hot; it could also offer a compact client executable. It will not shorten an upstream sample interval or automatically improve UI frame pacing. Compare complete processes, including toolkit and renderer, rather than comparing Rust with QML as languages.

## Product roles

| Surface | Job | Density | Interaction |
| --- | --- | --- | --- |
| Bar | Signal that deserves attention | One Herdr state cue, active count, one allowance or fleet warning | Open popover |
| Popover | Ten-second answer | Active threads by host, weekly Codex allowance, ws-255 inference status, offline/stale sources | Open companion; keyboard dismiss |
| Companion | Peripheral workspace while Herdr runs | Resizable thread roster plus allowance, inference and fleet panels | Inspect current evidence; launch existing Herdr separately |
| Herdr | Thread operation | Full terminal/session controls | Remains the authority for input and lifecycle |

Do not reproduce recent observations. They remain in Herdr. Never imply that a bar badge or subagent observation is a total count of all work in progress. The plugin is a view, not a thread controller.

## Layout and tiling

- **Bar:** a compact glyph, `N active` only when that count is supported by current permitted Herdr panes, and an alert dot for disconnected or stale sources. Avoid a permanently scrolling marquee or percentages with missing evidence.
- **Popover:** fixed width around 400–440 logical pixels, anchored to the bar on the current monitor. It should stay inside the work area at any bar edge, close on Escape/outside click and never claim a Hyprland tile. Rows summarise a few current threads; an overflow link opens the companion.
- **Companion:** ordinary Wayland toplevel, with no forced floating rule or always-on-top behaviour. Hyprland owns tiling, workspace placement, focus and monitor choice. The app must respond to its assigned width rather than query Hyprland to set geometry.
- **Wide tile (about 900 px and up):** thread roster takes the main column, allowance/inference/fleet form a narrow right rail.
- **Half tile (about 600–900 px):** roster remains first; metric modules wrap below or into two columns.
- **Narrow tile (about 360–600 px):** one scroll column with compact thread rows and concise metric cards. Keep details in accessible labels or expandable rows, never clip an age or missing-state label.
- Omarchy theme colours should come from its public shell theme tokens in the plugin and a matching exported palette in the companion. Check contrast for status text and never use colour alone. Respect reduced motion.

## Data ownership and trust

1. Observatory remains the collector and disclosure boundary. Both surfaces read its existing `GET /api/state` on `127.0.0.1:8789`. Its Host/Origin checks are currently browser oriented, so a future implementation must verify the Qt/Quickshell request headers against that contract before selecting a client transport. Do not weaken Host/Origin or expose the API on the tailnet to make the client work.
2. `hosts[].agents`, sampled timestamps, metrics and mapped `allowances` are presentation inputs. Reuse the service's permitted Personal/Work projection. Never read Herdr pane files, Codex credentials, session transcripts, SSH keys or Docker sockets in the UI process.
3. Each value retains its source timestamp and scope. Unknown, unsupported and stale are explicit. A disconnected host is not a zero-thread host. The popover and companion should use the same derivation rules, with source logic in a small tested adapter instead of duplicated QML expressions.
4. `hosts[].online` currently means a successful Herdr collection, not independently proved machine reachability. Label those rows "reporting" or "unavailable". Any future machine reachability cue needs its own source and timestamp. Existing GPU utilisation and memory describe the sampled device, not inference throughput. The first inference panel can show reported GPU occupancy and model residency only if a read-only Ollama source is available. Request count, prompt/output tokens, latency and active generation require explicit per-request instrumentation or a supported cumulative metrics endpoint; never derive them from GPU load, container health or `ollama ps` alone.
5. ws-255 Ollama is currently loopback bound inside its WSL/omaterm topology. Do not expose it to iapetus. A future collector can run a bounded, read-only local probe over the existing authenticated SSH path and publish a sanitised sample through Observatory. Verify the current service/API first. This is a separate delivery task with source-age, disclosure and error contracts.

## Proposed client modules

```text
Herdr / Codex hooks / host probe / optional ws-255 inference probe
                     │ existing private collection paths
              Observatory service
                     │ loopback read-only state
        ┌────────────┴─────────────┐
 Omarchy Quickshell plugin      Qt Quick or Rust/Slint companion
 bar + transient popover        tiled Wayland window
        └──── shared data rules and visual tokens ────┘
```

Build the actual plugin as a user-owned plugin with its own manifest, following the installed `bar-widget` contract and `qs.Ui.Panel` conventions. Keep plugin and companion code in this repository; copy the reviewed plugin files into the local user plugin directory with a reversible installer, without editing `/usr/share/omarchy`. The standalone concept page is an interaction and layout reference, not production code.

## Implementation sequence and later evaluation

1. Specify a bounded native presentation projection and tests for freshness, disclosure, host loss and allowance scope. Decide whether the existing state JSON is sufficient without an extra endpoint.
2. Build the Quickshell bar and popover against a synthetic fixture, then the live loopback feed. Check bar-edge anchoring, monitor placement, keyboard access and Omarchy theme changes.
3. Spike the same representative thread roster and metric cards in Qt Quick and Rust/Slint on the actual Omarchy host. Pin release build settings and record each candidate's actual window backend and renderer. In particular, select Slint's Wayland winit backend and a GPU renderer explicitly for the primary comparison; its Qt backend uses software rendering and would confound the result. Record Qt Quick's graphics backend too. Compare cold start, idle and updating CPU/RSS, frame pacing while resizing narrow/half/wide Hyprland tiles, accessibility, theme adaptation and packaging. Keep the data fixture and measurement conditions identical. Choose the companion toolkit from those results, then build the full toplevel and test it alongside a live Herdr window. See the [Slint backend and renderer documentation](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/) and [Qt Quick scene graph documentation](https://doc.qt.io/qt-6/qtquick-visualcanvas-scenegraph.html).
4. Add ws-255 inference telemetry only after selecting a measured source. Verify the local collector, SSH path, sample age and missing/error behaviour separately.
5. Package and trial the first plugin on the local Omarchy host. Keep the installed copy independent of the development worktree and verify that uninstall removes its files and bar entry without touching other plugins. Any later fleet deployment is a separate decision.

## Acceptance for this concept

- A reviewer can distinguish the bar, popover, companion and Herdr roles.
- The prototype shows the hierarchy at narrow and wide widths with synthetic data clearly marked.
- No displayed mock value is represented as live evidence or as an inference quota.
- The architecture names the source and authority of every proposed metric, plus gaps that require implementation.

# Omarchy companion concept

## Decision

Use **Quickshell/QML for the Omarchy bar widget and popover**, and a separate **Qt Quick Wayland toplevel** for the tiled companion. Use the existing Observatory service as the read-only data source. Share visual tokens and presentation rules between QML surfaces, but keep their process lifetimes independent. Do not add Rust to the first client: it would duplicate an already running collector without solving a current bottleneck. Revisit a small Rust library or service only if measured client parsing, local inference instrumentation or distribution needs justify it.

The [Rails World 2026 keynote](https://www.youtube.com/watch?v=vDjW_dRyKXY&t=1103s) makes the native-client case after the linked 18:23 segment (roughly 24–27 minutes) and proposes Rust for HEY's backend at roughly 29–33 minutes. That is an example, not a benchmark for this application. Observatory already has a Python backend and a loopback API. Its collection cost and system boundaries need measurement before a rewrite could be justified.

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
 Omarchy Quickshell plugin      Qt Quick companion
 bar + transient popover        tiled Wayland window
        └──── shared visual tokens and state adapter ────┘
```

Build the actual plugin as a user-owned plugin with its own manifest, following the installed `bar-widget` contract and `qs.Ui.Panel` conventions. Keep plugin and companion code in this repository; package and install through a later change rather than editing `/usr/share/omarchy`. The standalone concept page is an interaction and layout reference, not production code.

## Implementation sequence after concept approval

1. Specify a bounded native presentation projection and tests for freshness, disclosure, host loss and allowance scope. Decide whether the existing state JSON is sufficient without an extra endpoint.
2. Build the Quickshell bar and popover against a synthetic fixture, then the live loopback feed. Check bar-edge anchoring, monitor placement, keyboard access and Omarchy theme changes.
3. Build the Qt Quick toplevel with responsive layouts. Test real Hyprland tiling at narrow, half and wide sizes and alongside a live Herdr window.
4. Add ws-255 inference telemetry only after selecting a measured source. Verify the local collector, SSH path, sample age and missing/error behaviour separately.
5. Package via managed user configuration and verify installation on each intended Omarchy host. No host configuration changes are part of this concept.

## Acceptance for this concept

- A reviewer can distinguish the bar, popover, companion and Herdr roles.
- The prototype shows the hierarchy at narrow and wide widths with synthetic data clearly marked.
- No displayed mock value is represented as live evidence or as an inference quota.
- The architecture names the source and authority of every proposed metric, plus gaps that require implementation.

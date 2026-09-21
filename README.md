# Herdr Observatory

A passive display of Herdr agent activity across your machines. See working agents, tasks needing input, project titles, sampled state changes and resource telemetry in a full-screen dashboard that follows your Omarchy palette.

No agent controls, terminal transcripts, cloud service or remote installation. Python 3.11+ standard library on collector hosts, a browser, Herdr 0.9.x with `api snapshot`, and OpenSSH for remote collection. No npm dependencies or build step.

## Run

```sh
cp config.example.json config.local.json
python -m observatory --config config.local.json --profile personal
```

Open **http://127.0.0.1:8789**. Use the Full screen button or F11 for a wall display. Keep the process running while the display is in use. Ctrl+C stops it; collection workers finish within their bounded timeout.

For a work display:

```sh
python -m observatory --config config.local.json --profile work
```

Work is the default. Add approved absolute paths to each host's `work_roots` before expecting agents there. Personal displays include both categories and provide a project filter. Switching the server profile requires a restart; browser query parameters cannot unlock Personal data.

## Configure machines

`config.local.json` is ignored by Git. Keep real usernames, addresses and project roots there, not in examples or issues. Paths are evaluated on the source machine. The narrowest practical allowlist is best; `personal_roots` override work roots, and unclassified paths are always Personal.

```json
{
  "interval": 5,
  "theme_host": "desktop",
  "hosts": [
    {
      "id": "desktop",
      "label": "Desktop",
      "transport": "local",
      "work_roots": ["/home/user/work"],
      "personal_roots": ["/home/user/work/private"]
    },
    {
      "id": "workstation",
      "label": "Workstation",
      "transport": "ssh",
      "target": "workstation-ssh-alias",
      "herdr": "~/.local/bin/herdr",
      "work_roots": ["/home/user/work"]
    }
  ]
}
```

Use your SSH config for users, Tailscale routing, ProxyCommand and keys. Verify `ssh workstation-ssh-alias python3 --version` first. The collector uses batch mode and will not prompt for passwords or approve new host keys. Each sample sends the read-only Python probe over stdin and installs nothing remotely. New machines need working SSH, Python 3.11+ and Herdr before adding them. There is no network discovery.

`herdr` defaults to PATH, then `~/.local/bin/herdr`. Optional `session` chooses a named Herdr session; configure separate host IDs for multiple sessions. Each host has an independent worker. Snapshot polling is compatible with tested Herdr 0.9.0 and 0.9.1, avoiding terminal attach or takeover.

`theme_host` selects the machine supplying the palette. The collector reads `~/.local/state/omarchy/current/theme/colors.toml` and `theme.name`, falling back to the older `~/.config/omarchy/current` location. Valid colour changes appear on the next refresh. Missing colours use Tokyo Night defaults. No Omarchy hooks or config edits are needed. Fonts use installed JetBrains Mono / Cascadia Code with a monospace fallback; no external fonts are downloaded.

## Windows office display

Run the service in the same WSL distribution/container as Herdr. With the checkout and private configuration already available inside an `omaterm` container, run from Windows PowerShell:

```powershell
wsl -d archlinux -- docker exec -it -u omaterm -w /home/omaterm/Projects/herdr-observatory omaterm python3 -m observatory --profile work
```

The working directory in this example must point to the actual checkout containing `observatory/` and `config.local.json`. Open `http://localhost:8789` in the Windows browser. This relies on WSL localhost forwarding and the container sharing the WSL network namespace. For other container layouts, configure loopback forwarding explicitly; do not expose the service to the LAN. This launch path is documented, not an automated Windows installation.

A Windows-hosted display can read the laptop's Omarchy palette by setting `theme_host` to a configured SSH source. If the laptop is unreachable, the last valid palette remains until it reconnects. Windows locking or display sleep still hides the dashboard. No idle, lock, auto-login or startup policies are modified.

## What the numbers mean

- **Working / needs input / done:** Herdr's reported state, not inferred from CPU load. Done is the current number of agents in that state, not tasks completed today.
- **Task titles:** the title Herdr reports. They can contain sensitive text; approve Work roots accordingly. This is not a tool-call or model-reasoning stream.
- **Observed changes:** discovered agents and changes between snapshots. Rapid transitions between samples can be missed. The last 100 observations and 60 working-count samples per host are retained in memory and cleared at restart. History remains visible as historical data during an outage; the working-count trace leaves gaps for unavailable Herdr samples.
- **CPU and network:** deltas between successful samples. The first sample or a counter reset shows unavailable. Network totals exclude loopback, but may include virtual interfaces.
- **RAM / disk:** kernel memory and the filesystem containing the collector user's home. Containers can report kernel-wide memory, while disk scope follows their filesystem. These are not native Windows host totals or cgroup quotas.
- **GPU:** optional `nvidia-smi`; mean utilisation and summed VRAM across visible GPUs. Unavailable where drivers/devices are not exposed, including many containers. The service never invokes Docker to acquire broader access.
- **Unavailable:** failed SSH, missing Herdr, incompatible data or stale sampling. Disconnected agents are excluded from current counts. The browser stops live activity on fetch failure and also expires old host samples.

## Access and disclosure

The server binds only to `127.0.0.1`, validates Host and Origin, sends no CORS headers and exposes only static assets plus read-only `GET /api/state`. It is a trusted local-user application, not a multi-user authenticated service. Any local process can read its selected profile. Use SSH loopback forwarding for remote viewing; keep the same port on both ends.

Work filtering happens before data enters the browser or history. Full path fields, native agent session identifiers, raw snapshots and terminal buffers are not returned. Machine labels and aggregate telemetry remain visible in either profile. Local configuration is trusted operator input, including SSH aliases. Do not publish your local config or live API output.

## Verify

```sh
python -m unittest discover -s tests -v
node --check web/app.js
node --test tests/test_ui.cjs
openspec validate --all --strict
```

The application needs no OpenSpec installation to run. OpenSpec is used for project delivery. GitHub Actions runs Python tests and JavaScript syntax checks. Live compatibility testing is separate from automated fixture tests.

Protocol reference: [Herdr socket API](https://herdr.dev/docs/socket-api/).

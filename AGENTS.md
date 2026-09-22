# Herdr Observatory: agent guide

This repository builds a passive, single-screen display for Herdr. Read the
[README](README.md) for human-facing installation and daily operation. This guide
explains the implementation and constraints for agents maintaining it. Canonical
behaviour lives in `openspec/specs/`; completed decisions and verification live
in `openspec/changes/archive/`. Verify the actual checkout, image and private
configuration before acting. Historical release hashes are not deployment authority.

## Non-negotiable boundaries

- Collectors are read-only. Never send input to Herdr, change its lifecycle or
  session authority, attach to a terminal, or expose raw terminal output.
- Explicitly installed harness reporters may read `pane.get` and write only
  owned, expiring presentation metadata through `pane.report_metadata`.
- Work disclosure is a server boundary, never a browser filter. Only explicitly
  allowlisted Work roots qualify; Personal exclusions take precedence and
  unknown projects are Personal. Preserve filtering before history/publication.
- Music sharing is a separate explicit operator choice. Share only playback
  state, title, artist, bounded measured bands and capture time. Never forward
  audio, file paths, artwork URLs, provider metadata, secrets or raw IPC frames.
- Keep unknown, stale and unavailable values explicit. Do not invent tokens,
  costs, context usage, tool output, reasoning, beats or intermediate events.
- Use Python 3.11+ standard library, browser-native assets and pinned local WASM.
  Do not introduce runtime package installation, a database, a host observer
  daemon, an Omarchy hook or an external asset dependency as a shortcut.
- Do not commit private configuration, SSH material, live snapshots, personal
  project names or track metadata. Public examples must be synthetic.

## Runtime and source map

| Area | Source | Responsibility |
| --- | --- | --- |
| HTTP entry point | `observatory/__main__.py`, `server.py` | Loopback HTTP, explicit static-asset allowlist, Host/Origin checks, CSP and read-only state/music endpoints |
| Collection and disclosure | `core.py` | Config validation, per-host workers, normalisation, project classification, metric deltas, bounded history and snapshots |
| Host probe | `probe.py` | Herdr snapshot, kernel/filesystem metrics, selected palette files and sanitised telemetry; also sent to remote Python over SSH stdin |
| Work feed | `feed.py` | Work-only projection, authenticated publication, bounded validation, atomic receipt and source expiry |
| Music | `music.py` | Read-only cliamp IPC, independent freshness, persistent SSH stream and latest-sample receiver |
| Harness metadata | `telemetry.py`, `hooks/` | Session-bound report validation, Codex/Pi adapters and selective idempotent installation |
| Display | `web/app.js`, `index.html`, `style.css` | Persistent thread cards, state/event reconciliation, paging, icons, colours, footer layout and browser polling |
| Background | `web/background.mjs` | Pinned Omarchy renderer adaptation, real spectrum, muted theme accent, pointer/click reactions and reduced-motion guards |
| Text effects | `web/effects.mjs`, `web/vendor/` | Pinned WASM sessions; event-line and title-specific bounded factories; source/checksum/licence notices |
| Track title | `web/music-title.mjs` | Track-change-only effects, latest-only pending identity, accessible text and stale/reduced-motion guards |
| Personal title | `web/title.mjs` | Small Rich artwork, independent click/minute schedule, complete playback and static fallback |
| Deployment | `Dockerfile`, `.dockerignore`, `deploy/` | Versioned image, minimal build context, Compose variants and manual Windows browser launcher |

No npm build, React runtime or Rust toolchain is required to run the image.
Python, OpenSSH, collectors, reporters, receivers, browser modules, font and WASM
are inside it. The browser executes Canvas/WASM outside the container.

## Processes and data flow

One `herdr-observatory` container runs per display host. Docker init supervises
the Python application. The application creates host-collector and optional
publisher/music threads. HTTP request threads, probe Python processes and SSH
collector/feed invocations are short-lived parts of that container. Music
publication adds one persistent SSH child, with reconnection on failure. These
are not separately installed host services.

1. Each configured local/SSH host is sampled independently. The default interval
   is five seconds; slow requests can lengthen it. Remote probes are sent over
   stdin to existing Python, not installed as a remote agent.
2. `core.normalise` classifies agents and sanitises technical metadata. A Work
   service excludes Personal agents before creating browser state/history.
3. The Personal service can publish a selected host's Work projection and palette
   through SSH. The destination invokes `python3 -m observatory.feed` inside its
   existing image and stores a private feed. A file older than 30 seconds stops
   contributing agents. The destination still collects its own local Herdr.
4. The browser polls `/api/state` about every two seconds. It derives observations
   from changes between samples; it is not a lossless Herdr event stream. Typed
   harness metadata enriches these snapshots rather than streaming transcripts.
5. Optional music uses a separate path and expiry. It never changes agent state,
   project classification, the Work feed schema or the main collection interval.

## Music and Omarchy integration

There is no Omarchy API connection. `probe.palette` reads `theme/colors.toml`
and `theme.name` beneath the configured Omarchy current-theme directory. When
`theme_path` is unset, it checks the current state path then the legacy config
path; an explicitly configured path does not fall back. The validated palette travels in the existing Work feed.
The website renderer is adapted code served locally, not a live connection to
omarchy.org. Keep upstream attribution and pins when modifying vendored assets.

The source music config selects `/music/cliamp.sock`; the optional Compose
music override mounts the existing cliamp directory read-only. `Music` uses a
persistent Unix connection with only cliamp v2 `state.get` and `spectrum.get`.
It samples at up to 15 Hz. The inspected cliamp v2.2.0 source provides ten bands,
which the browser interpolates into 32 visual bands; this is not extra measured
spectral resolution. It does not start cliamp or control playback.

With explicit `music.publish`, a persistent authenticated SSH channel invokes
`python3 -m observatory.music --receive /feeds/music.json` in the recipient image.
Bounded NDJSON carries metadata/bands only. The receiver atomically replaces one
latest-sample file in the existing private feed volume; it does not accumulate
history. Source timestamps are preserved. Missing, invalid or older-than-three-
second samples become unavailable; paused/stopped playback has no audio energy.
No shared audio, Spotify desktop capture or synthetic silent timeline is used.

`/api/music` is polled at roughly 10 Hz by each visible browser. Closing/hiding
the browser stops its music polling and rendering, but does not stop source
observation or forwarding. Reduced motion suppresses animation. Removing the
music configuration and recreating the service disables its worker; remove the
optional socket mount too when no longer needed. See README for both-host steps.

A read-only bind mount does not restrict Unix socket methods. Code with socket
access has IPC authority, so keep the explicit read-method allowlist. The
observer UID must match socket permissions. Never solve access failures with
world-writable sockets or an entire-home mount.

## What remains outside the image

- Docker Engine/Compose, existing Herdr, SSH/Tailscale and a display browser.
  Collector hosts require existing Python 3.11+ for the read-only remote probe.
- The user's existing cliamp player and Omarchy theme files, when those optional
  integrations are enabled. No music daemon or theme watcher is installed.
- Private `config.json`, SSH config/credentials/known-host records, Compose
  `.env` and optional receiver feed files. These must survive image replacement.
- Small optional harness adapters in the environment where Codex/Pi actually
  run. `hooks/install.py` owns `~/.local/share/herdr-observatory/hooks/codex.sh`,
  `~/.pi/agent/extensions/observatory.ts` and its entries in `~/.codex/hooks.json`.
  It preserves unrelated entries and one private backup. It refuses symlinks,
  conflicting files and chezmoi-managed targets. Never bypass those checks.
- Native Herdr integrations and the harness hook feature configuration remain
  harness requirements. Adapters call `docker exec` to run the image's reporter;
  they are event-triggered, not resident agents. Reinstall image-supplied adapter
  payloads when those payloads change, then restart/reload the harness as needed.

## Installed fleet topology

Use the fleet skill before cross-machine work and confirm current paths/hosts.
The established installations are:

| Host | Profile | Deployment directory | Integration |
| --- | --- | --- | --- |
| iapetus | Personal | `~/.local/share/herdr-observatory/deploy` | Local Herdr, OS theme and optional cliamp socket; remote fleet collection and authorised office publication |
| ws-255 | Work | `~/.local/state/herdr-observatory/deployment` inside the SSH environment | Local Herdr plus received Work/theme and optional music feeds |

SSH to `omaterm@ws-255` reaches the existing harness container. Its Docker CLI
controls the native WSL Docker engine. The Work Compose variant uses explicit
subpaths of that existing home volume because daemon-side paths differ from
paths inside the SSH container. Do not recreate Herdr/Omaterm, its home volume,
Tailscale identity or Windows startup arrangement to update Observatory.

Current mount contracts are `/config`, `/herdr`, `/theme` on the Personal host,
optional `/music`, and `/config`, `/herdr`, writable `/feeds` on Work. The app
has no Docker socket mount, uses a read-only root and drops capabilities. HTTP
binds only `127.0.0.1:8789` with host networking. Neither profile has a public
HTTP receiver, permissive CORS or an authentication system for multiple users.

Docker's `unless-stopped` policy handles application restarts. Docker/WSL boot
startup is a host prerequisite, separate from browser launch. No browser login
or kiosk autostart is installed by this project; the PowerShell launcher is
manual. Never change lock-screen, firewall or VPN policy to make the display work.

## Presentation contracts

- One 16:9 viewport at 1280x720 or 1920x1080, no document scrolling. Eight cards
  per page, default 15-second auto-paging only when more than eight are present.
- Use Herdr's Working, Blocked, Done, Idle and Unknown labels. Preserve per-card
  impulses only for observed changes; no travelling bars or literal heartbeats.
- The pixel canvas ends at the full-width opaque footer. All background inks,
  including music/click peaks, are muted blends of the active accent/background.
- Recent events remain four bounded single lines with semantic icons. Effects
  stay aligned to the text span and never obscure its icon or thread cards.
- Event effects use a fixed internal 120-second cooldown, no replay backlog and
  no consecutive repeats. Title effects use a separate minute/click schedule.
  Do not relax event-line limits to support multiline title artwork.
- Hidden/reduced-motion and asynchronous loading failure paths must release
  sessions and retain readable static content. Keep the accessible Rich button
  and current-state transcript. Display only the theme name, without a suffix.

## Verification and release workflow

Run the repository checks for runtime changes and before a runtime release:

```sh
python -m unittest discover -s tests -v
node --check web/app.js
node --test tests/test_ui.cjs tests/test_wasm.mjs tests/test_background.mjs tests/test_title.mjs tests/test_music_title.mjs
openspec validate --all --strict
```

For documentation-only changes, verify paths, commands and claims against
source/current state, check the diff and validate OpenSpec. Do not rebuild or
restart healthy containers just to publish README/AGENTS changes.

Use change-lifecycle, git, worktrunk and change-request skills for repository
delivery. Review independently, record requirement-to-test evidence, synchronise
canonical specs, archive and publish. Do not merge/remove the worktree without
explicit authorisation. For documentation-only changes use `skip_specs: true`
when no observable behaviour changes; do not manufacture a behavioural delta.

Build one immutable versioned image and transfer that same image to both
engines. Preserve the immediate previous image and compatible private config
before replacement. Keep `.env.previous` and `config/config.json.previous` for
rollback. Verify Compose health, exact served assets, correct profile, Work
filtering, palette sync and optional music separately. Health alone proves only
HTTP availability. Re-run browser geometry checks for presentation changes and
refresh the actual browser after deployment. Do not claim Windows desktop QA
when only SSH/container endpoints were inspected.

Measure resource cost with a stated scope and duration. Docker CPU percentages
are fractions of one logical CPU, and container memory/CPU exclude the browser,
cliamp and host services. A whole-container reading is not music-only overhead.
Reader-only benchmarks exclude SSH and player-side IPC work. Do not publish live
metadata while profiling. Remove task-owned fixtures, test processes and stale
Observatory release tags after acceptance; do not prune unrelated Docker assets.


### Glanceable cards and hook coverage

Project and safe checkout leaf replace the terminal title in cards. The private probe keeps native checkout paths only until classification; the HTTP/Work projection exports a leaf only when its category matches the pane. Never add project mounts or infer a branch from its directory label. Work feed receipt revalidates leaf labels. Display Host/Client identity derives from the configured local host and file-feed presence, not Docker hostname or privacy profile.

Numeric tiles appear only for fresh supported values, with zero distinct from missing. Codex hooks do not expose usage counters; Pi coverage depends on provider usage/context. Codex SubagentStart/SubagentStop are latest parent-session-bound observations only. Drop child IDs/types/transcript paths/content, do not construct roster counts or interpret stop as permanent completion. Updating registrations requires the existing image-supplied hook installer on both harness hosts and harness restart/reload. No new persistent process is permitted.

Music title effects are separate from Rich and event effects, use the bounded single-line factory, and trigger only after a fresh track baseline changes. Preserve the sr-only track equivalent while the visual title is hidden. On rapid changes finish the session without displaying stale metadata and retain only the latest pending track; hidden/reduced-motion/stale paths cancel and reset the baseline.

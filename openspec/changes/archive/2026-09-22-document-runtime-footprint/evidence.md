# Audit and documentation evidence

Inspected release: `herdr-observatory:baa42fe`, healthy with `unless-stopped`. This change is documentation-only and does not redeploy, change private configuration or install services.

## Measurements and scope

- Whole-container cgroup v2 counters: ten approximately three-second windows, thirty seconds total. CPU mean 10.61%, minimum 1.81%, maximum 22.81% of one logical CPU. Memory.current range 25.8–38.1 MiB. Host reports sixteen logical CPUs. This includes probes/forwarding and excludes browser and player processes.
- Independent temporary reader inside the existing image: 450/450 available read-only samples in 30 seconds, 0.446% of one logical CPU. Publication disabled. This excludes SSH and player-side IPC cost. Its maximum RSS was 18 MiB for an entire temporary Python process, not incremental resident music memory; README does not present it as an extra service footprint.
- Current sanitised music frame encoded to 303 bytes, approximately 4.44 KiB/s at fifteen frames/second before SSH/Tailscale overhead. No track values were recorded.
- An instantaneous Docker sample was 1.73% CPU and 26.71 MiB; the longer sample is used in the README because collection has bursts.

## Architecture verification

- Docker process snapshot: init, Python and one SSH child. Code inspection confirms per-host and music threads, transient probe/feed children, and persistent music forwarding inside the container.
- Existing source mounts /config, /herdr, /music and /theme are read-only; no Docker socket or entire-home mount.
- User-service queries with the actual user bus returned no matching herdr-observatory or cliamp units/unit files. This is a scoped query, not a claim about all host services.
- probe.py reads Omarchy palette files. music.py hardcodes state.get and spectrum.get against existing cliamp v2.2.0. No Omarchy API call, theme hook or playback control is involved.
- Code and Compose confirm the receiver runs from the same image, writes only a bounded latest sample, and has independent expiry.

## Documentation acceptance

README distinguishes image contents, required host services/files and optional adapters/player; documents disabling/local-only music and browser-versus-container resource scope. AGENTS.md maps modules, sampling/disclosure, music/Omarchy paths, host exceptions, fleet deployment, renderer contracts, testing, release and cleanup.

No behavioural spec delta is required: skip_specs is enabled. Verified 21 referenced source/test/deployment paths and the documented commands against the repository. Independent documentation review found only a theme fallback precision issue, now corrected. Strict OpenSpec validation and git diff --check pass. Temporary resource audit JSON was removed and the temporary benchmark exited. No runtime tests were rerun for this documentation-only change; deployed image remains baa42fe. Publication is through the existing ready PR #1, without merging.

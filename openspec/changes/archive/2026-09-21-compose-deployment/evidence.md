# Acceptance evidence

Reviewed implementation and installed image tag: bace01e. Both engines run identical image ID sha256:ad2eab02dbcbb9e849d84722c09d905eb3a75f3566de05aea75f339b24d7cf59. The dedicated image contains application assets; neither service mounts a source checkout.

| Requirement / scenario | Implementation | Verification |
| --- | --- | --- |
| Reproducible supervised deployment / restart and update | Dockerfile, deploy/compose*.yaml, deploy/README.md | Both staged on alternate loopback ports, then migrated; both healthy after Compose restart; image IDs match; unless-stopped, read-only roots, non-root users, bounded logs and scoped mounts inspected; update/rollback procedure documented |
| Bounded socket observation / invalid socket response | observatory/probe.py, core.py; tests/test_probe.py | Fixed session.snapshot request, split frames, invalid/mismatched/error frames, 4 MiB size limit and total deadline tested; live socket compatibility on both Herdr versions verified |
| Work feed continuity and disclosure | observatory/feed.py, tests/test_feed.py | Container receiver uses quoted fixed module command over SSH; live Personal publication healthy, both Work sources online, personal-project exclusion and matching active palette verified after restarts |

35 Python tests pass both on the host and inside the release image. Six Node tests, JavaScript syntax, strict OpenSpec and diff checks pass. Independent adversarial review approved bace01e with no actionable findings and separately verified graceful SIGTERM exit.

Installed Compose configuration lives outside development checkouts. Personal uses read-only config, Herdr directory and active-theme parent binds. Work uses only named-volume subpaths for config and Herdr (read-only) plus its writable private feed directory. Neither mounts a whole home, SSH directory or Docker socket. Tailscale SSH works with a dedicated verified known-host file and no mounted private keys in this installation.

The old transient Personal unit is absent/inactive. Staging containers and the old broad-mount Work container were removed after acceptance; rollback metadata and previous application release remain private. Each host has exactly one running Observatory container. Docker service on the laptop is now enabled at boot after successful administrator authentication. Workstation retains its existing native Docker/WSL headless startup arrangement. No full host or engine reboot was performed, to avoid disrupting unrelated workloads; application restart and boot configuration were checked separately.

Health checks report HTTP availability independently of source availability. A read-only Unix-socket directory is not an OS-level read-only API: the fixed request and no-control HTTP interface are the application boundary. Windows physical-screen rendering and WSL localhost forwarding remain outside this container-only verification. No shared desktop/theme configuration changed. No programme register applies.

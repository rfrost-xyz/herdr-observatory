# Acceptance evidence

Implementation: a4a08e2; ordering correction and deployed runtime: 59040d6.

| Requirement / scenario | Implementation | Verification |
| --- | --- | --- |
| Single-screen activity / many agents | web/style.css, app.js | Five Node tests include bounded pages, aggregate counts and row budgets at 1280x720 and 1920x1080; source inspected for no document or panel scrolling |
| Work-only publication / mixed activity | observatory/feed.py, core.py | Allowlist and privacy tests; live office response includes permitted agents from both sources and excludes the configured personal project |
| Feed expiry and theme continuity / laptop offline | feed.py, core.py | Expiry, duplicate and out-of-order tests; stopped publisher for over 30 seconds: laptop agents and metrics absent, workstation remained online; restarted workstation container with cached palette retained |
| Independent Work service / restart | deploy/run-office-container.sh, Start-OfficeDisplay.ps1 | Work container running with unless-stopped restart policy, read-only filesystem and existing home volume; restart succeeded; independent local agent collection continued |
| Passive fleet observation / working agent, failure and recovery | core.py, probe.py, feed.py | Local and SSH collection plus private file transport tested; live publication resumed after source service recreation and both office sources returned online |

31 Python unit/integration tests, five Node UI tests, JavaScript syntax, strict OpenSpec validation and git diff checks pass. Independent adversarial review approved 59040d6 with no actionable findings after correcting replay ordering. No programme register applies.

The Personal service is a transient user systemd service on the laptop (not enabled across reboot). The Work service runs independently in the workstation WSL Docker environment. Publication uses authenticated SSH over the existing Tailscale route; dashboard HTTP remains loopback-only. Real configuration and activity are excluded from Git. Active palette delivery was verified against both live APIs.

Browser visual QA was unavailable. Layout claims have source and logic coverage, not rendered-browser evidence. Windows GUI access and Windows localhost forwarding are unavailable from the managed container, so physical-screen launch remains a documented user action: open http://localhost:8789 and enter full screen, or use the supplied PowerShell kiosk launcher. GPU metrics remain unavailable when NVIDIA tools are inaccessible. Sampled activity is not a complete tool-call log.

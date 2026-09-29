# Notion source investigation

The browser-extension implementation and subsequent setup card were rejected:
they did not meet the user's existing authentication model. All runtime, browser
extension and UI changes from that experiment are removed from the branch.
Earlier fixture/review successes did not establish acceptance of that design.

## Verified source facts

- Existing Codex collection uses bounded authenticated app-server account RPCs.
- Installed `ntn --help` exposes no allowance command.
- `ntn api ls` has no personal allowance/usage endpoint. Its one credit-related
  match is PATCH /v1/agents/{agent_id}/credit_limit, unrelated to this task.
- Earlier CLI-authenticated web allowance request returned HTTP 401.
- The official CLI installation and login are retained. No credential files were
  read, changed or exported during this correction.

## Status

Blocked on a suitable provider-owned account source. No live Notion allowance
integration is delivered. Canonical spec sync and archive are not appropriate.
Independent review confirmed runtime, UI, tests, CI and AGENTS.md match
origin/main exactly. The baseline release build and plugin manifest validation
passed. Restored the installed runtime and six presentation/support files;
verified payload equality, private file content/inodes and owner preservation.
No native bridge manifest or receipt existed. Restarted Omarchy and verified the
actual popover has no setup card; diagnostics report connected. CLI installation
and authentication were not changed.

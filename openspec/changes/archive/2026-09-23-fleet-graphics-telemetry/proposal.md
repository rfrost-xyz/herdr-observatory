# Proposal

## Why

Both configured fleet hosts report an unavailable graphics value. The collector only tries `nvidia-smi`, while iapetus uses Intel Xe and the ws-255 Observatory container has no WSL GPU device or driver mount.

## What Changes

- Collect Intel Xe whole-device engine utilisation through an isolated PMU monitor that publishes only a bounded aggregate to the dashboard.
- Provide opt-in Compose graphics integrations for iapetus and ws-255. Keep the dashboard's user, capability and network boundaries.
- Let the Personal display obtain ws-255 graphics from its loopback Work dashboard when the SSH probe has no direct GPU device.
- Show the source and scope of a measured graphics percentage, keeping absent or invalid readings unavailable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `activity-dashboard`: Define truthful graphics collection and its scope on Intel Xe and WSL NVIDIA hosts.
- `office-display`: Explain the source of the fleet graphics gauge when a reading is available.

## Impact

`observatory/probe.py`, an isolated Intel sampler, metric validation, fleet rendering, two optional Compose overrides, deployment instructions and focused tests. The Intel helper uses `CAP_PERFMON`, no host process or home mount and no network; the dashboard reads only its aggregate sample. The WSL override exposes only `/dev/dxg` and the read-only WSL driver libraries, without a Docker socket. The remote probe's optional loopback read retains only the configured host's graphics values.

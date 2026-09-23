# Proposal

## Why

Both configured fleet hosts report an unavailable graphics value. The collector only tries `nvidia-smi`, while iapetus uses Intel Xe and the ws-255 Observatory container has no WSL GPU device or driver mount.

## What Changes

- Collect Intel Xe client engine utilisation from read-only DRM fdinfo when a host `/proc` view is explicitly mounted.
- Provide opt-in Compose GPU mounts for iapetus and ws-255. Retain the existing container user, capability and network boundaries.
- Show the source and scope of a measured graphics percentage, keeping absent or invalid readings unavailable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `activity-dashboard`: Define truthful graphics collection and its scope on Intel Xe and WSL NVIDIA hosts.
- `office-display`: Explain the source of the fleet graphics gauge when a reading is available.

## Impact

`observatory/probe.py`, metric validation, fleet rendering, two optional Compose overrides, deployment instructions and focused tests. The Intel override exposes read-only host process statistics to the existing local service user. The WSL override exposes only `/dev/dxg` and the read-only WSL driver libraries, without a Docker socket.

# Spec Delta

## MODIFIED Requirements

### Requirement: Honest telemetry
The dashboard SHALL show sampled status transitions and available resource metrics with timestamps and collection scope; unavailable metrics SHALL remain unavailable. Graphics utilisation SHALL identify whether it measures visible NVIDIA devices or the busiest Intel Xe device engine.

#### Scenario: Missing GPU and stale browser
- **WHEN** GPU telemetry is unavailable or the browser loses its connection
- **THEN** GPU values are marked unavailable and stale browser activity stops appearing live.

#### Scenario: WSL NVIDIA graphics
- **WHEN** the NVIDIA GPU device and read-only driver tools are explicitly exposed to the collector
- **THEN** a valid `nvidia-smi` reading produces utilisation for visible NVIDIA devices, with the existing VRAM totals.

#### Scenario: Remote fleet view of WSL graphics
- **WHEN** an SSH collector cannot access the WSL GPU device but is configured to read the remote dashboard's loopback state
- **THEN** it uses only the matching host's fresh, valid NVIDIA aggregate, without forwarding the other dashboard's agent data or invoking Docker.

#### Scenario: Intel Xe device graphics
- **WHEN** an isolated monitor publishes a fresh aggregate of Intel Xe device counters
- **THEN** the dashboard shows measured utilisation for the busiest available engine, labelled as device scope, without inventing dedicated VRAM or exposing process data.

#### Scenario: Invalid or incomplete graphics counters
- **WHEN** a graphics source is missing, unreadable, malformed, resets, or has no matching interval
- **THEN** its percentage remains unavailable; other host metrics and agents remain available.

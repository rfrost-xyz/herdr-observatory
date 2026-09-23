# Spec Delta

## MODIFIED Requirements

### Requirement: Honest telemetry
The dashboard SHALL show sampled status transitions and available resource metrics with timestamps and collection scope; unavailable metrics SHALL remain unavailable. Graphics utilisation SHALL identify whether it measures visible NVIDIA devices or readable Intel Xe user clients.

#### Scenario: Missing GPU and stale browser
- **WHEN** GPU telemetry is unavailable or the browser loses its connection
- **THEN** GPU values are marked unavailable and stale browser activity stops appearing live.

#### Scenario: WSL NVIDIA graphics
- **WHEN** the NVIDIA GPU device and read-only driver tools are explicitly exposed to the collector
- **THEN** a valid `nvidia-smi` reading produces utilisation for visible NVIDIA devices, with the existing VRAM totals.

#### Scenario: Intel Xe user-client graphics
- **WHEN** a read-only host process view exposes Intel Xe client counters to the collector
- **THEN** the dashboard shows measured utilisation for the busiest readable engine, labelled as user-client scope, without presenting it as whole-device utilisation or inventing dedicated VRAM.

#### Scenario: Invalid or incomplete graphics counters
- **WHEN** a graphics source is missing, unreadable, malformed, resets, or has no matching interval
- **THEN** its percentage remains unavailable; other host metrics and agents remain available.

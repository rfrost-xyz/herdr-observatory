# Design

## Context

The probe runs inside each profile container. `nvidia-smi` is the only existing GPU source. The ws-255 WSL host already exposes `/dev/dxg` and `/usr/lib/wsl` to its separate Ollama container, and a non-root process there can query the card. On iapetus the Intel `xe` driver exposes per-client DRM cycles through `/proc/<pid>/fdinfo`; its aggregate PMU requires a capability the dashboard does not hold.

## Goals / Non-Goals

**Goals:** Preserve the current unprivileged container model and make source scope visible alongside actual measurements.

**Non-Goals:** Whole-device Intel utilisation, native Windows system totals and broader container access to Docker or the host home.

## Decisions

- Add optional Compose overrides per platform. WSL mounts the GPU device and read-only driver directory, and extends executable/library search paths. Intel mounts host `/proc` read-only. The base service still starts without either integration.
- Sample Intel Xe `drm-cycles-*` and `drm-total-cycles-*` twice inside a single probe, 200 ms apart. Deduplicate client IDs within a PCI device, compare only matching clients, divide summed busy deltas by the engine interval and capacity, and report the busiest readable engine. This measures accessible user clients; the UI names that scope. Sampling inside the probe exports only an aggregate, not process identifiers or fdinfo contents.
- Keep NVIDIA as the preferred source if it yields a valid reading; fall back to Intel Xe when available. Invalid data stays unavailable. Preserve a source enum through sanitisation for the UI description.

## Risks / Trade-offs

- A read-only `/proc` mount allows the service user to inspect the host processes that user can already inspect. The override is opt-in and the probe reads only DRM fdinfo, emits no process identity, and keeps the existing UID and capability restrictions.
- Intel sampling adds about 200 ms to the five-second collector interval. Missing clients or a counter reset produce unavailable rather than zero.
- Client counters omit work by processes the service UID cannot read, so the UI must label the result as user-client scope.

## Migration Plan

Build a versioned image. Enable the Intel override only on iapetus and the WSL override only on ws-255. Recreate one service at a time, verify source-labelled readings through the local API and browser, and retain the previous image and Compose configuration for rollback.

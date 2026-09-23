# Design

## Context

The dashboard runs as an unprivileged container. `nvidia-smi` is its only existing GPU source. The ws-255 WSL host already exposes `/dev/dxg` and `/usr/lib/wsl` to a separate Ollama container, where a non-root process can query the card. On iapetus, Intel Xe exposes aggregate engine counters through perf events, which need `CAP_PERFMON` under the current kernel policy.

## Goals / Non-Goals

**Goals:** Preserve the dashboard's unprivileged container boundary and make the source of each graphics reading clear.

**Non-Goals:** Native Windows system totals, client/process reporting and a mount of the host process filesystem.

## Decisions

- The WSL override mounts only `/dev/dxg` and the read-only driver directory, with executable/library search paths. The existing dashboard UID and capability set stay intact.
- The Intel override adds a dedicated monitor container using the same versioned image. It has no network, no host filesystem mount, a read-only root, dropped capabilities except `CAP_PERFMON`, and a small shared volume. The dashboard mounts that volume read-only. This isolates the necessary capability from Herdr and SSH credentials. A whole-host `/proc` mount was considered and rejected after review because process `root`, `cwd` and `fd` links could expose host files.
- The monitor opens available Xe `engine-active-ticks` and `engine-total-ticks` PMU pairs. It calculates the busiest engine over a two-second interval and atomically publishes only timestamp, percentage and source. The dashboard accepts fresh, finite, bounded samples and never exports individual engine or process identity.
- The dashboard tries a valid NVIDIA sample first, then the optional Intel aggregate. Invalid or expired measurements stay unavailable. Source names survive metric sanitisation for the fleet gauge description.

## Risks / Trade-offs

- The Intel helper runs as root inside its container to hold `CAP_PERFMON`. No host process path, home path, credentials or network is mounted into it, and it can write only the aggregate volume.
- The busiest engine is a useful activity gauge rather than an average of all engine capacities. The UI names that scope. An idle engine can correctly report zero; missing counters cannot.
- Perf event encoding is x86-64 and Xe-specific. Unsupported hardware or kernel access keeps graphics unavailable without interrupting other metrics.

## Migration Plan

Build a versioned image. Enable the Intel override only on iapetus and the WSL override only on ws-255. Recreate one service at a time, verify source-labelled readings through the local API and browser, and retain the previous image and Compose configuration for rollback.

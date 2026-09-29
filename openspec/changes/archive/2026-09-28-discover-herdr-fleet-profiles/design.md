# Design

## Context

The native runtime currently loads configuration once and owns fixed host/account worker threads. Navigation separately resolves saved Herdr profiles by host label or id. Peer probes require an exact configured host id. Existing deployed peers and private mappings must survive this incremental change.

## Goals / Non-Goals

**Goals:** bounded automatic discovery; durable profile identity; prompt cancellation and removal; exact navigation; unchanged telemetry/privacy/allowance semantics.

**Non-goals:** remote provisioning during discovery, remote service changes, an onboarding wizard, manual refresh controls, new metrics or UI redesign.

## Decisions

1. An explicit `fleet_discovery` switch enables a ten-second start-to-start bounded `herdr machine list --json` poll. Each command has a five-second deadline and 256 KiB output limit. Inventory accepts at most 64 records including disabled profiles, with at most 16 effective hosts and distinct allowance targets. Account mappings remain limited to four. Installation documentation enables discovery for fleet use. Existing static configurations remain supported when disabled. A valid bounded complete profile list is authoritative; failed/malformed/oversized lists retain the last accepted set and expose discovery unavailable separately from host transport health.
2. Private host overrides bind through `profile_id`. The override's existing `id` stays the wire/checkpoint identity for deployed peers. New profiles use the stable saved profile id, never their mutable label. Active profile target and session supersede the bound override's route; project roots and unrelated selectors remain. Disabled/deleted bound overrides do not become static hosts, including after restart. Unbound static hosts remain explicit, with ambiguous collisions rejected instead of duplicated.
3. Each effective route has a generation. Reconciliation cancels and joins retired workers, removes their active samples/cursors, and rejects queued events with an old generation. Target/session changes discard old route data even when the visible host id stays the same. Owner EOF cancels discovery and all worker processes. Checkpoint persistence remains owner guarded.
4. Allowance collection derives enabled peer targets from the effective fleet and retains explicit source/profile bindings. A removed or disabled profile stops its thread probes and removes its allowance source; another enabled profile or explicit source for the same target may keep that account worker alive. Account mappings remain explicit, returned rows are revalidated and deduplicated by account identity, and activity never gates account reads. Account generations bind to targets because account RPCs have no session selector. Removing or changing a target invalidates its remote account results; label/session-only changes and identical inventories preserve account workers and cadence.
5. Navigation carries the exact effective profile identity and route that produced each displayed thread. A click validates the current saved enabled profile and fails closed if target/session changed; no label fallback can reroute an old thread. Existing static local navigation remains supported.
6. A static bounded SSH peer-presence response distinguishes an absent executable from unreachable transport. The popover projects `setup_needed` as “Setup needed”. No installer is invoked. Discovery health is concise and does not renew measurement timestamps.

## Risks / Trade-offs

- Mutable or duplicate labels → stable profile ids and exact route validation.
- Deletion during a slow SSH request → per-generation cancellation, bounded process-group cleanup and late-event rejection.
- Herdr discovery outage → keep last accepted inventory, visibly mark discovery unavailable, never infer deletion.
- Existing peer id compatibility → retain explicitly bound override id; new peer uses saved profile id.
- Account startup cost → retain existing account cadence and deduplicate targets; no probe per thread or discovery tick.

## Migration Plan

Root adds private `profile_id` bindings to the existing host and allowance source, enables `fleet_discovery`, and provisions the new peer explicitly using its saved profile id and current session. Source workers do not edit installed state or remote hosts. After review, root installs the staged runtime/UI files preserving marker inode, account identities and preferences. Live acceptance covers both existing machines and the new machine, setup state, navigation, allowances, restart and no Python/service dependency. Rollback restores the previous reviewed payload and private config; peer removal remains receipt bound and separate. Archive follows root's installed acceptance.

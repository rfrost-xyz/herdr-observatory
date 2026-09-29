# Design

## Context

Anton runs a bundled Python child from Quickshell. The current collector retains the dashboard profile; native completion parsing is incremental but called from hooks. The popover uses stable host/pane identities, source timestamps and local privacy preferences. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:** dependable signals, bounded ownership, compatible account/source expansion and motion which does not interrupt interaction.

**Non-Goals:** Rust rewrite, Claude collection, independent daemons, new remote services, transcript publication or agent control.

## Decisions

1. Keep Quickshell/Qt Quick and Python. Split presentation components and isolate the native collection profile rather than introduce an unmeasured rewrite.
2. Add compatible native outcome counts beside existing total/done/source time. Validate partitions and keep older total/done-only sources usable without inferring their remaining outcomes. Herdr remains authority for parent status.
3. Native Codex hooks register bounded session/pane association; a collector-owned reader follows validated local records independently, and an ephemeral SSH probe follows remote records with locally retained opaque cursors and no remote installation. Retain legacy hook enrichment for unchanged web/remote installations. Never expose transcript paths/content/child identity.
4. Use one signed allowance gap with neutral on-pace, green reserve and red deficit. Keep a stable expected-position marker and concise remaining/expected hover. Avoid relative division by nearly zero reset time.
5. Native account rows gain provider/account identity while legacy labels and private mappings remain supported. Identity emails remain local to the plugin. New display labels never change Work disclosure categories.
6. Match motion and acknowledgement to stable thread identity. Hydration, reconnect, reordering, filtering and collapse do not create entrance events. Expand/collapse stays instant. Decorative motion is visible-only and honours reduced motion.
7. Stop unused Anton metrics/history and suppress duplicate full snapshots, retaining a bounded heartbeat inside the six-second UI transport deadline. Heartbeat time never changes measurement timestamps. Collect metrics for an explicitly published host because its remote dashboard still consumes them; retain only the previous measured sample needed for rates.
8. Large native output is common. Validate oversized JSON envelopes incrementally with bounded grammar state and fixed type masks; preserve compaction coverage only for recognised unambiguous record types, never by trusting a prefix or storing raw content. Incomplete, malformed and ambiguous lifecycle records remain unavailable.

## Risks / Trade-offs

- Incomplete native history or unsupported records: keep counts unavailable until bounded complete evidence exists.
- Old remote adapters: preserve their total/done and account contracts; Anton independently enriches supported remote native records over the existing read-only SSH probe without updating the web service.
- Closed popover: continue essential state collection but stop decorative work.
- User navigation failure: acknowledge completion only after successful exact-thread navigation.
- Optional office publication/music: keep its existing explicit configuration and filtered downstream behaviour.

## Migration Plan

Build the private runtime bundle, run repository and focused UI checks, then replace only owned plugin files and reinstall owned native hooks using the existing installer. Preserve private config/identities and preferences. Verify one owner-bound runtime, both sources, account data and no local listener. Uninstall removes only owned new state/components; rollback uses the retained prior plugin bundle/config. Root owns live installation and acceptance.

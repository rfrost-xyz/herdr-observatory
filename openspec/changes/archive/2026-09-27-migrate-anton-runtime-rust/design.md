# Design

## Context

The current plugin streams sanitised Observatory snapshots from a resident Python collector. Local polls run every two seconds, configured remote polls usually every five; publication, allowance and optional music workers share that lifetime. Private replay checkpoints have an inode lease, strict ownership checks and retirement exclusion. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:** Move orchestration and current-state ownership into a typed Rust runtime while proving existing privacy and behavioural parity. Measure the whole local process family and distinguish synthetic, local transport and remote execution scopes.

**Non-Goals:** Rewriting QML, the web application, all transcript parser logic, navigation, or remote services. No intrinsics, assembly, daemon, listener, new autostart, remote installed agent or runtime package download.

## Decisions

1. A small Rust binary uses standard threads, bounded channels and pinned serde/serde_json/libc dependencies. It owns host and allowance scheduling, typed host state, stable status-since values, rates, feed replay/expiry, publication status, snapshot emission and heartbeat. The accepted JSON snapshot contract remains unchanged. This avoids a Rust launcher around the unchanged Python Observatory loop.
2. A private version-one JSON request adapter retains the existing validated domain logic. Bounded requests and responses carry only operations needed for init, sample, local allowance projection, remote allowance refresh, publish and checkpoint persistence. Rust supervises process groups, kills on deadlines/owner EOF, and never exposes child stderr. Native domain subprocesses also use an opt-in nonblocking runner with a four-MiB stdout limit enforced while reading, discarded stderr, concurrent bounded stdin and the existing operation deadlines. The shipped remote probe carries the same opt-in runner; legacy Python/web defaults remain unchanged. The adapter never starts Observatory or a collection scheduler. Measured interpreter startup first justified retained adapters, but their duplicate interpreters increased memory. The accepted consolidation uses one request-driven Python domain broker with at most twenty explicit concurrent operations. Rust owns unique request IDs, a bounded pending map, deadlines and exact response correlation. The broker keeps only bounded parser memoisation and receives authoritative cursors on every request. Invalid, duplicate, unknown or timed-out responses discard the broker process group and pending operations; subsequent scheduled calls use a fresh process. Init and final checkpoint flush remain short-lived operations. This preserves existing domain I/O code rather than duplicating Herdr transport while Rust retains schedule and state authority.
3. The adapter supplies sanitised normalised samples and opaque cursor state. Rust retains all-host cursors; checkpoint writes reuse the initial inode lease and writer metadata, preserve meaningful-write throttling and the existing retirement guard. Native source timestamps remain original and no heartbeat renews measurements. Specialised optional Music keeps its existing 15 Hz IPC/publication thread inside the shared broker. Rust issues an idempotent music-start request every two seconds to restore it after broker replacement. Broker closure stops Music and its SSH publisher; no second resident Python interpreter is used.
4. QML launches the bundled binary. Python runtime.zip and runtime.py remain for hook reporting, identity/allowance operations and rollback. The binary is built before installation with a locked dependency graph; the installed plugin needs no Rust toolchain. Explicit installed-file lists retain clean uninstall.
5. Keep one request in flight per configured host and concurrent broker execution so slow SSH cannot queue ahead of local state. Remote allowance refresh has its own worker; local/cache projection remains independent. A protocol/deadline failure resets the shared domain process and fails its outstanding requests explicitly. Bound input/output to four MiB, desktop snapshots to two MiB, outstanding worker events and process deadlines. Preserve source failure/connecting/empty distinctions.

## Risks / Trade-offs

- Python process startup can outweigh saved snapshot cost. Compare full adapter transport benchmarks and live family CPU/RSS, document retained Python honestly and adjust adapter lifetime when warranted.
- Cross-language state changes can weaken privacy or freshness. Reuse mature validation at adapter boundaries and exercise synthetic Work/Personal exclusions, invalid data, old feeds, native timing, subagents and allowance reset expiry.
- Concurrent checkpoint writes can lose host state. Rust owns the complete merged cursor map and a single checkpoint writer, retaining the original lease and throttle state across requests.
- A shared-broker reset briefly interrupts music alongside domain operations; the idempotent Rust request restores the music worker on the next two-second retry and receiver freshness expires old frames.
- Optional publication/music are easily overlooked. Keep explicit adapters and lifecycle tests; installed acceptance verifies authorised publication without modifying its receiver.

## Migration Plan

Capture the existing installed and synthetic baseline before implementation. Build and verify the native runtime against synthetic old/new snapshots and failure fixtures. Run fmt, clippy, Rust tests, repository checks and independent adversarial review. Root installs only after source acceptance, preserving private config and a rollback copy, then verifies source parity, runtime lifetime and the real popover. Archive only after installed acceptance. No Git/forge actions are authorised in this local development turn.

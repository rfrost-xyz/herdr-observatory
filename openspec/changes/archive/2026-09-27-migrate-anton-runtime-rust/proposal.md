# Proposal

## Why

Anton now has a substantial native collector with lifecycle, freshness and private replay state. Move its orchestration and state model into idiomatic Rust while preserving the accepted popover and existing data contracts, measuring the complete runtime rather than assuming language alone improves performance.

## What Changes

- Introduce a plugin-owned Rust process for scheduling, bounded child supervision, typed host/allowance snapshot state, deduplication and owner-pipe lifetime.
- Retain specialised Python domain adapters for the existing validated probes, transcript parsing, checkpoints, allowance RPC/cache handling and Work projection, plus the optional music bridge. These do not run the Python Observatory collector.
- Bundle a compiled executable alongside the existing reporter runtime and keep removal ownership explicit.
- Add synthetic parity, process-failure and comparative resource evidence.

## Capabilities

### New Capabilities

None. This is an implementation migration with behaviour parity.

### Modified Capabilities

None. Existing native collection, disclosure, freshness, lifecycle and reversible-installation requirements remain authoritative. `skip_specs: true` avoids inventing a behavioural delta.

## Impact

Native plugin source, build/install ownership, runtime tests and developer documentation. The web application and remote installed services remain unchanged. QML keeps its visual design and private JSON snapshot contract; only its launched executable changes. Cargo is a build prerequisite, not a runtime prerequisite. No Git publication or deployment is part of source implementation.

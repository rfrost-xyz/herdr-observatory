# Evidence

Synthetic fixtures only. No private transcripts, ids, paths or account data. The corpus check records aggregate counts only.

## Baseline

The unchanged runtime at `3ed6c11` is byte-identical in source to `2d2be90`, whose baseline was recorded in the archived research change:

- release binary sha256 `74f50d695c89e4cc01198c77df9d31ab9b1230e8e5948a6a73dceb569d76931d`;
- runtime CPU 0.031 s over 30 s;
- peak RSS 4,100 KiB;
- mean snapshot 27,674.6 bytes;
- all suites green.

Intermittent test: in CI, `allowances::tests::account_rpc_flood_is_bounded_and_cancellation_prevents_spawn` failed at `allowances.rs:749` (`create_dir` AlreadyExists) on 2 of 12 runs on the research branch. The cause is fixture directories named by pid and timestamp only. `0b9669e` adds a per-site sequence, and five consecutive local full runs passed. A deterministic regression test is not possible, because the collision needs two equal clock reads.

Baseline Claude metric (`5d93863`, old binary): 0 of 4 local and 0 of 4 peer Claude agents had native telemetry.

## Implementation

The implementation was delivered by a staged workflow: A1 to A4 (core), B1 to B3 (integration), then verification. A first single-agent attempt stalled while writing the whole module in one output. It was stopped, and its usable work was recommitted as `a45c121` and `607937d`.

## After (HEAD `637f731` plus docs)

Gates:

| Gate | Result |
|---|---|
| fmt, clippy `-D warnings` (locked, offline, all targets) | clean |
| `cargo test`, 3 runs | lib 130, main 11, native_navigation 6, native_process 29, all passing |
| `node --test` (State, Pi hooks, distribution) | 88 of 88 |
| `run-qml.sh` | 95 passed, 0 failed |
| `run-qmllint.sh` | no warnings outside Panel.qml |
| `run-shell-harness.sh` (installed Omarchy modules) | failures 0, 4 rows, `--open-thread` 0 |

Release binary sha256: `293cd9f0318a27738a15dfc7d50f3931d5218b00787f5937bc14b48e8094e62e`.

Measurement with `--repeat 3`, 2 synthetic hosts and 32 Codex/Pi agents over 30 s. Both columns were measured the same day on the same machine. The earlier 0.031 s baseline came from a quieter machine state and is not comparable.

| Metric | origin/main | HEAD |
|---|---|---|
| Runtime CPU | 0.117 s | 0.118 s |
| Peak RSS | 7,564 KiB | 4,588 KiB (noisy; runs 4,536 to 7,736) |
| Mean snapshot size | 27,677.5 B | 27,675.5 B |

Claude probe (4 Claude agents per host, 30 s):

| Variant | Agents with native telemetry (local/peer) | First data | CPU | Peak RSS |
|---|---|---|---|---|
| origin/main | 0/0 | n/a | 0.068 s | 6,968 KiB |
| HEAD, 25 KB transcripts | 4/4 | 32/41 ms | 0.160 s | 7,860 KiB |
| HEAD, 6 MB transcripts | 4/4 | 100/128 ms | 0.605 s | 14,652 KiB |

All 8 HEAD agents carry the five totals, `context`, `compactions`, `subagent_total` and turn timing, with source `claude-transcript`.

## Counts-only corpus check

`cargo run --release --example claude_corpus` on the local projects root prints aggregates only, and nothing private is recorded.

**Files and binding.**
- 18 main files, 18 with identity verified and 18 caught up. The bounded replay agrees with a line-by-line shadow replay on all 18.
- Predecessor check:
  - 13 clear;
  - 3 clear but growing (not cached);
  - 2 unknown, where a newer file's first `session_id` names the bound id, so these are correctly refused as `/clear` predecessors.
- No scan hit its bound.

**Usage.**
- Usage is known for 12 files. The other 6 have no counted assistant record.
- Totals and last-response values match those counts: 12 known, 6 not.
- Reopened groups: 0. Identity mismatches: 0.
- Deduplication merged 1,488 of 2,617 assistant lines into 1,129 groups.

**Children, compactions and turns.**
- Children are valid in 18 of 18 files, with 17 children in total.
- Compactions are known for 12 files, and 0 were seen.
- Turn coverage is valid in 17 of 18 files: 161 finished turns, 21,702 s in total. The one unknown file is a silent end, two `end_turn` responses with no `turn_duration`, which fails closed by design.

**Oversized lines.** 23 lines over 64 KiB, all classified.

This change publishes for 10 of 18 files.

## Test reliability

- **Allowances test.** The intermittent `allowances.rs:749` failure is fixed in `0b9669e`.
- **Checkpoint test.** `native::tests::checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation` is a pre-existing test, and it failed once during staging. The root cause, found in review round 1, is the lock: `Checkpoints::new` and `persist` took it with `flock(LOCK_EX|LOCK_NB)` and zero wait.
  - A process spawned concurrently by another test holds the lock's open file description until it calls exec.
  - That left the lease `None`, and `loaded.reconcile(&empty).unwrap()` panicked at `src/native.rs:1704:34`. The `std::fs::read` was not the failing call.
  - A reviewer repro failed 97 of 300 times under concurrent spawns. `893ab77` adds a 250 ms bounded wait.
  - The new test `checkpoint_lock_waits_out_concurrent_process_spawns` failed 20 of 20 runs before the fix and passed 20 of 20 after.

## Traceability

| Requirement / scenario | Implementation | Verification | Commit |
|---|---|---|---|
| Native Claude Code transcript replay: bound session | `claude.rs` discover, open_session, replay, `Row::usage`; `native.rs` dispatch | claude.rs unit fixtures; `native_process` peer probe; bench Claude probe 4/4 | `1639708`, `1984ce7`, `d21194f` |
| Split and interleaved responses | claude.rs dedup groups and closed ring | dedup fixtures; corpus 1,488 lines merged | `6ce954a` |
| Ambiguous, forked or predecessor binding | `discover` (`Ambiguous`, `Truncated`), `predecessor`, `forked` | discovery, predecessor and fork fixtures; corpus 2 predecessors refused | `1639708`, `d21194f` |
| Unsupported record shapes | `claude/classifier.rs` and the coverage table | classifier fixtures; oversized replay across `TAIL` | `499ca8c`, `1389b7a`, `39b24b2` |
| Hook-time structured usage enrichment (Claude Code reader) | as above | as above | as above |
| Scoped cumulative metrics: intermittent Claude replay | native.rs local retention; main.rs peer retention | native and main unit tests; `native_process` retention cases | `010327a`, `ed7764b`, `602d102` |
| Plugin-owned native enrichment: peers | v1 envelope unchanged; `State::sample` revalidation | `native_process` peer and old-cursor tests | `ed7764b`, `602d102` |
| Bounded native outcome summaries | claude.rs children and the unknown bucket | children fixtures; corpus 18/18 valid | `54d5122` |
| Native turn wall-clock summaries | claude.rs turns with turns.rs helpers | turn fixtures; corpus 17/18 valid | `a45c121`, `a2b3363` |
| Private native replay checkpoints | `Cursor.claude` block and the hashed key; Codex-first bounds; per-row load; lock wait | checkpoint, stripped-block and rebinding tests; Codex golden tests; bound, unreadable-row and lock tests | `d9d2ee6`, `d21194f`, `c38efe7`, `faab8bf`, `893ab77` |

## Review round 1 and remediation

The independent four-lens review of `ef2ddc5` found 3 blocking, 9 non-blocking and 4 nit findings. All are fixed, each with a regression test that failed before its fix.

**Blocking:**
1. **The header record was not replayed** (`2412f2a`).
2. **A killed turn with only enqueued input absorbed the idle gap.** Joins now need dequeue or remove evidence (`aa9ed0a`, `4b5536d`).
3. **Claude rows evicted Codex rows** from the shared cursor and checkpoint bounds. Codex is now charged first (`c38efe7`), and ring hashes shrank to 64 bits (`ac47bc9`). A full block went from 2,702 to 1,118 bytes.

**Other fixes:**
- a trigger at Unix second 0 (`e0e2257`);
- an ambiguous trigger no longer publishes a truncated turn (`d3c5940`);
- the classifier no longer persists non-schema key bytes (`ec6da8e`);
- `unknown` status on Claude rows only (`abb247d`);
- the checkpoint lock wait (`893ab77`);
- a truncated predecessor scan is rescanned (`2ebf832`);
- a deadline skip re-emits the retained sample (`bf04583`);
- peer retention needs rows in both request and response (`f1d3a36`);
- checkpoint rows load individually (`faab8bf`);
- the bench Claude agents are opt-in (`d5bfd3a`);
- old-local cost is measurable (`c162861`);
- evidence corrections.

**Turn timing changes:** the queue and ambiguity fixes removed 4,385 s of turn time that a pending-start join would have added, and one 136 s turn after an ambiguous trigger (169 to 168 turns). The corpus check aggregates for coverage (17/1), publishable (10/8) and usage known (12/6) are unchanged.

### Measurements after remediation (`c162861`, release sha256 `09c3b037a6b3548c816673d0e0c8853c2601c6da4d592b08da0dae09045f9af5`)

The two binaries were measured in the same session, one after the other, with load average about 2 to 2.6. The baseline is `2d2be90`, sha256 `74f50d69…6931d`.

Runtime (`--repeat 3`, 2 hosts with 32 agents each, 30 s):

| Metric | HEAD | Baseline |
|---|---|---|
| CPU | 0.088 s | 0.087 s |
| Peak RSS | 4,480 KiB | 4,480 KiB |
| Snapshots | 10 | 10 |
| Mean snapshot size | 27,677.8 B | 27,675.3 B |
| Herdr RPCs (local / remote) | 15 / 6 | 15 / 6 |

Claude probe (4 agents per host, one 30 s run per variant):

| Variant | Agents with telemetry (local / peer) | CPU | Peak RSS |
|---|---|---|---|
| HEAD, standard | 4/4, 4/4 | 0.132 s | 8,224 KiB |
| HEAD, large | 4/4, 4/4 | 0.559 s | 14,576 KiB |
| Baseline | 0/0 | 0.05 s | 6.9 MiB |

Old local with a new peer (`--claude-old-local`, harness `c162861`, local `74f50d69…`, peer `09c3b037…`, 4 agents per host, one 30 s window per variant):

| Variant | Native agents (local / peer) | First peer data | CPU | Peak RSS |
|---|---|---|---|---|
| Standard, new local | 4/4 | 28 ms | 0.110 s | 4,964 KiB |
| Large, new local | 4/4 | 121 ms | 0.550 s | 13,824 KiB |
| Standard, old local | 0/4 | 37 ms | 0.095 s | 7,384 KiB |
| Large, old local | 0/4 | 150 ms | 0.866 s | 11,720 KiB |

**Bench default:** `tests/bench_anton_native.mjs` now defaults to 0 Claude agents. Bench figures recorded earlier with the old default of 4 are therefore not default-run figures.

**Gates at `faab8bf`:**

| Gate | Result |
|---|---|
| fmt, clippy | clean |
| `cargo test` | lib 142, main 12, native_navigation 6, native_process 30 |
| `node --test` | 88 of 88 |
| `run-qml.sh` | 95 of 95 |
| qmllint | clean outside Panel.qml |
| Shell harness | 0 failures |

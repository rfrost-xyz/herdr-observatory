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

- **Allowances test.** The intermittent `allowances.rs:749` `create_dir` collision is fixed in `0b9669e`. A second allowances flake ("Command unavailable") is ETXTBSY: a sibling test's spawned child inherited the write descriptor of a freshly written script. `1b2a36b` and `4234e93` write executable fixtures from a child process. The regression tests failed 10 of 10 before the fix and passed 300 of 300 after.
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

The independent four-lens review of `ef2ddc5` found 3 blocking, 9 non-blocking and 4 nit findings. All are fixed. Each code fix has a regression test that failed before its fix; harness and evidence fixes have none.

**Blocking:**
1. **The header record was not replayed** (`2412f2a`).
2. **A killed turn with only enqueued input absorbed the idle gap.** Joins now need dequeue or remove evidence (`aa9ed0a`, `4b5536d`).
3. **Claude rows evicted Codex rows** from the shared cursor and checkpoint bounds. Codex is now charged first (`c38efe7`), and ring hashes shrank to 64 bits (`ac47bc9`). A block with a full ring went from 2,702 to 1,118 bytes. That is not the worst case: a row with 512 finished turns and 128 children is about 44 KB.

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

Corpus figures in this file come from different runs of a live corpus. Absolute counts drift between runs, while the stated ratios reproduce.

## Review round 2 and remediation

The four-lens review of `d0eb92a` found 5 blocking and 6 other findings. All are fixed, and each code fix has a regression test that failed before it.

**Blocking:**
1. **Rejected turn start.** A rejected start left an unresumable state, so a file larger than `TAIL` never caught up. A lost turn was also published truncated. Fixed in `ee2fe7f`: `ambiguous` is now set wherever a turn may still be running, and a fuzz test of 4,500 cases checks that every reached state passes the resume gate.
2. **Identity mismatch was not sticky.** Fixed in `fd8015d` with the `foreign` flag.
3. **Lost-turn paths published truncated turns.** Also fixed in `ee2fe7f`.
4. **Peer revalidation had zero clock-skew tolerance,** which dropped Codex peer telemetry too. It now allows 1 s (`6be49b5`); round 3 widened this to 1 s plus the 6 s snapshot gap from the later of now and `sampled_at` (`c3b2a96`).
5. **A sample rejected by revalidation re-emitted stale retained values.** Fixed in `1b154b4`.

**Other:**
- presence-only classifier fields of any length (`61be52d`);
- a four-counter sum above 2^53 (`59d6a66`);
- Claude rows shrink before being dropped at the byte bound (`8e81779`);
- the doc comment is back in place (`2f74830`);
- ETXTBSY test fixtures (`1b2a36b`, `4234e93`);
- evidence wording.

The corpus aggregates did not change.

**Known residual:** `allowances::receive` takes its lock with zero wait. This is pre-existing, outside this change, and was not seen failing.

### Verification at `4234e93` (release sha256 `16807ed38fbc88661e4e99e1a320ce04dd0d21a217ffa6a4aa61c28119ee1374`)

| Gate | Result |
|---|---|
| fmt, clippy | clean |
| `cargo test`, 3 runs | lib 153, main 14, native_navigation 6, native_process 31 |
| `node --test` | 88 of 88 |
| QML | 95 of 95 |
| qmllint | clean outside Panel.qml |
| Shell harness | 0 failures |
| History | no wip, fixup or duplicate subjects |

Runtime window, alternating reruns in the same session:

| Rerun | HEAD | Baseline `2d2be90` |
|---|---|---|
| 1 | 0.035 s | 0.035 s |
| 2 | 0.034 s | 0.033 s |

- **Peak RSS:** both binaries range from about 4.3 to 7.5 MiB.
- **Snapshot size:** unchanged.
- **Claude probe:** standard and large variants are 4/4 local and 4/4 peer, with all fields, `context` and `turn_timing`.

## Review round 3 and remediation

The four-lens review of `699c4a7` found 3 blocking and 9 other findings. All are fixed, and each code fix has a regression test that failed before it.

**Blocking:**
1. **Silent end then queue.** A queue operation after a silent end let a prompt join the ended turn. Fixed by the `silent_end` flag (`734327c`).
2. **Lost trigger while idle.** A record lost while idle let a later turn be published truncated. Fixed by the `lost_idle` flag (`fd081c3`).
3. **Stale peer values after a foreign record.** A peer re-showed old values after a foreign record in an incomplete pass. Fixed in `a4e6ece`.

**Other:**
- image-only prompts are unknown (`e0373ed`);
- a lost `sessionId` is invalid for every kind (`4faacad`);
- unpaired surrogates are read the same at both sizes (`867774c`, `5ac4706`);
- a pass ends on a shrunk file (`77ab50b`, `d245fb0`);
- stronger fuzz invariants with an independent oracle (`dd31309`);
- Codex panes are enriched before Claude panes, so Claude replay can no longer starve Codex (`d19dcdf`, `e6a6c20`);
- a deadline inside rediscovery is a skip (`fab4ddd`);
- locks are released with `LOCK_UN` before close (`4b372b4`), which also covers the allowances receive and reporter locks;
- peer revalidation bound (`8a58937`, `c3b2a96`).

**Checkpoint flake mechanism.** The parent closed a locked descriptor without `LOCK_UN` while a sibling thread's spawned child still referenced the open file description. With zero wait the lock test failed 45, 53 and 41 of 300 runs without the guards, and 0 in 5 runs with them.

**Corpus.** Aggregates are unchanged (coverage 17/1, publishable 10/8, usage known 12/6). One unknown-turn attribution moved from a silent-end shape to a queue shape, and no line used the serde-rejection fallback.

**Residual (pre-existing, not in this diff).** `common::owner_guard` and `native::retire` also close locked descriptors without `LOCK_UN`. `retire` waits up to 3 s.

### Verification at `c3b2a96` (release sha256 `e45104ca7ce6165c19313db350e452cd9654c4803f9516b5693dbfd2813e9914`)

| Gate | Result |
|---|---|
| fmt, clippy | clean |
| `cargo test`, 3 runs | lib 165, main 14, native_navigation 6, native_process 32 |
| `node --test` | 88 of 88 |
| QML | 95 of 95 |
| qmllint | clean outside Panel.qml |
| Shell harness | 0 failures |
| History | clean |

Runtime CPU: 0.033 s for both binaries. Claude probe: 4/4 local and peer, standard and large.

### Peak RSS check

Peak RSS was rerun in alternating order, Codex/Pi runtime window only, `--repeat 3`, twice per binary:

| Run | HEAD windows (KiB) | Baseline windows (KiB) |
|---|---|---|
| 1 | 7,688 / 4,228 / 4,404 | 4,260 / 4,372 / 4,164 |
| 2 | 7,568 / 4,528 / 4,704 | 4,124 / 4,444 / 4,192 |

CPU was 0.034 to 0.035 s for both binaries.

**Interpretation.** The harness sums RSS over the runtime's process family, which includes the fake-SSH peer probes, and samples it every 50 ms. Values are two-valued: about 4.3 MiB, or about 7.6 MiB when a second `anton-runtime` process overlaps at the sampled instant. The baseline also reached that state in earlier sessions (7,280, 7,396 and 7,564 KiB). HEAD reaches it more often in its first window. Its steady-state windows are within 0.5 MiB of the baseline.

**Binary size.** The release binary grew from 1,853,520 to 2,093,472 bytes.

## Ground-truth turn fuzzing (before review round 4)

The new fuzzer (`0ac8792`) found five root causes that published wrong turn numbers. Violations counted against the final fuzzer by reverting one fix at a time (the counts overlap):

| Shape | Violations | Fix |
|---|---|---|
| A trigger replaces an unconfirmed start | 322 | replacing a pending start is ambiguous (`75012fd`) |
| Local command while a prompt is pending | 177 | clears the pending start and sets `lost_idle` |
| Input taken, then a kill, then the restarted session's prompt | 27 | a join needs the trigger stamped no later than the evidence (`1467ba3`) |
| A record lost while idle, then a dequeue or remove | 6 | ambiguous (`97e442a`) |
| A second record lost while idle | 2 | ambiguous (`42b3118`) |

After the fixes the fuzzer reports 0 violations. Each fix has a minimised regression test, and one existing native fixture that was already wrong against the truth was corrected.

`4a01b0d` omits `window` and `context_percent` from bound Claude telemetry in `enrich()` and peer responses. The local snapshot struct still renders them as null.

**Coverage cost.** From the counts-only corpus replay, accepted under AGENTS.md (incomplete accumulated coverage stays unknown; never invent):

| Metric | Before | After |
|---|---|---|
| Sessions with a valid accumulated total | 18 of 19 | 12 of 19 |
| Finished turns | 221 | 215 |
| Turn seconds | 23,071 | 22,415 |
| Sessions with the current turn known | 15 | 14 |

The current and last turn values still publish after a proven end.

All 36 real joins in the corpus still join.

**A separate no-join experiment** (on the pre-fuzzer build) removed joins entirely. It also gave 12 of 19 sessions valid. Whether it lost the same files was not compared. Removing joins would buy no extra coverage, so the join logic is kept.

**Test counts:** lib 171, main 14, native_navigation 6, native_process 32.

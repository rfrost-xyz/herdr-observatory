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
  - The new spawn-storm test (since renamed `checkpoint_cycles_succeed_while_siblings_spawn`) failed 20 of 20 runs before the fix and passed 20 of 20 after. The wait is now guarded by `checkpoint_lease_and_write_wait_out_a_brief_holder`, and the release by `checkpoint_lock_is_free_at_once_after_each_holder_while_siblings_spawn`.
  - A later `read checkpoint: NotFound` failure had a different cause, reproduced in round 15: a lib test binary from between `51ab736` and `895fabe`, sharing `/tmp` from another pid namespace, swept live fixtures. The age gate in `895fabe` closes it (see rounds 11 and 15).

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
3. **Claude rows evicted Codex rows** from the shared cursor and checkpoint bounds. Codex is now charged first (`c38efe7`), and ring hashes shrank to 64 bits (`ac47bc9`). A block with a full ring went from 2,702 to 1,118 bytes (1,212 bytes after later fields were added; the test bound is 1,250). That is not the worst case: a row with 512 finished turns and 128 children is about 44 KB.

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

**Bench default:** `tests/bench_anton_native.mjs` now defaults to 0 Claude agents. Separately, `a984f7f` fixed the bench's fake-SSH peer match (`*--probe*)`). On origin/main the remote peer never came up, so the origin/main bench cannot produce a baseline. Bench figures recorded earlier with the old default of 4 are therefore not default-run figures.

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

## Ground-truth turn fuzzing (first oracle; superseded counts below)

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

The corpus has 18 real joins (`turns.joins`). An earlier figure of 36 counted each join twice, once in the shadow scan and once in the bounded replay.

**A separate no-join experiment** (on the pre-fuzzer build) removed joins entirely. It also gave 12 of 19 sessions valid. Whether it lost the same files was not compared. Removing joins would buy no extra coverage, so the join logic is kept.

**Test counts:** lib 171, main 14, native_navigation 6, native_process 32.

## Review round 4 and remediation

The four-lens review of `41e5a07` found 4 blocking and 7 other findings. All are fixed. Each code fix has a regression test that failed before it.

**Blocking:**
1. **Compactions from unclassifiable records.** An unclassifiable assistant record left `compactions` published. Fixed in `5652aca`.
2. **Silent ends shown as running.** A silent end published a running turn and a complete total through the idle gap. Fixed in `4666632` by masking publication through `published_turns()`.
3. **Lenient oracle.** The fuzzer oracle accepted values that were unchanged but stale. Fixed in `950f360` with the strict oracle.
4. **Epoch-stamped queue records.** These left an unresumable row. Fixed in `a56cbaf`.

**Other fixes:**
- a pending start and `lost_idle` publish an unknown current turn (`4666632`);
- a resume failure keeps the row and publishes all-null (`a77f804`);
- retention counts only panes that can have a row (`0d4abf5`);
- predecessor candidate open errors are truncated (`d3323da`);
- serde-rejected predecessor lines, headers and repeated keys go through the classifier (`8351109`, `6916e66`, `353b8dd`, `e239375`);
- native-shape State.js and shell fixtures (`8a89b8a`);
- a counts-only join counter (`677a34b`).

### Strict oracle (`950f360`)

The run covers 217,723 records with 0 restarts. Before the fixes the buckets were: current 3,147 pending, 1,163 silent end, 90 `lost_idle`; total 393 silent end; gate 7 epoch.

Final result: **0 violations**. The two allowances were counted: 4,372 values unchanged across a kill, and 54,006 last valid intervals.

Violations with each fix reverted by hand on HEAD:

| Revert | Violations |
|---|---|
| `75012fd` (a trigger replaces a pending start) | 2,764 |
| Local command while a prompt is pending | 0 (guarded by its unit fixture) |
| `1467ba3` (stamp order, approximate revert) | 832 |
| `97e442a` (an idle loss, then a queue operation) | 9 |
| `42b3118` (a second idle loss) | 0 (guarded by its unit fixture) |

### Counts-only corpus (`677a34b`)

| Measure | Result |
|---|---|
| Files | 19 |
| Joins | 18 |
| Publishable | 11 of 19 |
| Accumulated coverage valid | 12 of 19 |
| Current turn known | 14 of 19 (raw `Turns`, not the published mask) |
| Turn timing supported | 17 of 19 |
| Predecessor | 14 clear, 3 growing, 2 unknown |
| Oversized lines classified | 24 of 24 |
| Shadow agreement | 19 of 19 |
| Finished turns | 231 |
| Turn seconds | 22,573 |

The corpus is live. The totals grew because it grew, not because of a code change.

### Verification at `677a34b` (release sha256 `507f24e441dacd0539f8edf908e866bc55f8eb08cca12f27a6ece9a704f7dafe`)

| Gate | Result |
|---|---|
| fmt, clippy | clean |
| `cargo test`, 3 runs | lib 181, main 15, native_navigation 6, native_process 32 |
| `node --test` | 88 of 88 |
| QML | 95 of 95 |
| qmllint | clean outside Panel.qml |
| Shell harness | 0 failures |
| History | clean |

Runtime medians, measured in the same session:

| Metric | HEAD | Baseline |
|---|---|---|
| CPU | 0.093 s | 0.086 s |
| Peak RSS | 4,612 KiB | 4,348 KiB |

Per-window CPU was 0.088 to 0.093 s for HEAD and 0.085 to 0.091 s for the baseline. Both have one roughly 7.7 MiB process-overlap window.

The Claude probe reports 4/4 local and 4/4 peer, with all 9 fields, in both variants.

**Safety-classifier note.** The automated safety classifier timed out on one fix agent. The coordinator checked its actions: the worktree is clean, nothing was pushed, `openspec` and AGENTS.md are untouched, and the installed plugin is unchanged.

## Review round 5 and remediation

The four-lens review of `1d91cc2` found 3 blocking and 6 other findings. All are fixed, and each code fix has a regression test that failed before it.

**Blocking:**
1. **Local-command output left coverage valid.** When local-command output cleared a pending start, coverage stayed valid. `0816e74` simplifies the rule: local output clears only a pending command echo and never opens `lost_idle`. This rests on an assumption, now recorded: a command whose echo is followed by local output ran locally.
2. **Trigger after a record lost while idle.** Such a trigger opened a normal start. Fixed in `e9a6dc0`: it is now ambiguous.
3. **Peer replacement re-showed old totals.** A caught-up peer restart could publish nothing, so the local re-showed the replaced file's totals. Fixed in `4e6e3e2`: an all-null sample is published, or the row is withheld.

**Other fixes:**
- an overlong `sessionId` is foreign at both line sizes (`0b5b161`);
- a header that loses only a non-identity field binds (`0ada82a`);
- a reporter lock spawn test (`18083fe`);
- a startup lease failure is reported (`8f11ece`);
- spec wording on the last valid interval;
- the retention scope is documented.

**Ground-truth fuzzer.** The run covers 215,289 records with 0 violations, 0 restarts, 4,432 values unchanged across a kill and 51,755 last valid intervals.

Sessions with at least one violation when a fix is reverted:

| Reverted | Sessions with a violation |
|---|---|
| Local-output rule only | 8 |
| Lost-idle trigger only | 2 |
| Both | 36 |

**Counts-only corpus.** Same snapshot, run back to back:

| Metric | `1d91cc2` | Fixed |
|---|---|---|
| Coverage valid | 12 of 19 | 13 of 19 |
| Current turn known | 14 of 19 | 14 of 19 |
| Finished turns | 237 | 238 |
| Joins | 19 | 19 |
| Publishable | 11 | 11 |

### Verification at `8f11ece` (release sha256 `17cbd0f8e1811630a34b17254a2709ddae5c7bfa4e55133ffa7c5280d1f35e2b`)

| Gate | Result |
|---|---|
| fmt, clippy | clean |
| `cargo test`, 3 runs | lib 188, main 16, native_navigation 6, native_process 32 |
| `node --test` | 88 of 88 |
| QML | 95 of 95 |
| qmllint | clean outside Panel.qml |
| Shell harness | 0 failures |
| History | 108 commits, clean |

Runtime CPU: 0.095 s for HEAD against 0.089 s for the baseline, per-window 0.088 to 0.098 against 0.087 to 0.090. Peak RSS is noisy in both, swinging between 4.2 and 7.7 MiB.

Claude probe: 4/4 local and 4/4 peer, with all fields, in both variants.

## Review round 6 and remediation

The four-lens review of `085db29` found 1 blocking and 5 other findings. All are fixed.

**Blocking:**
- **Unverified peer row.** A peer deadline skip returned an unverified cursor row, which the local would re-emit without limit. `87f2625` publishes the all-null sample, or withholds the row, when there is no current binding. The narrower window, where the deadline passes after binding but before any pass, was found during remediation. `e0e6fc5` routed it through the same skip, but round 7 showed that had no effect on a peer, because `bind` had just made the binding current. `0abe129` fixes it properly.

**Other fixes:**
- a lasting resume failure keeps the 60 s cadence (`43cb273`);
- an overlong `sessionId` cut after the bound is a mismatch (`d9dc9af`);
- lock tests for genuine contention and holder release (`7e90254`, `1e48c49`);
- an assistant record with no turn running is ambiguous (`cf81cbc`);
- the current turn is masked after a slash command's local output (`0b92ce8`);
- an abort after local output is unknown (`1f7c77b`);
- the corpus check counts the published current turn (`75bff6e`).

**Fuzzer.** It ran 240,640 records with 0 violations and 0 restarts, then a temporary 10× seed sweep of 3,157,537 records, also with 0 violations. It counts 4,172 values unchanged across a kill and 63,531 last valid intervals.

**Corpus, counts only.** The only change from the previous build is the published current turn known, which goes from 13 to 12 of 19, because 2 files end with `local_idle` set. Coverage stays 13 of 19.

**Known limit (documented).** A slash command that runs the model, and is killed or has queued input taken before its first response, stays undercounted.

### Verification at `75bff6e` (release sha256 `032fb4a24d080c9eb0a259b35ae2c8d1d1e48cb74cc66fc734c43e0871b24f6f`)

| Gate | Result |
|---|---|
| fmt, clippy | clean |
| `cargo test`, 3 runs | lib 193, main 17, native_navigation 6, native_process 32 |
| `node --test` | 88 of 88 |
| QML | 95 of 95 |
| qmllint | clean |
| Shell harness | 0 failures |

| Measurement | HEAD | Baseline |
|---|---|---|
| Runtime CPU | 0.092 s | 0.094 s |
| Mean snapshot | 27,677.3 B | 27,675.1 B |

Peak RSS is noisy in both, the same process-overlap pattern as before. The Claude probe reports 4/4 local and 4/4 peer for every field.

`e0e6fc5` was verified afterwards with fmt, clippy and the full Rust suite: lib 193, main 17, native_navigation 6, native_process 32.

## Review round 7 and remediation

The four-lens review of `461c716` found 2 blocking, 1 non-blocking and 2 nit findings.

**Blocking:**
1. **Unrecognised origin with `isMeta` was ignored.** A user record with an unrecognised `origin` and `isMeta` was ignored instead of unknown, which turned unknown into a complete undercount. `8fb42b2` swaps the classification order. Its regression test failed before the fix.
2. **`e0e6fc5` had no effect on peers.** When the deadline passed right after a fresh bind, `skip_claude` saw the binding `bind` had just made current, so a peer still returned an unverified row. `0abe129` adds `skip_after_bind`: it re-emits only a retained sample, and otherwise publishes all-null or withholds the row. Round 8 found that the direct test reached only the helper. `15cb5d4` makes `claude_deadline_after_bind_never_returns_an_unverified_row` reach the call site through `enrich_claude` with an expired pass deadline. With `skip_claude` at the call site, both the all-null case and the withheld case fail.

**Other:**
- the checkpoint cycle test's doc comment and name now state what it checks (`4ab7901`);
- the test names in this file are corrected;
- the over-bound consumed-string difference is documented in D3 as fail-closed.

**Verification:** fmt and clippy are clean. The full Rust suite passes: lib 195, main 17, native_navigation 6, native_process 32.

## Review round 8 and remediation

The four-lens review of `ad1ddd0` found 3 blocking, 2 non-blocking and 2 nit findings.

**Blocking:**
1. **A trigger stamped before a proven end was published after an ambiguous turn.** `3571b3b` adds `end_floor`, the latest proven end. It also counts takes of queued input while idle, which the fuzzer showed was needed: violations went from 3,536 to 66 with ends only, and to 0 with idle takes.
2. **Two panes on one Claude session could re-show a replaced file on a peer.** `e4302f4` enriches each session once per probe, and `eefaefd` stops retention from re-emitting beside telemetry in the same generation.
3. **The round 7 test reached only the helper.** `15cb5d4` routes it through `enrich_claude`.

**Other:**
- `3fd22a9`: content after a notification block is unreadable.
- `09498b0`: refactor.
- `14c6684`: large bench transcripts end a minute in the past.
- A stale test name is corrected.

**Lens 1 finding, lines between 64 KiB and `TAIL`.** Such a line, cut at a pass end or while being written, lost coverage for the rest of the binding. `a20e5c2` fixes it: the line is rewound and read whole on a later pass. Three new tests failed before the fix: a static cut at `TAIL`, a line being written, and a user record. Three existing tests that encoded the bug now use lines longer than `TAIL`. The full suite passes: lib 203, main 18, native_navigation 6, native_process 32. The ground-truth fuzzer passes, and the counts-only corpus output is unchanged.

**Fuzzer:** 240,491 records, 0 violations, 0 restarts.

**Corpus** (counts only, the current corpus of 20 files, identical before and after these commits):

| Measure | Result |
|---|---|
| Coverage | 14 of 20 |
| Published current turn | 14 of 20 |
| Publishable | 12 of 20 |
| Joins | 20 |
| Shadow agreement | 20 of 20 |

### Verification at `14c6684` (release sha256 `c9cf061d240de63a122972d5bb9fd80083f007bf68dd2f9ce9e91c3c085887d6`)

| Gate | Result |
|---|---|
| fmt, clippy | clean |
| `cargo test` (3 runs) | lib 200, main 18, native_navigation 6, native_process 32 |
| `node --test` | 88 of 88 |
| QML | 95 of 95 |
| qmllint | clean |
| Shell harness | 0 failures |
| History | 131 commits, clean |

| Measure | HEAD | Baseline |
|---|---|---|
| Runtime CPU | 0.033 s | 0.033 s |

Peak RSS is the known two-valued process-overlap noise.

## Review round 9 and remediation

The four-lens review of `ade3fad` found 1 blocking, 2 non-blocking and 3 nit findings.

**Blocking: an end with no usable timestamp.** A `turn_duration` or abort whose timestamp was missing, unparseable, epoch-stamped or in the future did not raise the proven-end floor. A later trigger stamped before it could then be published.
- `1e228fb` leaves turn starts unknown after such an end.
- `1aff002` handles the same hole for idle takes with no usable second.
- `zz_trigger_before_an_unstamped_end_is_never_published` gave 50 wrong values before these fixes.

**Other fixes:**
- **Lines longer than `TAIL`:** `999bcaa` cuts them only at `TAIL` multiples from their start, so a line observed mid-write reads as the whole file does. Its regression test failed before the fix.
- **Old local:** stripped Claude rows can evict Codex rows from an old local's checkpoint. This is now documented in D8 and D9.
- **Comments:** two test comments are corrected (`0f790eb`).
- **Bench:** the bench peer-match fix is now recorded.

**Housekeeping:** about 12 GB of reviewer build copies in the shared scratch space were removed, because `/tmp` had reached 77%.

**Fuzzer:** 314,561 records, including a targeted stamp-fault mode, with 0 violations and 0 restarts.

**Corpus:** byte-identical to `ade3fad`.

**Gates:** fmt, clippy and the full suite (lib 205, main 18, native_navigation 6, native_process 32).

**Known behaviour change (fail-closed):** after an abort with no usable second, a directly following `<synthetic>` record is ambiguous.

## Review round 10 and remediation

The four-lens review of `c85abd2` found 1 blocking, 3 non-blocking and 1 nit finding.

**Blocking.** Input taken while a slash-command echo was pending, followed by local output, left no trace. The next start was then published at the input's queue time.
- `ec97d37` makes such a take ambiguous.
- A fixture covers five forms of the take. Each form published a wrong start, last interval and total before the fix.
- A new fuzzer shape reported 12,625 violations over 319,317 records before the fix and 0 after.

**Other fixes:**
- **Predecessor scan.** `6df8e19` caches a binding next to an unchanging finished sibling. Before, it rescanned on every probe, and a deadline skip lost the retained sample.
- **Codex-first ordering test.** `65a92fd` checks the recorded order rather than timing. As a mutation check, swapping the two loops in `enrich_until` made it fail.
- **Checkpoint test steps.** `b296f4f` names each failing step and adds `#[track_caller]`. `357471e` gives unit fixtures a prefix disjoint from process fixtures. Both respond to a later read failure in the checkpoint test.
- **Test name.** `1397cdb` renames the over-`TAIL` regression test after its guarantee.

**Corpus.** Unchanged. The example scans cold, so the new predecessor caching rule is covered by its unit test, not by the corpus.

**Gates.** fmt and clippy are clean. The full suite passes: lib 207, main 18, native_navigation 6, native_process 32.

## Review round 11 and remediation

The four-lens review of `ca52bda` found 2 blocking, 3 non-blocking and 1 nit finding. Lens 1 was **CLEAN**.

**Blocking:**
1. **Injected triggers after local output.** After a slash command's local output, a task notification, peer or coordinator trigger could open a normal start inside a turn that the command was running.
   - `e90f4fd` makes these triggers ambiguous.
   - The fixture failed on HEAD for all four forms.
   - The fuzzer shape found 8,175 violations before the fix and 0 after (320,010 records).
   - Corpus cost: accumulated coverage falls from 14 to 13 of 20, with 1 fewer finished turn and 104 s.
2. **Request-row process test.** The test never reached the gate it was named after. `23a4ef7` replaces the request row count with the request's key set: a response's Claude row keys must be a subset of the request's keys. The process test was rewritten to store a sample first, and both it and a new `main.rs` test failed on HEAD.

**Other fixes:**
- `faf270a`: the Codex-first test pre-seeds Codex discovery.
- `51ab736`: process fixtures use their own prefixes and fixtures remove dead-pid siblings they own.
- `d70898b`: the checkpoint diagnostic prints the root inode and listings.
- `76d7470`: corrects the bench wording.

**Checkpoint test failure.** The later read failure has a plausible cause that is not confirmed. A clean-up of leaked `/tmp/anton-native-*` directories from a killed run matched the old unit prefix. A sweep reproduced the read failure 150 times in 150 runs under the old prefix, and 0 times in 150 under the new one. Leaked directories from one killed run were present and have been removed. The new diagnostic records enough to confirm the cause if the failure recurs.

**Gates:** fmt and clippy pass. The full suite passes: lib 208, main 19, native_navigation 6, native_process 32.

## Review round 12 and remediation

The four-lens review of `a97f2ac` found **no blocking findings**: 4 non-blocking and 1 nit. All are fixed, and each fix has a regression test that failed before it.
- **Classifier depth bound** (`a2037a9`). A well-formed record nested past the 128-container bound was rejected wholesale. It now follows the coverage table, so totals stay published.
- **Queued prompt attachments outside a turn** (`c562e4a`). These are now ambiguous, and an idle take clears abort adjacency. The fuzzer gave 311 violations with this fix reverted and 0 with it (319,933 records).
- **Request row rejected by the peer** (`caae67d`). When the peer's own validation rejects the request row (a peer clock step), the row is withheld.
- **Fixture sweep** (`895fabe`). It now also needs an hour of inactivity, which is safe across pid namespaces.
- **Checkpoint diagnosis** (`2627ffd`). It is printed at every failing step.

**Corpus:** unchanged (0 of 59 lines).

**Gates:** fmt and clippy are clean. The full suite passes: lib 215, main 19, native_navigation 6, native_process 32.

**Commit signing:** these five commits were made unsigned by the fix agent, because 1Password SSH signing failed ("agent returned an error"). They were re-signed once signing worked again.

## Review round 13 and the turn-timing gate (user decision)

The round 13 review of `5a2fb76` (re-signed as `2627ffd`) found three blocking findings. Lens 3 was CLEAN.
1. **Compaction iteration on a reopened group.** It was ignored. Fixed in `8e1b875`.
2. **Final stops other than `end_turn`.** With no `turn_duration`, `max_tokens`, `refusal` and `<synthetic>` stops merged two turns into one.
3. **Deferred `turn_duration`.** Claude Code defers a turn's `turn_duration` while background agents run, and may write it during a later turn. This was established by reading the Claude Code 2.1.287 bundle (`[bin]`).

**Turn timing on real transcripts** (counts-only corpus measurement):
- 13 of 260 finished intervals (5%) had a published start differing from the start implied by `durationMs` by more than 2 s. 12 of those differed by more than 10 s.
- 11 of the 13 showed signs of background agents.
- 10 of the 13 implied a start before the previous recorded end. Round 14 corrected the interpretation (see below): these are turn-end records whose `durationMs` Claude measures from the first turn of a pending run, not split turns.

**User decision: gate and mask.** Given the measurement and 13 rounds that kept reopening D7, the user chose "Gate + mask, then ship". Implemented in these commits:
- `097908b`: the `durationMs` gate. It only rejects intervals and never replaces a bound.
- `aac7db3` and `2d2e71e`: the current turn is shown only from a dated, clean state.
- `a7befd3`: an aborted interval is shown only from a dated turn, and every abort makes accumulated coverage unknown.
- `e36d6c2`: any final stop other than tool use is a silent end.
- `109b3cd`: an orphan `turn_duration` keeps injected triggers ambiguous.
- `0991c86` and `4b98a19`: fuzzer shapes and corpus counters.

All commits are signed. Each new unit test failed before its fix.

**Fuzzer:** 326,541 records, 0 violations, 0 restarts. Removing a rule brings violations back:

| Rule removed | Violations |
|---|---|
| final-stop rule | 86 |
| orphan rule | 532 |
| orphan rule and gate | 7,233 |

**Corpus, before and after the gate.** Same snapshot, run back to back:

| Measure | Before | After |
|---|---|---|
| Gate | | 203 kept, 18 rejected |
| Files with valid coverage | 13 | 12 |
| Finished intervals | 283 | 204 |
| Running turns with a published start | 7,500 of 8,441 records | 2,722 of 7,393 records |

The drop in finished intervals is mostly a cascade: after a rejected `turn_duration`, `local_idle` makes later injected triggers ambiguous. A lever that kept 262 intervals was first declined on the premise that a rejected `turn_duration` is a deferred flush. Round 14 disproved that premise, and the lever was then applied (see below).

**Gates:** fmt and clippy are clean. The full suite passes: lib 221, main 19, native_navigation 6, native_process 32.

## Review round 14 and remediation

The four-lens review of `af69645` found 2 blocking findings, both small, and several non-blocking ones.

**Blocking:**
1. **Compaction iteration on a `<synthetic>` record.** It was ignored. Fixed in `5a1940d`.
2. **Contract text versus behaviour.** The contract said the current turn is published only after a checked end, but the session's first turn is also published. `113f5fb` corrects the wording in the spec and AGENTS.md, and allows a dated aborted interval as `last`.

**Producer correction (lens 2).** Evidence from the Claude Code bundle and counts-only corpus queries:
- 0 of 289 `turn_duration` records appear mid-turn. Each is written at its own turn's end.
- A record with nothing pending, written after a pending run, measures `durationMs` from the run's first turn.

So a rejected record is a real turn end. `2283b41` ends the turn at a rejected record without setting `local_idle`. This is the lever that round 13 declined on a wrong premise, and it stays within the user's gate-and-mask decision.

| Corpus measure | Before | After |
|---|---|---|
| Finished intervals | 207 | 265 |
| Gate | kept 206, rejected 18 | kept 264, rejected 19 |
| Coverage valid | 12 of 20 | 12 of 20 |
| Published current turn | 12 of 20 | 12 of 20 |

**Remaining limitation:** the 19 rejected records are real ends whose intervals are probably correct. They stay unknown, which fails closed. Round 15 showed that only 11 are pending-run records, which modelling the run could recover; the other 8 reflect dialog-paused time that the transcript does not record (see round 15).

**Other fixes:**
- `6f57c3d`: the fuzzer models pending runs, background-agent turns and a hostile mid-turn `turn_duration`. With the gate it reports 0 violations over 327,091 records; with the gate disabled it reports 11,834.
- `bc2c348`: older non-file siblings are skipped in the predecessor scan.
- `bc3bb34`: synthetic turns in the popover measurement take their `durationMs` from their span, and `turn_timing` counts only with a numeric duration.
- `615b9f9`: the Codex-first test is made deterministic.
- `ddc7a29`: the rediscovery test is made robust to load.
- `6f8a3fa`: the checkpoint test names the load step that finds no file.

**Pre-existing follow-up, not in this change:** a fresh Codex cursor's `seq = (time*1e6) as u64` can round above `time` in `telemetry_view_at`, which leaves a one-probe gap after a fresh Codex binding with no child records. It is present on origin/main, and production Codex code is unchanged here.

**Gates:** fmt and clippy are clean. The full suite passes: lib 223, main 19, native_navigation 6, native_process 32.

## Review round 15 and remediation

The four-lens review of `c1d0604` gave three CLEAN lenses: 1, 3 and a lens 4 with no blocking findings.

**Lens 2, blocking: workflow launch.**
- An `async_launched` tool result without an `agentId` left the current turn published while the launched work ran. Workflow launches look like this: 43 in the corpus without an `agentId`, against 25 with one (counts only).
- Fixed in `c08eeee`: any async launch clears `clean`. The regression case in `turns_current_turn_is_published_only_from_a_clean_state` failed before the fix.
- Corpus cost: running records with a published start fell from 2722 to 2703. Coverage, the gate and published-current counts are unchanged.

**Lens 2, non-blocking: paused time.**
- Claude Code subtracts time paused on blocking dialogs from `durationMs`.
- 8 of the 19 rejections imply a later start. In each, the turn's first assistant record precedes the implied start, so these are paused time and cannot be recovered. The other 11 are pending-run rejections.
- D7 Durations and Risks now record this, along with the swarm-defer path, which a scratch trace confirmed fails closed.

**Lens 2, nit.** The Publication paragraph now says local-command output clears `clean` only when it clears a pending slash-command start.

**Lens 4, non-blocking: checkpoint failure cause, now reproduced.**
- A lib test binary built between `51ab736` and `895fabe`, running in another pid namespace with a shared `/tmp`, treats every host pid as dead and sweeps live fixtures.
- With that binary as a namespaced neighbour, the checkpoint test failed 73 times in 24,732 runs, including the reported `read checkpoint: NotFound`. With HEAD's binary as the neighbour, it failed 0 times in 26,383.
- The age gate in `895fabe` closes this. An `flock`-based liveness check would also survive clock jumps; it is recorded as a follow-up.

**Lens 4, nit.** The round 12 hashes above were updated to their re-signed commits. The round 13 review ran on the unsigned `5a2fb76`, which was re-signed as `2627ffd`.

**Gates:** fmt and clippy are clean. The full suite passes: lib 223, main 19, native_navigation 6, native_process 32.

## Review round 16 and remediation

The four-lens review of `1386117` produced:
- lens 3: CLEAN;
- lenses 1 and 4: no blocking findings;
- lens 2: one blocking finding.

**Lens 2, blocking: teammate launches.**
- A teammate launch is written as `teammate_spawned`, with a snake-case `agent_id`. A remote launch is written as `remote_launched`. Neither cleared `clean`, so the current turn stayed published while the launched work ran. The corpus has 4 `teammate_spawned` results and 0 `remote_launched` (counts only).
- The fix: any of `async_launched`, `teammate_spawned` or `remote_launched` now masks the current turn. Both shapes are added to `turns_current_turn_is_published_only_from_a_clean_state`, which failed before the fix.
- The spec now names the background work it masks for. Background shell tasks are excluded, because they do not change `durationMs`.

**Lens 2, non-blocking: swarm-defer accept case.** D7 now describes the case where a swarm-deferred `turn_duration` lands while its spawning turn is still open. The gate then accepts Claude Code's own measure of that turn.

**Lens 2, nit.** The test doc comments for the mask and the aborted interval were moved and corrected.

**Lens 1, non-blocking: empty key in an oversized line.**
- On an oversized line, a top-level `""` key matched the root node. A long value at that key then made the record unclassified, although the short-line path classified it.
- The classifier now ignores empty keys, as `viable()` already did. A parity line in `classifier_matches_parsed_path_for_every_chunking` failed before the fix.
- Lens 1's random parity checks agreed in all cases: 264,262 records, 200,000 with checkpoints at every chunk, and 400 growth transcripts.

**Lens 4, nit.** The checkpoint test summary above now points to the reproduced `NotFound` cause.

**Gates:** fmt and clippy are clean. The full suite passes: lib 223, main 19, native_navigation 6, native_process 32.

## Review round 17 and remediation

The four-lens review of `857fbe0` produced:
- lenses 1 and 3: CLEAN;
- lens 4: no blocking findings;
- lens 2: 2 blocking findings in the current-turn mask.

Neither finding published a wrong number: the start shown is Claude Code's own, and the next turn end reports the work as pending. In each, though, the current turn stayed published while background work ran.

1. **Resume of an agent not launched in this file.**
   - Claude Code writes `resumedAgentId` only when it resumes on the background path.
   - Corpus (counts only): 2 such resumes, against 3 of known children.
2. **Task notification without a final status.**
   - A resume by the user or by another agent is written as a notification with no `<status>`, which says the agent is running again. For a known child it set the status `unknown`, which `running()` does not count.
   - The corpus has none of these (counts only).

**Fix.** The rule is now conservative for every child-related signal. `clean` is cleared by:
- any successful resume, known child or not;
- any task notification that does not report a final status;
- a task notification whose task id cannot be read.

The next gated end with nothing pending restores `clean`. Three cases were added to `turns_current_turn_is_published_only_from_a_clean_state`. The resume and status-less cases failed before the fix. The unreadable-task-id case was already masked through invalid children (round 18 correction), so its redundant `clean` line was removed.

**Corpus impact:** none. Published running records stay at 2703, the gate kept 267 and rejected 19, coverage is valid in 12 of 20 and the current turn is known in 12 of 20.

**Lens 4, non-blocking.** This repeats the round 15 finding about the fixture sweep's liveness check: pid plus age is unreliable across pid namespaces and clock jumps. The `flock` liveness check stays a follow-up. The review workflow now runs every cargo test with a private `TMPDIR`, which removes cross-run sharing.

**Gates:** fmt and clippy are clean. The full suite passes: lib 223, main 19, native_navigation 6, native_process 32.

## Review round 18 and remediation

The four-lens review of `b3d587d` produced:
- lenses 1 and 3: CLEAN;
- lens 4: one non-blocking finding, the round 17 claim corrected above;
- lens 2: 5 blocking mask findings, none with a wrong number.

For round 18, lens 2 was briefed to list every producer shape that starts or resumes background work in one pass, instead of one per round.

**Fixed (`c95427a`):**
- **B1: `/fork` and `/subtask`.** These start a background agent and write only `system/local_command` records. Such a record now clears `clean`. The corpus has 1 instance of this shape (counts only).
- **B2: Skill `status: "forked"`.** It runs in the background by default and now counts as a launch for the mask. The corpus has 0 instances; the binary is the evidence.

Both cases were added to `turns_current_turn_is_published_only_from_a_clean_state`, and each fails with its fix removed.

**Spec narrowed, residuals recorded under Risks:**
- **B3 and B4.** A SendMessage resume of an agent owned by another agent, or of an evicted teammate, carries no `resumedAgentId`. The only signal is message text (0 of each in the corpus).
- **B5.** Claude Code does not count remote tasks as pending, so a later gated end restores the mask.

The spec now promises a mask only for work the transcript records in a structured field. None of these cases publishes a wrong number.

**Non-blocking:**
- Forked-skill agents are not counted as children. D6 now says so.
- The `stopped` status maps to `unknown`.

**Lens 2 sweep dispositions.** These were checked against the binary:
- **Covered:** all launch statuses and `resumedAgentId`.
- **Fail closed:** notifications without a final status or task id, main-session backgrounding (an abort), and detached tool calls (a silent end).
- **Not affecting `durationMs`:** background Bash and Monitor tasks.

**Corpus (counts only):**
- Running records published: 2693, down from 2703.
- Gate: kept 268, rejected 19.
- Coverage valid: 12 of 20 files.
- Current turn known: 12 of 20 files.

**Gates:** fmt and clippy are clean. The full suite passes: lib 223, main 19, native_navigation 6, native_process 32.

**Round 19.** Lenses 1 to 3 were CLEAN. Lens 4 found no blocking or non-blocking issues and two design.md wording nits, both fixed: the unreadable task-id case makes children invalid and does not clear `clean`, and the residual list now has three bullets.

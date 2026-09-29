# Evidence

## Scope

Implementation migration preserving canonical omarchy-companion, harness-telemetry and account-allowances contracts. Local dirty worktree is retained; no Git commit/forge, Docker or remote deployment is part of this change.

## Pre-change baseline

Captured before executable implementation. Synthetic two-host, 32-agent-per-host Python normalisation + snapshot/deepcopy + double JSON: 2,000 cycles across five repeats, CPU seconds 2.2799, 2.2753, 2.3009, 2.3009, 2.3022; median 1.150 ms/cycle; peak RSS 37,252 KiB, ~22.6 KB wire. This excludes transport, transcript parsing and real SSH.

Bundled Python cold lifecycle with one unavailable synthetic local socket and checkpoints, five isolated temporary installs: first snapshot median 118.06 ms (105.92–121.44), RSS 28,256–28,352 KiB, owner-EOF exit 15.39–15.51 ms, all exit zero. Initial snapshot is Connecting rather than a fabricated successful sample.

Installed baseline from parent, popover closed: 30.01 seconds, Python collector CPU 0.26 seconds (0.866% of one logical core), RSS 31.32 MiB, seven threads. Excludes SSH children, remote CPU and Qt/GPU. Separate 30.13-second local-family sample: 0.81 CPU seconds (2.688% of one logical core), peak four observed local-family processes, 22 new descendants observed at 200 ms sampling. Includes parent/reaped-child counters, excludes remote CPU and Qt/GPU; short-lived children can evade process counts. No wake-up or frame-time claims.

## Verification

Design reviewed by coordinator, root and migration validator. The implementation-only scope and Rust/domain-adapter ownership were accepted. No canonical behavioural requirement changes.

First full-process synthetic comparison exposed an avoidable interpreter-start regression: 10-second, two-host/64-thread fixture measured Python 0.4579 CPU seconds, sampled peak RSS 56,928 KiB, owner EOF ~15 ms; initial one-shot Rust orchestration measured 2.017 CPU seconds, 73,168 KiB, EOF ~1,116 ms. All-connected latency changed from ~1,094 to ~283 ms. This is diagnostic evidence, not accepted performance. Design changed to request-driven retained domain adapters with bounded parser memoisation, while Rust retains cursor/state/schedule authority.

Coordinator adapter tests cover exact existing normalisation, opaque session generation, Work disclosure with native timing stripped, checkpoint lease retirement and cross-process write throttle, invalid version handling and absent allowance mapping. An early combined regression run passed 208 of 210 tests; the two migration failures were corrected during retained-adapter implementation. Final complete regression gate passes all 223 tests.


The intermediate sixty-second run with absent-allowance and empty-checkpoint work suppressed separated startup from steady cost. Python total family CPU 2.3100 s; observed steady CPU 2.13 s / 57.990 s; sampled peak RSS 56,800 KiB and PSS 27,287 KiB; all-connected 1100.78 ms; owner EOF 31.53 ms. Per-host retained Rust version: total CPU 2.3868 s; observed steady CPU 2.00 s / 57.991 s; RSS 94,176 KiB, PSS 43,400 KiB; connected 280.68 ms; EOF 7.31 ms. RSS sampled every 50 ms, PSS every second; observations may miss short-lived peaks and RSS counts shared pages. Sustained CPU was comparable but duplicate-interpreter memory remained material. Root required consolidation before installation. The implementation now uses one shared bounded request-ID domain broker, preserving concurrent host requests and exact response correlation. This intermediate version was never installed.


## Shared-broker comparison and review remediation

The first consolidated sixty-second pair measured Python total family CPU 2.3131 s, observed steady 2.09 s / 58.033 s, sampled peak RSS 57,124 KiB, all-connected 1092.82 ms and owner EOF 116.37 ms. Rust measured total CPU 2.3371 s, steady 2.06 s / 58.002 s, RSS 63,600 KiB, connected 293.88 ms and EOF 9.67 ms. This shows CPU parity with approximately 6.3 MiB additional sampled family RSS, while synthetic connection visibility improves. One-second PSS peaks missed brief descendants unevenly and are unsuitable for a precise peak comparison. This package preceded the final existing-checkpoint cleanup and native subprocess runner; final accepted-source verification follows below.

Independent review caught local allowance rows waiting behind slow remote refresh and an expired/removed-host checkpoint retention edge. Fixed by separate Rust remote refresh and local projection requests, plus guarded startup reconciliation of an existing owned checkpoint. Tests exercise a blocked remote request while local allowance changes remain visible, no empty checkpoint creation, unchanged-file stability, expired rows and removed-host rows.

Coordinator and independent review identified unbounded intermediate subprocess capture in retained Python helpers. The native adapter now opts into a shared nonblocking runner for SSH, local/remote Herdr, NVIDIA queries and publication, with bounded concurrent stdin/stdout, discarded stderr and existing deadlines. Legacy Python/web execution defaults remain unchanged. This is an enforcement of the native bounded-process contract, not a remote service change.

## Final source verification

Frozen implementation passes 13 Rust unit tests, `cargo fmt --check` and locked/offline Clippy across all targets with warnings denied. Complete Python suite passes 223 tests in 40.181 seconds, including ten domain-adapter, nine native process and seven bounded-I/O tests. Eight JavaScript suites, all 24 QML fixture tests and plugin manifest validation pass. Strict OpenSpec validation passes all six active/spec items; `git diff --check` passes. Independent adversarial review approves the implementation after the allowance, checkpoint and subprocess corrections.

Contract evidence: exact legacy/native snapshot fixture parity preserves usage, context, subagent outcomes and session generation; heartbeats retain source timestamps; blocked remote operations do not delay local host or allowance projection; malformed/oversized/wrong-ID broker output cannot change state; process reuse/recovery, owner EOF and SIGTERM are exercised. Bounded-I/O tests cover stdout floods, discarded stderr floods, concurrent input/output, blocked-input deadlines and direct-child reaping. Existing installation/uninstall, reporter hooks, Work publication/disclosure and navigation regression suites remain green.

QML presentation is unchanged apart from the collector executable command. Python remains required for specialised parsing, reporting, domain I/O and optional music. The migration adds Rust state/lifecycle ownership; it does not claim a complete Python replacement, UI speedup or measured wake-up reduction. Installed acceptance remains pending with the parent coordinator.

## Final synthetic resource comparison

The final frozen package was compared for sixty seconds per runtime with two synthetic hosts and 64 total threads. Python total family CPU was 2.2484 s, observed steady CPU 2.07 s / 58.021 s, peak sampled family RSS 57,224 KiB, first frame 93.02 ms, all-connected 1094.32 ms and owner EOF exit 34.11 ms. Rust plus its retained Python broker measured total CPU 2.3550 s, steady 2.05 s / 58.006 s, RSS 64,128 KiB, first frame 123.09 ms, all-connected 305.49 ms and EOF 9.89 ms. Sustained CPU is comparable; total CPU is 0.107 s higher and RSS is 6.74 MiB higher (about 12%). The initial frame arrives later while both synthetic hosts become visible sooner. These are synthetic observations, not a prediction of live SSH or UI performance.

The harness includes local children and a local fake-SSH subprocess, excludes remote CPU, real network latency and Qt/GPU, and freezes both packages before each pair. RSS is sampled every 50 ms and sums shared pages. One-second sampled PSS peaks were 35,919 KiB and 41,024 KiB; short-lived children can be missed unevenly, so these do not support a precise memory-saving claim. No speculative assembly or intrinsics were introduced.

Final compared release binary SHA256: `6c0d47aaf54150372f4e34ca6d26909b9332cf7f96d8ed7012091b510166b943`. Adapter: `28701cf5f3f02caba11e59c2fa0c7ef307dd5ae0362d0e2a0b59b2c1d084bcc3`. Benchmark archive: `7d08d4fe46adda6752b6ab26dd236ba3fbfc6bd3a9d30597ac67fcf6ff5ebbd8`. Zip container hashes depend on the generated empty hooks entry timestamp; module-content parity is also checked when staging the installed payload.

The accepted binary was built with system Cargo/rustc 1.98.1 using `PATH=/usr/bin:/bin /usr/bin/python3 omarchy/herdr.observatory/build-native.py DESTINATION`. The available mise 1.96.0 toolchain also passes Rust gates but produces a different binary hash. The installed handoff preserves the exact benchmarked 1.98.1 artifact. Staged runtime.zip SHA256 is `08d169c9491a6537e197790e7caccd6bacb694d3cb0e438a927fdec166a6b4f1`; every non-generated member was checked against current source bytes.

## Installed acceptance preparation

Root independently verified all six staged file hashes, source parity and every runtime.zip member, then repeated strict validation (6/6) and whitespace checks. Installation is waiting for the desktop to be unlocked; the existing installed plugin remains unchanged.

An immediate pre-rollout closed-popover baseline from the parent measured 0.95 local-family CPU seconds / 30.18 seconds (3.148% of one logical core); sampled summed RSS minimum 41.9 MiB, maximum 70.8 MiB and mean 44.02 MiB; peak four processes and 19 observed new descendants. This includes local children, excludes remote CPU and Qt/GPU. The 200 ms samples can miss transients, summed RSS double-counts shared pages, and live workload varies. Post-install comparison must use the same sampler and scope.

## Live resource finding and music consolidation

The first installed revision passed live functional checks: both hosts reported three threads with usage/timing, both allowance rows were available, and Work publication was fresh (2.7 seconds, no Work agents expected, two allowance records). Private configuration/account files and the owner marker inode were preserved. Installed acceptance was held because the configured optional Music path, absent from the first synthetic benchmark, retained another Python interpreter. A 30-second sample measured mean family RSS 81.99 MiB versus the pre-rollout 44.02 MiB; CPU was 3.66% versus 3.148% of a core with varying live work. Individual RSS observations were Rust 4.64 MiB, domain broker 35.97 MiB and separate Music interpreter 28.96 MiB.

The source now consolidates the existing Music IPC/publication thread into the single domain broker. An explicit, broker-only idempotent music-start request re-establishes it after broker replacement; repeated/concurrent requests cannot create duplicate workers. Broker closure marks the owner closed before stopping Music and its publisher. The installed working revision remains until renewed process, configured-music benchmark and independent review gates pass. This live finding supersedes the earlier source acceptance and keeps installation/archive tasks open.

Renewed source verification after Music consolidation: 227 complete Python tests pass in 48.960 seconds, including twelve adapter tests and eleven native process tests. The configured Music fixture uses a fake cliamp Unix socket plus local fake SSH publication receivers, with GPU utility stubbed. It verifies one broker/interpreter, one active cliamp connection and Music publisher, repeated-start idempotence, broker-crash recovery without duplicate workers, sanitised Music/Work frames, and owner EOF/SIGTERM cleanup of sockets and descendants. All thirteen Rust tests, formatting and Clippy pass. Independent review renewed approval after inspecting the final integration tests and independently running adapter/Rust tests. Existing JavaScript/QML results remain applicable because no UI source changed in this correction.

## Consolidated Music resource comparison

The final configured-Music and Work-publication sixty-second pair measured Python total family CPU 9.4308 s, observed steady CPU 8.73 s / 58.013 s, sampled peak RSS 79,960 KiB, first frame 314 ms, all-connected 1317 ms and EOF exit 68.43 ms. Rust plus its one shared Python broker measured total CPU 9.2203 s, steady 8.54 s / 58.011 s, RSS 83,352 KiB, first frame 365 ms, all-connected 974 ms and EOF 10.33 ms. Sampled peak RSS overhead is now 3,392 KiB (3.3 MiB, 4.2%) in this fixture; CPU is comparable. These values include frequent synthetic Music/Work publication and cannot be compared directly with the earlier no-music fixture.

Both runtimes maintained exactly one cliamp connection, maximum one active connection, across the entire run. Python emitted 894 Music and 29 Work frames; Rust emitted 888 and 28. One-second sampled PSS peaks were 41,507 and 47,546 KiB, with the same brief-child sampling limitation. All transport/receiver endpoints were synthetic; anonymous read-only kernel metrics exercised Work publication while the GPU utility was stubbed. No live remote service was used.

Final consolidated binary SHA256: `64dac84e712007367fc3ecb56a0d5b1718f1646baba27e5bdec486345acdcafc`; adapter `e8fe8c087f560315a756ab4f3ca548d1dc27a1141b693724a6276a45d5f5e647`. The staged executable is a direct copy of the measured system-Rust 1.98.1 artifact. Renewed installed acceptance remains the final gate.

## Final installed acceptance, 27 September 2026

Root accepted the revised installed release after direct visual and runtime checks. Both hosts were connected/reporting; all three threads carried usage, timing and totals, none was stale, and both allowance rows were available. The live popover retained the existing design and all expected rows without an error. It was closed after inspection.

The warm restart succeeded. Previous Rust and broker PID/start-time identities were gone, and the fresh runtime contained exactly one Rust collector and one Python broker, with no separate Music interpreter. All six installed payload hashes matched the final staged release; private config/accounts fingerprints and the install-owner marker inode were unchanged. Work receiver file age was 5.2 seconds, source age 7.1 seconds, online. The Music receiver was freshly updated with `available: false` and no source age, correctly indicating no current live music measurement rather than inventing playback. Synthetic fixture tests separately exercised actual music frames and publication.

The final closed-popover thirty-second live family sample measured 1.11 CPU seconds / 30.28 seconds (3.665% of one core), summed RSS minimum 50.41 MiB, maximum 90.88 MiB and mean 54.67 MiB; peak six processes and 24 observed new descendants. The earlier Python sample was 3.148% of a core and mean 44.02 MiB. These are different live workloads/times, not a controlled speed comparison; summed RSS counts shared pages, sampling misses brief children, and remote CPU plus Qt/GPU are excluded. The controlled configured-Music fixture above provides the narrower comparison: similar CPU with 3.3 MiB higher sampled peak family RSS.

Root approved final installed acceptance. No source payload was edited after review. All task-owned synthetic fixtures/processes/logs were removed; root owns removal of live screenshots/profile files, staged releases and rollback after this evidence was recorded. Existing private state, source worktree and normal ignored Cargo build cache are preserved. This implementation-only change has no delta specifications; canonical behaviour remains unchanged. Delivery is local and installed, with no Git/forge, Docker or remote service mutation in this change.

Archived locally to `openspec/changes/archive/2026-09-27-migrate-anton-runtime-rust` with every task complete. Final strict validation passes all five canonical specifications and all 52 archived change task registers; whitespace validation passes. No canonical spec sync was required (`skip_specs: true`, no delta artifacts).

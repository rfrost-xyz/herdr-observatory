# Evidence

## Scope and authority

One local implementation change in the existing dirty worktree. No Git/forge action, container/service change or automatic peer provisioning. Coordinator owns source/specs; root owns private configuration, installation and remote rollout. Prior accepted changes remain intact.

## Requirement trace

| Contract | Implementation | Verification |
| --- | --- | --- |
| Bounded saved-profile discovery and legacy bindings | `fleet.rs`, `config.rs`, `main.rs` | Projection/validation units and real empty-to-add-to-target/session replacement fixture pass |
| Generational host/account retirement and checkpoint ownership | `main.rs`, `collection.rs`, `allowances.rs` | Generation unit, rename/cadence/removal/restart fixture, slow host/account process-group retirement, discovery EOF and last-good inventory fixtures pass |
| Exact observed profile navigation | `navigation.rs`, `Panel.qml`, `State.js` | Five focused Rust units and two native CLI fixtures pass; changed/removed/disabled/duplicate profiles make no focus/window calls |
| Setup-needed and discovery-unavailable presentation | `State.js`, `PopupContent.qml` | Forty State assertions and 25 production QML checks pass; synthetic screenshots inspected by coordinator |
| Stable labels/collapse/effects and account identity | Route-bound projection and existing UI identities | Focused JavaScript/QML fixtures pass; renewed independent integration review approved |
| Explicit peer provision and complete removal | Plugin README and `deployment.md`, existing receipts | Synthetic configurations validate unchanged; root verified installed mappings and two unique peer receipts |

## Visual inspection

Synthetic production-QML captures show “Setup needed” in the existing machine hint and “Discovery unavailable” within the existing Threads header. Both retain the compact layout, section/machine collapse and status-coloured thread metrics. No new blocked thread or menubar priority is inferred. These fixture captures are not installed desktop acceptance.

## Gate status

- Planning: strict OpenSpec validation passes for all four active/canonical items; diff whitespace check passes.
- Navigation/UI worker: focused Rust navigation units 5/5, native navigation process fixtures 2/2, State assertions 40/40, native distribution checks, QML 25/25, Clippy with warnings denied pass.
- Backend worker: all focused new discovery, reconciliation, retirement, session echo and cancellation fixtures pass, including executing the actual peer-presence shell against an absent fixture-owned peer. Format check and all-target Clippy with warnings denied pass.
- Coordinator independently reran all three Node suites, production QML 25/25 and manifest validation successfully. Both documented synthetic configuration projections validate and retain mappings unchanged. The EOF-reading configuration CLI was checked with a regular fixture input descriptor; Node 26 synchronous `input` piping hit its deadline, so no runtime change or success claim relies on that harness behaviour.
- Independent Rust gate: 42 library and four binary tests passed. After the reviewed fixture correction and identity regression, the affected suites passed 17 process and two navigation tests. There are no remaining actionable review findings.
- Final strict OpenSpec validation passes 4/4 before archive and `git diff --check` passes. Locked offline release build succeeds. Parent-owned installed acceptance is recorded below.

## Independent review findings

1. Parallel process fixtures exposed an existing timestamp-only temporary-directory collision (`EEXIST`). Fixture names now include a per-process atomic sequence in both process and navigation suites. Runtime behaviour was unaffected; both affected suites passed independently after correction.
2. Identity refresh still truncated peer targets at four after fleet source capacity increased to sixteen. Identity collection now covers all sixteen bounded targets. A real CLI fixture places one mapped identity on the first target and another only on the fifth, confirms both survive the private-file refresh, deduplicates the third target and excludes unmapped second/fourth targets. The four-account mapping limit remains intentional. The reviewer inspected the correction and independently passed the final fixture.
3. A final bounded-write inspection exposed a missing-peer classification defect with existing large cursors. The reviewer reproduced actual `collection::remote` against an isolated missing-peer shell: 0/8 KiB returned `setup_needed`, while 64/128 KiB returned `Command input failed` in all three attempts. The missing-peer branch now consumes bounded stdin before returning its sentinel. Generic subprocess bounds, deadline and write-error handling remain unchanged. The reviewer independently passed the new actual-shell 64/128 KiB regression and existing missing-peer/last-good-inventory process fixture, then renewed approval. Format/Clippy and the locked offline release build pass after this delta.

## Frozen release

Final staged release is `/tmp/anton-fleet-release-final.lGt86b` for root's installed acceptance. It supersedes `/tmp/anton-fleet-release.ZJ5zOM`, whose initial binary hash was `284cae778a1c7f913bca4f5a745ef7b6e3ab1ea0f43bd450db7ace79d91d1bdf`. Only the binary changes between these stages. Local update list is `anton-runtime`, `Panel.qml`, `PopupContent.qml`, `State.js` and `README.md`. Both peers use the same final binary. The new peer additionally receives `uninstall-peer.sh` renamed to `uninstall.sh`, plus root-projected private config/marker/receipt. No private payload is in either stage. All six final files match frozen source/build bytes.

| Payload | SHA-256 |
| --- | --- |
| anton-runtime | `1be6785c3258f0363c83304a0c1ccd31f2b5a4bb6d7c20cc2c2666c4f08329cd` |
| Panel.qml | `17d464e7a6f7867365a1a5a64229b3857ca9b2b56e0028afecb6ec8d6a04fb6d` |
| PopupContent.qml | `fc7ce473577b34a8a524abcdf201e5145c5340a963d04add893b60ed15d4bb18` |
| State.js | `0957ec7e5d3457262ba0be1d4b16439fd38f353a0e1a8c3950637f2f4295265e` |
| README.md | `e2cc3f381d72c5d797c961f8e7d7d2d8a57f96081796ed65006c90c9d0a2f7cd` |
| uninstall-peer.sh | `3d99bd63e87e9bbc4ac322f7af899b0a8c45b916bd99a700796a799acd43668d` |

Source inventory: Rust `fleet`, `config`, `main`, `collection`, `allowances`, `identity`, `navigation`, `model`, `lib`, `native`; native process/navigation tests; the three UI files above and their JavaScript/QML checks; distribution check; root/plugin READMEs; this change's OpenSpec artefacts. Existing dependencies and unrelated source changes remain unchanged. Private configuration and peer receipts are migrated only by root's explicit rollout.

## Installed acceptance

Root deployed the final runtime to the existing peer and local plugin, then observed the new saved machine as “Setup needed” while its peer was absent. Root explicitly provisioned its plugin-owned peer with its exact profile id/session and only its existing mapped account, recorded ownership and appended the target once to the local removal receipt. Direct native probe and session echo passed; its allowance matched the existing Personal mapping. Without another restart, the new group became connected. The live plugin then reported three hosts, four threads and two deduplicated allowance rows.

The new machine's idle Codex thread had no native Herdr `agent_session.kind/value` binding. Usage/timing correctly remain unknown for that thread; this is not claimed as parser or complete-telemetry acceptance. Its existing Codex hook checksum is unchanged.

Root intentionally did not raise navigation windows during the user's active call. This change's stale/current exact-route behaviour is covered by the approved native CLI fixtures; previous live focus-handler evidence remains valid. The new machine's actual profile/session transport was verified by the native probe.

Root explicitly approved final installed acceptance. All five local payload hashes match the final stage; the owner-marker inode, `.accounts.json` bytes and account mapping remain unchanged. Warm restart passed with one current native collector and two unique provisioned-peer receipts. Three hosts are connected, four threads are present and both allowances are available.

`privacy.ini` changed during live use after rollout; root did not edit it, and no byte-identical-preference claim is made. Current concealment/collapse settings are preserved rather than restored from an older snapshot. The attempted installed screenshot did not establish the new machine's visible row and a later capture raced rendering, so no new live visual acceptance is claimed. Production QML fixtures and the synthetic captures above establish presentation; live structured diagnostics establish discovery and transport. Root deleted its screenshots.

## Lifecycle completion

All seven tasks are complete. Both added requirements and all eight scenarios were synchronised verbatim into their existing canonical capabilities, preserving prior requirements. Strict validation passed 4/4 before archive and 3/3 afterwards; the active change list is empty and the whitespace diff check passes. The change is archived as `2026-09-28-discover-herdr-fleet-profiles`.

Both source-owned release stages, the synthetic visual captures and source-baseline scratch directory were removed after installed acceptance. Worker fixture/proof directories clean themselves up; the ordinary ignored Cargo build cache remains. Root removed its private rollout rollback and account-check temporary files. Pre-existing private history and unrelated dirty work are preserved. No Git/forge operation was performed.

## PR integration review, 29 September 2026

PR #14 preserves the installed native migration and all completed follow-up
archives in Git. Independent native review reproduced a wrong-session local
navigation defect using only mocked executables: a configured non-default source
could focus the same pane identifier in `default`.

The correction in `main.rs`, `navigation.rs` and `State.js` enforces the existing
exact-route contract. Every configured source carries an opaque observation
binding. Activation revalidates configuration, preserves the selected local
session/executable, and requires a unique enabled saved target/session for SSH.
Missing, changed and socket-only routes fail before control. Window matching uses
the exact local session. Paths and targets remain outside the projected binding.

`tests/native_navigation.rs` covers exact named/default local sessions, custom
local IDs, hostname-colliding SSH IDs, stale and missing bindings, socket-only
sources and absent/ambiguous/changed saved routes. `test_omarchy_state.cjs` covers
opaque configured binding arguments and malformed identities; navigation unit
fixtures verify that a window from another local session is never selected.
All six navigation process fixtures and 51 JavaScript cases pass. The full serial
native suite, production QML fixtures (25), manifest and three canonical OpenSpec
specifications pass; formatting and all-target Clippy deny warnings.

This is repository delivery, without installed payload/config changes or a
plugin restart. The installed files matched the preserved migration before the
review correction; that correction is source-only in this PR. Existing historical
live acceptance remains separate from these synthetic regression results.

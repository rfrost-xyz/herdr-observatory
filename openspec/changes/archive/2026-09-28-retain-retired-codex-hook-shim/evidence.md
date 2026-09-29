# Evidence

## Confirmed regression and immediate recovery

Root inspected the rendered hook failures, live process command and current on-disk hook state without recording private terminal content or session identifiers. Existing Codex processes still invoked the exact former Observatory shell command after migration removed the helper. The current hooks file contained only native Herdr session-start registration; listing current hooks did not reload those processes. Native Herdr's current hook succeeded under all checked process environments.

Before recovery, root verified the owned native migration backup contained the exact former command, the current runtime and owner were valid, the Pi extension hash matched the existing receipt, and the legacy helper directory was absent. Independent review approved this exact inert payload:

```sh
#!/bin/sh
# herdr-observatory retired Codex hook v1
cat >/dev/null 2>/dev/null || :
exit 0
```

SHA-256: `0b6dabc77c3845f29cc93defaff8b38b8c79319254e1d7e321997cdeec5a5b73`.

Root created only the private helper directories/file and added the compatibility path/hash to the existing receipt. Current hooks configuration and user processes were left intact. Independent and installed smoke checks consumed empty, 64 KiB and 1 MiB inputs, each completing within three seconds with exit zero and no stdout/stderr. Both previously affected long-running Codex sessions subsequently made natural tool/wait calls without further hook failures. No process or shell restart was needed.

## Durable source acceptance

The implementation is restricted to native hook installation, repair and uninstall, the command dispatcher, focused process fixtures and lifecycle documentation. Collection and QML code are unchanged. Recovery records receipt intent before creating an absent helper through an exclusive, no-follow open relative to a verified directory. It preserves a concurrent conflicting file. Explicit uninstall preflights both Pi and compatibility bytes before deleting either.

Nine focused hook unit tests passed, covering migration, proof recovery, input draining, idempotency, conflicts, symlink ancestry, failed receipt writes, absent-intent retry, legacy-preimage retry and uninstall preflight. The actual repair CLI and complete local/peer uninstall fixture passed. Formatting and all-target Clippy with warnings denied passed. Existing Pi, state and distribution JavaScript suites passed, as did strict change validation and whitespace checks.

The locked offline release build passed. Independent frozen-source review approved the change with no actionable findings. Its full serial locked/offline native suite passed 49 library, four binary, two navigation and 18 process tests. An initial default-parallel run failed an existing checkpoint ownership fixture; its isolated rerun and the full serial gate passed. The cause remains unconfirmed, and no unrelated collection changes were made. Strict validation passed all four current spec/change items.

Reviewed staged payload hashes:

- `anton-runtime`: `27ce74652b0340fd60d645e3ed93b84aa0dfb29773e7b817e462327be4a64860`.
- Installed `README.md`: `7cfce7a413aa02f5236f4ae016671f2a26f9f35c77f2f96653ed3129aafdbb69`.

Both staged files byte-match their build/source counterparts. Root atomically installed only these two files and independently verified their exact hashes. Installed `--repair-retired-hooks` exited zero idempotently. Before/after hashes remained identical for current hooks, Pi extension, hook receipt, migration backup, compatibility shim, private configuration and account mapping. The installation marker inode was unchanged. No processes were restarted, and both old sessions' natural cached calls had already been verified quiet.

The existing resident collector may retain its previous executable inode until its normal restart. Collection code is unchanged; the newly installed on-demand repair and uninstall commands use the reviewed binary. No remote update, live uninstall or new visual check was required. Actual destructive local/peer uninstall behaviour was verified against synthetic installations.

## Closure

The added requirement was synchronised verbatim into the canonical Omarchy specification. All five tasks are complete. The change was archived on 2026-09-28; canonical strict validation passed all three specifications, no active changes remain, and whitespace checks passed. The two-file release stage was removed after installed acceptance. Root removed its private rollout backup and diagnostic screenshots. Existing dirty work and normal Cargo build caches were preserved; no Git or forge operations were performed.

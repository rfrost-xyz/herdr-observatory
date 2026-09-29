# Native release deployment and rollback

This is the local maintainer runbook for the reviewed release. Run only after
source acceptance. The parent owns all installed and remote changes. Commands
use private staging files; do not print configuration, identities or snapshots.
No container, web service, generic Anton installation or Herdr integration is
updated by this deployment.

## Release contents and preflight

Build `omarchy/herdr.observatory/build-native.sh RELEASE/anton-runtime` with the
locked offline Cargo build. Copy source `uninstall-peer.sh` as
`RELEASE/peer-uninstall.sh`. The local payload set is the native binary, all QML
files, `State.js`, `manifest.json`, `README.md` and `uninstall.sh`. No Python,
zip archive, web bundle, broker or build tooling is installed. Hash every staged
file and compare installed hashes after replacement.

Before mutation, preserve private mode-600 rollback copies of the installed
configuration, accounts, plugin payloads, current Codex hooks and Pi extension
on each affected host. Record the local owner marker device/inode and the hashes
of `.accounts.json` and state `privacy.ini`. Check whether any touched external
hook/config path is chezmoi-managed; the native hook installer refuses managed
paths. Do not silently overwrite a conflict.

## Private configuration migration

Use the staged binary, keeping stdin/stdout private:

```sh
umask 077
"$release/anton-runtime" --migrate-config < "$plugin/.config.json" > "$private_stage/config.json"
```

This validates the result and removes top-level `publish`/`music`, retired
host metric selectors, `allowances.publish` and each old source `container`.
Host IDs/SSH targets, account hash keys and their mappings remain intact. Compare
those projections privately before replacing the config. Preserve the existing
`.accounts.json`, owner marker inode and privacy preference without rewriting.

## Peer first

The remote root is exactly `$HOME/.local/share/herdr.observatory-peer` and its
state is `$HOME/.local/state/herdr.observatory-peer`. Both were absent before
this change. Abort if either has appeared unexpectedly. The existing generic
`$HOME/.local/share/anton` is outside this deployment.

Prepare a minimal private peer configuration from the reviewed local config and
confirmed remote Herdr selector, selecting the remote host ID unchanged:

```json
{
  "hosts": [{"id": "REMOTE_HOST_ID", "herdr": "/home/USER/.local/bin/herdr"}],
  "allowances": {"accounts": {"EXISTING_VERSIONED_ACCOUNT_HASH": "Work"}}
}
```

Preserve the original remote `herdr` selector and its project-root settings;
drop its SSH `target` and set its peer transport to local. Parent verified the
remote CLI snapshot works and did not discover a socket at the guessed runtime
path, so this deployment must not invent a `socket_path`.

The hash and mapping are copied exactly from the existing explicitly configured
remote allowance mapping. A structured mapping is also copied unchanged. Never
copy local-only host paths, other account mappings, SSH sources, publication
configuration or authentication files. No `.accounts.json` or email needs to be
installed on the peer. Validate this projection with `--migrate-config` too.

Provision only the reviewed binary (755), peer-uninstall.sh as `uninstall.sh`
(755), `.config.json` (600), and marker `.herdr-observatory-install` containing
`herdr.observatory` plus newline (600), in a new mode-700 root. Then run on peer:

```sh
"$HOME/.local/share/herdr.observatory-peer/anton-runtime" --record-peer
"$HOME/.local/share/herdr.observatory-peer/anton-runtime" --install-hooks --adopt-legacy-hooks
```

The explicit adoption flag is needed because the former peer Python helper had
another install path. The installer recognises the old Observatory marker,
removes only its exact Codex shell callbacks and marked helper files, and writes
the native Pi extension plus a hash receipt. Unrelated/native Herdr callbacks
remain. Preserve the old hook/extension copies for rollback until acceptance.

Verify peer `--probe` with private JSON stdin
`{"version":1,"host_id":"REMOTE_HOST_ID","cursors":{}}`, then standalone
`--allowances-probe` with `{}` stdin. Inspect only sanitised aggregate results.
Source timestamps must be original, and allowances must work with no active
thread. A failed native probe blocks local migration.

After successful provision, atomically record the unique exact SSH target in
local mode-600 `.peers.json`, version 1, `targets` array. This receipt is required
for complete uninstall and must survive partial remote failure.

## Local replacement

Replace the reviewed local payloads and private migrated config. Keep the owner
marker inode unchanged. Run the installed native binary with `--install-hooks`
(no adoption flag needed when the previous local helper names this same plugin
root). It removes only proven-owned legacy Observatory callbacks and installs
the native Pi reporter, preserving Herdr and unrelated callbacks.

Remove these former local payloads only after their expected ownership and
replacement have been checked:

- `runtime.py`, `runtime.zip`, `native-adapter.py`
- `open-thread.py`, `refresh-allowances.py`, `refresh-identities.py`
- Any old installed `build-native.py` or `build-runtime.py`, if present and known
  to this plugin. Build scripts are not runtime payloads.

Do not delete unknown files or private configuration/state. The hook installer
removes its proven-owned old `codex.sh`, `codex_usage.py` and
`allowances_probe.py` under `~/.local/share/herdr-observatory/hooks`; remove any
remaining obsolete adapter state only after parent ownership inspection.
Copy `SnapshotStore.qml` last, then perform one authorised Omarchy warm restart.

## Acceptance and rollback boundary

Confirm both hosts, all expected thread metrics/source ages, account allowances,
exact clicked-thread navigation through the selected Herdr instance, unchanged
popover, zero Python/broker children, and zero forwarding subprocesses. Confirm
warm restart removes the old native process identity and leaves one collector.
Recheck private account/privacy hashes and owner inode. Profile the same local
process-family scope as the recorded baseline; remote CPU and Qt/GPU are excluded.

Before acceptance, rollback first removes the newly provisioned peer through
its receipt-bound uninstaller while its native Pi extension still matches the
receipt. Then restore the original peer hooks/extension. Separately restore the
saved local hybrid payloads/private config and original local hook files, and
restart the shell. Restoring the peer extension before native removal would
correctly trigger an ownership conflict. The user-authorised removed
publication/music keys must stay removed. Preserve backups until the restored
runtime is verified. After acceptance, discard only task-owned private rollback
copies and release staging.

## Complete uninstall

Run the installed local `uninstall.sh`. It first calls each recorded peer's
`uninstall.sh`, persists partial progress, then disables/removes the local plugin.
A missing connection or ownership conflict retains the local installation and
remaining receipt for retry. On the peer, `uninstall.sh` can also be invoked
directly. Retired retries finish owned state deletion, preserve unknown state,
and remove only recognised payloads. Existing web containers and the unrelated
historical Anton application remain untouched.

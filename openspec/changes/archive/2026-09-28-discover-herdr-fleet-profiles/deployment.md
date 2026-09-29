# Reviewed deployment contract

This document uses synthetic identifiers. Resolve the current saved profile privately from `herdr machine list --json`; never copy live profile IDs, targets, account hashes or email into source evidence. Root owns installed and remote writes.

## Private local configuration

Preserve every existing account mapping, local host selector, project root and preference. Add `fleet_discovery: true`. For each existing peer-backed host, set `profile_id` to its exact saved Herdr profile id while retaining the host's existing `id`. Bind its existing explicit allowance source with that same `profile_id`. The saved profile becomes authoritative for target/session. A synthetic fragment is:

```json
{
  "fleet_discovery": true,
  "hosts": [
    {"id":"desktop","transport":"local","session":"default"},
    {"id":"workstation","profile_id":"saved-workstation","transport":"ssh","target":"user@workstation"}
  ],
  "allowances": {
    "accounts": {"<existing-private-hash>":"Personal"},
    "sources": [{"target":"user@workstation","profile_id":"saved-workstation"}]
  }
}
```

Use the release executable's `--migrate-config` with private JSON stdin to validate the final configuration before replacing it atomically at mode 600. Preserve `.accounts.json`, `privacy.ini`, checkpoint ownership and the existing `.herdr-observatory-install` inode. Do not replace the owner marker during a payload update.

## New peer projection

Verify compatible Linux architecture/libraries and absence of conflicting peer paths. Use only `~/.local/share/herdr.observatory-peer` and its matching state directory. Do not use a generic Anton application path. The remote `.config.json` contains a local host with `id` equal to the new saved profile id, the exact current session, the verified local Herdr executable selector and only that machine's explicitly permitted existing account mapping. Omit local-desktop paths, sources, fleet discovery and unrelated mappings. For example:

```json
{"hosts":[{"id":"saved-new-machine","transport":"local","herdr":"/usr/bin/herdr","session":"default"}],"allowances":{"accounts":{"<existing-private-hash>":"Personal"}}}
```

Provision only the reviewed native `anton-runtime`, `uninstall-peer.sh` copied as `uninstall.sh`, this validated private config and the exact owner marker `herdr.observatory\n`. Directory mode 700, binary/uninstaller 755, config/marker 600. Run the peer's `--record-peer` to record owned payloads. Codex collection uses native Herdr session identity and requires no new Codex hooks. Install Pi reporting only when that harness is required, preserving unrelated integrations.

Record the successfully provisioned target once in the local private `.peers.json` receipt. Preserve existing targets. A removed/disabled profile is not a reason to delete this receipt: it remains the authority for explicit uninstall.

## Acceptance and rollback

Update existing peers to the reviewed native binary before enabling discovery: session-selected probes now require the same session echoed in the response, and an older peer that ignores the selector is rejected. Install the reviewed local UI/runtime delta and private discovery bindings while the new peer is still absent, restart once, and verify its machine group says “Setup needed”. Then explicitly provision it and verify native `--probe` with bounded JSON stdin (`version`, exact `host_id`, exact `session`, empty `cursors`), plus independently mapped `--allowances-probe`. Confirm the group becomes connected automatically without another restart. Verify all host data, exact navigation, mapped allowances, owner lifetime, private fingerprints and payload hashes. Verify automatic profile reconciliation using synthetic process fixtures and safe live inventory observation; do not alter unrelated user profiles merely to exercise a test.

Retain a private local payload/config rollback until acceptance. For an existing peer, preserve its earlier reviewed executable for rollback; for a new peer, use its receipt-bound `uninstall.sh` if rollout must be reverted. No service, container or unrelated application changes belong to this rollout. The complete local uninstall first removes all receipt-recorded peers and retains the remaining receipt if any removal cannot finish.

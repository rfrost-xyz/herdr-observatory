# Anton for Omarchy

Anton is a menubar icon and compact themed popover for Herdr threads and Codex
allowances. It has no companion window or web application.

## Runtime

QML owns `anton-runtime`. The Rust collector reads local Herdr, validates bounded
Codex session records and invokes the same executable on explicitly configured
SSH peers. It stops on owner-pipe closure. No Python, Docker, listening service,
independent autostart or music forwarding is required.

The collector writes one JSON snapshot per line. It emits when state changes and
otherwise every `heartbeat_seconds` (4 s), a value carried in each snapshot so
the popover can derive how long to keep a snapshot without a new one. The
coordinator waits at most 1 s for events between heartbeat checks.

The owner pipe (the collector's stdin) accepts newline-delimited commands of at
most 64 bytes. The only command is `refresh`. Longer lines, invalid UTF-8 and
unknown commands are ignored; EOF stops the collector. `refresh` wakes only local
Herdr sampling, recomputes allowances from the local cache without a Codex call
and emits once the fresh local samples arrive, typically within about a second.
Refreshes are honoured at most once every 2 s; requests in between coalesce into
one. SSH peers, account reads and fleet discovery keep their own cadence. The
popover sends `refresh` for `r`, middle-click and the IPC `refresh` call.

The collector no longer reads the Omarchy theme; the popover follows the theme
itself. Existing `theme_host` and per-host `theme_path` settings are still
validated as before so installed configurations keep loading, and are otherwise
ignored. Peers report `theme` as `null` for compatibility with older plugins.

The unchanged popover shows context, input/output, cache composition, subagent
outcomes and turn time. Missing data stays unknown. Codex collection does not
need Observatory hooks; Herdr's native integration supplies session identity.
The optional Pi extension calls the native reporter for supported live metrics.

## Build and local install

Build inputs are Cargo plus dependencies pinned in `../anton-runtime/Cargo.lock`.
The build is offline and installs no packages. Omarchy, Herdr and ordinary SSH
must already be available. The installed plugin does not need Cargo.

```sh
omarchy/herdr.observatory/build-native.sh /tmp/anton-runtime
ANTON_CONFIG=/absolute/private/config.json omarchy/herdr.observatory/install.sh
```

Installation refuses an existing plugin. Updates must preserve `.config.json`,
`.accounts.json`, `.peers.json`, private state and the existing owner marker inode.
The example below uses synthetic paths. Set host IDs to the actual configured
Herdr machine IDs so navigation resolves the same exact machine. Enable automatic
fleet discovery to follow saved enabled profiles without restarting the plugin.

```json
{
  "interval": 5,
  "fleet_discovery": true,
  "hosts": [
    {"id": "desktop", "socket_path": "/run/user/1000/herdr/main.sock"},
    {"id": "workstation", "profile_id": "saved-workstation", "transport": "ssh", "target": "user@workstation"}
  ]
}
```

Account mappings are private, keyed by the existing versioned account hash. Each
mapping may retain its legacy Personal/Work label or an explicit stable ID/label/
category. Explicit remote `allowances.sources` contain an SSH `target` and may
bind to a saved machine through `profile_id`. The native reader
refreshes mapped accounts independently of whether any thread is active. It uses
read-only Codex app-server RPCs and never parses auth files, redeems resets or
changes login. Unsupported information remains unavailable.

## Automatic fleet discovery

With `fleet_discovery: true`, Anton checks `herdr machine list --json` every ten
seconds. Enabled profiles appear automatically; removing or disabling a profile
stops its thread and allowance collection. Label changes keep the same machine
identity. Target or session changes retire the old readings before collecting
the new route. Discovery errors retain the last accepted inventory and show a
concise unavailable indicator. A reachable machine without its native peer says
“Setup needed”. Discovery never installs or removes remote files.

Update existing native peers to the same reviewed release before enabling
discovery. Session-selected probes require an exact session echo; an older peer
that ignores that selector is rejected rather than displaying the wrong session.

For a new profile, provision its peer with the saved profile's exact `id` as its
local host id. For an existing deployment, add `profile_id` to the existing host
override so its current host id, peer configuration and project roots remain
compatible. Bind an existing explicit allowance source to the same profile id.
The current saved profile supplies target and session. A bound override remains
inactive when its profile disappears, including after a restart. Keep the
profile binding when disabling or removing the machine; deleting the binding
would turn the override back into an explicitly configured static source.

Enabled discovered peers are also allowance sources, even with no threads.
Both peer and local account mappings must already permit the account. Discovery
does not create account mappings; the same account on several machines still
appears once. Unbound static hosts and sources remain explicitly configured.
Omitting or disabling `fleet_discovery` keeps the fixed configuration behaviour.

Use the installed executable's `--refresh-identities` command to refresh verified
emails into `.accounts.json`. Clicking any allowance conceals every email using
locally persisted aliases. Concealed identities stay out of tooltips and
accessibility text. `--refresh-allowances` requests a bounded local account
refresh without starting an agent; periodic remote account reads run independently.

## Native peer

A peer is required for native remote collection. Provision the reviewed executable
as `~/.local/share/herdr.observatory-peer/anton-runtime`, mode 755, and copy
`uninstall-peer.sh` beside it as `uninstall.sh`, mode 755, with a private
`.config.json` and `.herdr-observatory-install` containing `herdr.observatory`.
The peer configuration defines its own local Herdr source with the same ID as the
calling plugin's remote host (the saved profile id for newly discovered hosts),
plus the explicitly allowed account mappings. Its session must match the saved
profile. Use the peer's local Herdr executable or socket selector, never a local
desktop path copied from another machine.
It does not copy local-only paths or remote sources from another machine.

Use mode 700 for the directory and 600 for private files. The generic
`~/.local/share/anton` may belong to another application and is never used.
After explicit provision, run the peer executable with `--record-peer` and
`--install-hooks --adopt-legacy-hooks` to record ownership and migrate only proven
Observatory integrations. Native Herdr and unrelated hooks are retained. The peer
state directory is `~/.local/state/herdr.observatory-peer`.

The plugin invokes fixed `--probe`, `--allowances-probe`, `--identity-probe` and
user-initiated `--focus` commands over SSH, with variable selectors in bounded
JSON stdin. No helper daemon or remote web container is started. A missing or
incompatible peer becomes unavailable without falling back to another host.
A missing executable is distinguished as “Setup needed”. Removing a profile from
Herdr stops observation but leaves a provisioned peer and its uninstall receipt
intact, so explicit removal remains possible.

Record successfully provisioned peers in the local private `.peers.json`:

```json
{"version":1,"targets":["user@workstation"]}
```

Targets must be unique and correspond to helpers provisioned for this plugin.
Keep this receipt until removal succeeds.

## Migration from earlier builds

Run the new binary's `--migrate-config` with the old private JSON on stdin and
write its validated output to a private staged file. It removes retired
`publish`/`music` keys and known Docker exporter selectors, retaining host/account
configuration. Preserve `.accounts.json` and the state `privacy.ini` unchanged.
Do not keep old Python/runtime.zip payloads in the installed directory once native
parity and the private configuration migration are accepted.

Existing remote web containers are outside this plugin's ownership and are left
untouched. This plugin no longer forwards Work data, allowances or music to them.

### Existing Codex sessions after migration

An already-running Codex process may retain its old Observatory hook command even
after current hook registration is removed. Migration keeps a tiny inert shell
helper at the former `~/.local/share/herdr-observatory/hooks/codex.sh` path for
those callers. It drains stdin to `/dev/null`, produces no output and exits
successfully. It reads no configuration or credentials and performs no collection
or forwarding. New hooks are not registered.

If an earlier native migration already removed that helper, recover it with:

```sh
~/.config/omarchy/plugins/herdr.observatory/anton-runtime --repair-retired-hooks
```

Recovery requires the native installation's owned hook backup to prove the exact
retired command and its current ownership receipt to match the native Pi
extension. It creates only the compatibility helper and updates its receipt;
current hook configuration and running sessions remain intact. Conflicting or
changed files and symlinks are preserved and reported. The helper remains until
explicit uninstall, which removes it only while its recorded path and hash still
match. An empty current hooks list does not prove old processes have stopped
using the command.

## Complete uninstall

```sh
~/.config/omarchy/plugins/herdr.observatory/uninstall.sh
```

This first removes receipt-recorded native peers, then disables the local plugin,
removes its still-owned Pi extension, retires checkpoint writers and removes only
known plugin/state files. It preserves unrelated hooks, files and applications.
An unreachable or conflicting peer stops removal and retains the local plugin and
remaining receipt so the same command can be retried. Unknown local files also
stop removal. No remote container or unrelated Anton installation is deleted.

For a separately provisioned peer, its explicit standalone removal command is:

```sh
~/.local/share/herdr.observatory-peer/uninstall.sh
```

## Development checks

```sh
cargo fmt --manifest-path omarchy/anton-runtime/Cargo.toml --check
cargo clippy --manifest-path omarchy/anton-runtime/Cargo.toml --locked --offline --all-targets -- -D warnings
cargo test --manifest-path omarchy/anton-runtime/Cargo.toml --locked --offline
node --test tests/test_pi_hooks.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs
bash tests/run-qml.sh
omarchy-plugin-validate omarchy/herdr.observatory
openspec validate --all --strict
```

Production QML fixtures exercise the actual `PopupContent.qml`; include their
layout/keyboard/reduced-motion checks and `omarchy-plugin-validate` for UI changes.
Resource comparisons must include all local children, state their scope and avoid
claims about unmeasured remote CPU or Qt/GPU work.


Thread activation requires the opaque route binding from its observation. Local
and static SSH routes are revalidated against current configuration; saved
profiles are revalidated against the current Herdr inventory. A changed route or
missing binding fails before focus. Local named sessions use that exact session
and configured executable. Socket-only collection has no verified terminal route,
so activation reports an error without attempting another session.

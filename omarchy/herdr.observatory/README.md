# Anton for Omarchy

Anton is a menubar icon and compact themed popover for Herdr threads and Codex
allowances. It has no companion window or web application.

## Runtime

QML owns `anton-runtime`. The Rust collector reads local Herdr, validates bounded
Codex and Claude Code session records and invokes the same executable on explicitly configured
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
outcomes and turn time. Missing data stays unknown. Codex and Claude Code
collection do not need Observatory hooks; Herdr's native integration supplies
session identity. Claude Code transcripts are read from
`$CLAUDE_CONFIG_DIR/projects` or `~/.claude/projects`. Transcripts carry no
context window, so the Claude Code context dial needs the Claude Code mod below;
without it the window and percentage stay unknown.
The optional Pi extension calls the native reporter for supported live metrics.

## Claude Code context window mod

The local install adds a small Claude Code mod, `anton-observatory`, as a
personal skills-directory plugin in `~/.claude/skills/anton-observatory/`
(`.claude-plugin/plugin.json`, `hooks/hooks.json` and `hooks/register.js`). It
needs Claude Code 2.1.287 or later, which loads it as
`anton-observatory@skills-dir` with no `settings.json` or `enabledPlugins` edit.
After each turn and at session start it runs the installed `anton-runtime` with
the Herdr pane, a sequence, the session id and the context window as arguments.
It sends no model, prompt, message, cost, rate limit or account data, never
blocks or changes the event, and runs only inside a Herdr pane. The reporter
writes the window to that pane's metadata only when Herdr binds the pane to the
same Claude Code session; the collector then shows the replay context as a
percentage of that window. Peers never install the mod and ignore any report on
their panes, so remote Claude Code threads show no window or percentage.

The mod loads only at the next session start or after `/reload-plugins`. A
session that was already running stays unknown until it reloads, as does any
session in which mods are off (for example an untrusted workspace,
`disableAllHooks`, `--bare` or `--safe-mode`).

The files and their hashes are recorded in the plugin's `.hooks-receipt.json`.
Installation refuses a target it cannot prove absent or owned, symlinks, a set
`CLAUDE_CONFIG_DIR` other than `~/.claude`, a target (or, for chezmoi, its
`~/.claude` or `~/.claude/skills` parent) managed by chezmoi, mise
dotfiles or a Git repository with a real `.git` marker. A refusal leaves the
plugin installed and the dial unknown. To remove only the mod:

```sh
~/.config/omarchy/plugins/herdr.observatory/anton-runtime --uninstall-claude-mod
```

Removal refuses, keeping every file and the receipt, when a recorded file was
changed; restore or delete that file and retry. A directory holding a file Anton
did not write is kept and reported. The complete uninstall below also removes
the mod.

If mods are withdrawn or cannot load, the documented fallback is a Claude Code
`statusLine` wrapper that passes the session id and context window size to the
same reporter. It is not built, because it needs an edit to `settings.json`.

## Popover structure

The popover is plain QML and JavaScript in this directory:

- `Panel.qml` is wiring only: the bar button, `KeyboardPanel`, the `.accounts.json`
  identity file and the `herdr.observatory` IPC target (`open`, `close`, `toggle`,
  `refresh`, `status`, `diagnostics`).
- `SnapshotStore.qml` owns the collector process and the projected view.
- `State.js` holds every pure rule: projection, staleness, readings, formatters,
  focus, acknowledgements and keyed-list edits. `tests/test_omarchy_state.cjs`
  covers it.
- `AntonTheme.qml` reads the status palette from `colors.toml` in the theme
  directory the shell reports as current (`Color.currentThemePath`) and derives the
  ink, muted, line and state colours.
- `AntonPreferences.qml` wraps the unchanged `privacy.ini` settings (same keys,
  JSON-text encoding and one-time migration) and parses each value once.
- `AntonController.qml` holds navigation, completion acknowledgements, keyboard
  focus, filters and collapse, identity concealment and refresh requests.
- `PopupContent.qml` and the card components receive only the typed values and
  callbacks they use.

The projected view is structural: it changes when a snapshot arrives, when the
collector restarts or drops, or when a freshness boundary passes. Ages, the
running turn stopwatch, the reset countdown and expected allowance are readings of
that view at the store's `now`, which advances every second while the popover is
open. Staleness rules are unchanged and also apply while the popover is closed.

Machine groups, threads, providers and accounts are keyed Repeater rows
(`AntonKeyedModel.qml`), so a row keeps its element and any running highlight when
other rows appear, disappear or reorder. Only a thread that newly appears on a
reporting machine plays the entrance; hydration, reconnect, filtering, collapse
and sorting do not. One shared `AntonToolTip` shows every hover hint with the
same delay, timeout, placement and suppression while scrolling.

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
After updating the installed runtime, run
`~/.config/omarchy/plugins/herdr.observatory/anton-runtime --install-claude-mod`
to install or refresh the Claude Code mod; an unchanged mod is left as it is.
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

### Allowance rows

Each snapshot `allowances` entry is one provider-neutral row per configured
account, in a stable order sorted by account key. Every key is always present,
with `null` where unknown:

```json
{"provider":"codex","provider_label":"Codex","account_id":"Personal",
 "label":"Personal","status":"available","status_text":null,"plan":"pro",
 "sampled_at":1800000000.25,"reset_count":1,"reset_expires_at":null,
 "windows":[{"kind":"weekly","label":"Weekly","used_percent":40.0,
 "resets_at":1800302400,"duration_s":604800,"pacing":true}]}
```

- `status` is `available`, `unavailable` or `auth_needed`. A row that is not
  available has `windows: []` and null `sampled_at`, `plan`, `reset_count` and
  `reset_expires_at`. `status_text` is null or 1 to 80 characters supplied by
  the source; Codex always sends null.
- `sampled_at` is the original source time. An observation older than 600 s,
  or more than 1 s in the future, makes the account unavailable.
- `windows` holds at most eight windows. `used_percent` is 0 to 100,
  `resets_at` is Unix seconds and `duration_s` is the source's window length.
  Exactly one window has `pacing: true`; the popover projects balance and pace
  from that window only and never from list order. A past reset nulls
  `used_percent` and `resets_at`; pass expiry nulls only `reset_count` and
  `reset_expires_at`.
- Codex rows always carry one `weekly` window of 604800 s. The runtime selects
  it by its 10080-minute duration, never by field order, and reports
  `reset_count` from the native `availableCount`.
- Account token activity (`lifetime_tokens`, `peak_daily_tokens`,
  `daily_usage`) is still collected and cached but is not part of the popover
  row.

The private `allowances.json` cache and the peer `--allowances-probe` output keep
their earlier Codex source shape (`weekly_remaining`, `weekly_resets_at` and the
token activity fields). An updated plugin therefore reads caches written by
earlier builds and rows from peers that have not been updated, and an earlier
plugin still reads what an updated one writes. Extra peer fields such as `theme`
or `email` are dropped. The runtime converts source rows to the row above in one
place, `allowances::snapshot`.

Omarchy's own agents plugin records a provider as `limits: [{label, percent,
resetsAt}]` with `tierLabel` and `usageStatusText`. A window `label` maps to
`label`, `percent` is `used_percent / 100` and `resetsAt` is `resets_at` as
ISO 8601 UTC; `plan` maps to `tierLabel` and `status_text` to
`usageStatusText`. That record has no duration, kind or pacing, so consuming it
needs a source-supplied duration or pace stays unknown, and reset passes are not
representable. Design D7 of the `generalise-anton-allowance-windows` OpenSpec
change has the full table. No adapter is implemented.

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
removes its still-owned Pi extension and Claude Code mod, retires checkpoint
writers and removes only known plugin/state files. A changed mod or Pi extension
file stops removal with the plugin and receipt intact, so the command can be
retried. It preserves unrelated hooks, files and applications.
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
node --test tests/test_pi_hooks.mjs tests/test_claude_mod.mjs tests/test_omarchy_state.cjs tests/test_native_distribution.mjs
bash tests/run-qml.sh
bash tests/run-qmllint.sh
bash tests/run-shell-harness.sh
omarchy-plugin-validate omarchy/herdr.observatory
openspec validate --all --strict
```

`tests/run-qml.sh` fails on QML binding errors (`TypeError`, `ReferenceError`,
`Unable to assign`) as well as on test failures. Set `QMLTESTRUNNER` to use a
particular Qt 6 build; CI uses Qt 6.8.3. It runs with a private temporary
directory for preference and palette files, never the user's configuration.

`bash tests/run-qmllint.sh` lints the plugin QML against the repository stubs
(`tests/qml/anton`) and Qt's own modules only. Every file except `Panel.qml` must
be free of warnings; `Panel.qml` may only report the missing `qs.Ui` module and
its consequences. A member or property that Panel names on a plugin, stub or Qt
type still fails the gate, as does a child object placed inside one. It supports
Qt 6.8 (as in CI) and Qt 6.11 or later, and refuses Qt 6.9 and 6.10. Set
`QMLLINT` to use a particular Qt 6 build.

`bash tests/run-shell-harness.sh` loads `Panel.qml` in Quickshell, offscreen,
against the installed Omarchy `qs.Commons` and `qs.Ui` modules, which the QML
suite replaces with stubs. It hot-reloads the widget, opens, closes and reopens
the popover and leaves it open unattended, then checks for a visible card with
one row per thread and fails if the runtime was asked to open a thread. The
runtime is a fake that serves a synthetic snapshot, HOME and the theme palette
are synthetic, and `KeyboardPanel` runs as a floating window because offscreen
has no layer-shell backend, so compositor focus and pointer input are not
covered. It is a local-only gate that CI does not run, and it skips when
Quickshell or the shell modules are not installed; set `OMARCHY_SHELL_DIR` to
choose the shell directory.

The QML tests exercise the real `PopupContent.qml`, theme, preferences (against
captured `privacy.ini` files), controller, keyed rows and shared tooltip with
synthetic data and fixed colours; include their layout/keyboard/reduced-motion
checks and `omarchy-plugin-validate` for UI changes. The screenshot tests write
`/tmp/anton-continuity-*.png`; their bytes also depend on the text that earlier
test files rendered in the same process.
Resource comparisons must include all local children, state their scope and avoid
claims about unmeasured remote CPU or Qt/GPU work.


Thread activation requires the opaque route binding from its observation. Local
and static SSH routes are revalidated against current configuration; saved
profiles are revalidated against the current Herdr inventory. A changed route or
missing binding fails before focus. Local named sessions use that exact session
and configured executable. Socket-only collection has no verified terminal route,
so activation reports an error without attempting another session. Activation
comes only from a key, tap or accessibility press while the popover is open; the
controller ignores it once the popover has closed, including during the fade.

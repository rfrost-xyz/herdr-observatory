# Design

## Context

See proposal.md for the motivation. Current state at `53f2407`:

- **`Panel.qml` (456 lines).** It owns:
  - the palette `FileView` on a hard-coded `~/.local/state/omarchy/current/theme/colors.toml`;
  - `Core.Settings` (`privacy.ini`), with JSON strings re-parsed inside bindings (17 parse calls in QML);
  - aliases, acknowledgements, the navigation `Process`, focus, IPC, and formatting (`tokens`, `percentReading`, `paceText`, `parseList`, `parseObject`, `alpha`, `stateColour`).
- **Components.** Seven components declare `required property var ui`, and QML references `ui.` 149 times. `tests/qml/anton/FixtureUi.qml` re-implements that surface for the screenshot tests.
- **`SnapshotStore.qml`.**
  - It calls `State.project(raw, Date.now())` on each receipt and, while open, on every 1 s tick.
  - It also runs `State.stableThreads`, `JSON.stringify` and a signature compare each time.
  - Host, usage, child, turn and allowance ages, the stopwatch, the reset countdown and the expected allowance are all baked into the view. The view is therefore replaced every tick: 60 per simulated open minute.
- **`PopupContent.qml`.**
  - Outer Repeaters take integer counts. Cards look up `ui.overview.threads[threadIndex]` and `provider.accounts[index]`.
  - Heights are sums of literals (95, 85, 0.55, 180, 300).
- **Tooltips.** `AntonSurface.qml` declares a `ToolTip` per surface: 6 per `ThreadCard`, and 29 in the standard fixture.
- **Screenshots.** The seven screenshot hashes in the archived changes are reproduced at `a7ff5c0` (evidence.md, Baseline). Five of them depend on `FixtureUi.qml`'s token formatter, which keeps a trailing `.0` that production strips (D12).
- **Constraints.**
  - Rust is out of scope. The release binary must be byte-identical to baseline SHA-256 `74f50d69…`, so peers and the wire contract cannot change.
  - The plugin loads from a flat directory. Components in that directory are available by type name, and every shipped file must be listed in `install.sh` and in both lists in `uninstall.sh`.
  - `qs.Ui` (Panel, KeyboardPanel, BarIconButton, PanelKeyCatcher) is unavailable to `qmltestrunner`, so `Panel.qml` cannot be instantiated in tests. It must therefore become thin wiring.

## Goals / Non-Goals

**Goals:**

- Split `Panel.qml` into a theme, preferences and a controller, each tested for real.
- Use typed narrow properties everywhere, with no `required property var ui`, and pass qmllint under the repository stubs.
- Put pure helpers in `State.js`, with node tests.
- Separate structural projection from the clock, preserving thresholds exactly, whether the popover is open or closed.
- Use keyed delegates, one tooltip and a layout-based `PopupContent`.
- Keep the seven screenshots byte-identical to the baseline rendering with production formatting (D12).

**Non-Goals:**

- Runtime changes.
- Snapshot or allowance wire changes.
- New visible text or new features.
- Omarchy `PanelToolTip` styling, whose different delay, padding and corners would change visuals (D10).
- Changing the preference file format.

## Decisions

### D1. Files and lane ownership

Two implementation lanes with disjoint files. The coordinator owns the planning artefacts, `evidence.md`, `tests/measure_anton_popover.mjs`, `tests/capture_popover_oracles.cjs` and `tests/fixtures/popover-*`, all of which are frozen after baseline.

| Lane | Files |
| --- | --- |
| A (state and store) | `omarchy/herdr.observatory/State.js`, `omarchy/herdr.observatory/SnapshotStore.qml`, `tests/test_omarchy_state.cjs`, `tests/qml/anton/tst_store.qml` |
| B (popover structure) | `omarchy/herdr.observatory/{Panel,PopupContent,AntonSurface,AntonText,SectionHeader,ThreadCard,AllowanceCard,MetricDial,SheenTitle,ThreadSignal,BurnEffect}.qml`; new `AntonTheme.qml`, `AntonPreferences.qml`, `AntonController.qml`, `AntonToolTip.qml`, `AntonKeyedModel.qml`; `omarchy/herdr.observatory/{install.sh,uninstall.sh,README.md}`; `tests/qml/anton/**` except `tst_store.qml`; `tests/run-qml.sh`; `tests/test_native_distribution.mjs`; new `tests/run-qmllint.sh`; `.github/workflows/checks.yml` |

Why two lanes: the pure JS contract (D2 to D4) can be built and node-tested independently of the QML restructure (D5 to D11).

Why the lanes are phased (A1, A2, A3):

- The view-shape change is the only coupling between the lanes.
- It is made additive first (A2), consumers migrate (B7), and only then are the old fields removed (A3).
- This keeps every commit green for `git bisect`.

If the lanes share one worktree, each stages explicit paths only and never commits, amends or rebases the other lane's files. Only the coordinator rebases.

### D2. `State.js` pure helpers (lane A, phase A1, additive)

Each helper is exported through `module.exports` and reachable from QML as `State.<name>`. Unless noted, each has the exact semantics of the baseline code it replaces, and node tests cover them:

- `tokens(value)`: from `Panel.tokens`. Null or undefined gives `—`; values scale to K, M or B with one decimal and a trailing `.0` stripped.
- `percentReading(value)`: from `Panel.percentReading`, including `—`, `>99%` and `<1%`.
- `paceText(reading)`: from `Panel.paceText`. It takes any object with `remaining` and `timeRemaining` and returns `Pace unavailable` when either is null.
- `parseList(text)` and `parseObject(text)`: from Panel. Invalid JSON or the wrong type gives `[]` or `{}`.
- `stateColourName(state)`: `working` gives `yellow`, `blocked` gives `red`, `done` gives `green`, anything else gives `muted`.
- `accountKey(account)`: `provider + ":" + id`.
- `toggleListValue(list, value)`: a new array with `value` appended when absent, or removed at its index when present.
- `boundAcknowledgements(object, limit = 256)`: a copy with the earliest `Object.keys` entries removed until `limit` remain. This is Panel's `shift` order.
- `reconcileAcknowledgements(acknowledged, threads)` returns `{value, changed}`. It removes the acknowledgement for any listed thread that is not `done`, or whose `completionEpisode` differs. It does not touch threads that are absent (baseline `onOverviewChanged`).
- `acknowledgeNavigation(acknowledged, target, threads)`: after a successful launch whose target `state` was `done`, adds `target.key → target.episode` only if the current thread with that key is still `done` with the same episode. Otherwise it returns the input unchanged (baseline `onExited`).
- `assignAliases(accounts, pool, random)`: a Fisher-Yates shuffle of `pool` using the injected `random()` (Panel passes `Math.random`), assigning `accountKey(account) → shuffled[index % pool.length]`.
- `keyedEdits(before, after)`: an ordered list of `{op: "remove", index}`, `{op: "insert", index, key}` and `{op: "move", from, to}`.
  - Applying the list in order to a copy of `before` yields `after`.
  - A key present in both lists is never removed and re-inserted. It is only moved or left in place.
  - Both inputs are arrays of unique strings.
- `groupThreads(view, hidden, collapsed)`: gains `keys`, the thread keys of `indices` in the same order, alongside the existing fields.
- `providerGroups(allowances)`: gains `keys`, the account keys in order, alongside the existing fields.

### D3. Time separation and view shape (lane A)

**Phase A2 (additive).** `project(raw, nowMs)` keeps every baseline field and adds source times:

- `thread.usage.at`: the usage stamp in ms, or null when usage is empty.
- `thread.children.at` and `thread.completion.at`: stamp in ms, or null.
- `thread.timing.observedAt`: already present, in seconds. Also adds `startedAt` (the validated start in seconds, or null) and `finishedTotal` (`total_finished_duration_s` when `complete`, otherwise null).
- `allowance.resetAt` (seconds), `durationS` and `sampledAt` (seconds). All three are non-null only when the balance is current, that is when `remaining` is non-null.

Readings are pure functions of a structural value and `nowMs`. Each returns exactly the baseline shape, so existing consumers and hints are unchanged:

| Reading | Returns (baseline shape) |
| --- | --- |
| `usageReading(usage, nowMs)` | `threadUsage`: `contextPercent, inputTokens, outputTokens, uncachedTokens, cachePercent, compactions, age, stale` |
| `childrenReading(children, nowMs)` | `childObservations` or null |
| `completionReading(completion, nowMs)` | `childCompletion` or null |
| `turnReading(timing, nowMs)` | `turnTiming`: `active, elapsed, last, total, complete, stale, age, observedAt, outcome`, or null |
| `allowanceReading(account, nowMs)` | `allowanceView` (11 keys) |
| `readView(view, nowMs)` | the baseline view without `host.age` and `thread.age` |

Time behaviour of the readings:

- `stale` comes from the structural view.
- `age` labels, `elapsed` for an active non-stale turn (`floor(now - startedAt)`), the accumulated `total`, `timeRemaining`, `paceDifference` and the `reset` label all come from `nowMs`.
- Stale-active elapsed stays frozen at `floor(observedAt - startedAt)`.

Also in A2:

- **`diagnostics(view, nowMs)`**: returns the baseline IpcHandler JSON string (field order included). It uses `turnReading` for timing and works on the A2 and A3 shapes.
- **`SnapshotStore.now`** (additive): a `double` that is set to `Date.now()` when `visualUpdates` becomes true, on each tick while `visualUpdates` is true, and on each receipt while `visualUpdates` is true.

**Phase A3 (gated on B7 and B8).**

`project` stops emitting the time-derived fields:

- `host.age` and `thread.age`;
- `age` in `usage`, `children` and `completion`;
- `timing.elapsed`, `timing.total` and `timing.age`;
- allowance `timeRemaining`, `paceDifference`, `reset` and `age`.

It keeps the structural fields: `connected, working, partial, threads, hosts, allowances, discoveryLabel, note`, plus every other baseline field. It also adds these exports:

- **`viewSignature(view)`**: the replacement signature (`JSON.stringify`).
- **`nextDeadlineMs(raw, nowMs)`**: the earliest instant after `nowMs` at which a structural value of `project(raw, ·)` can change, or null if there is none. Candidates, where `ceil` gives the first integer ms at or after the instant:
  - a host `sampled_at` in the future;
  - host `sampled_at + maxAge`, where reporting ends because `age < maxAge` is strict;
  - telemetry `seq / 1000`;
  - each of the usage, child and completion stamps (at the stamp, and just after stamp + 120 s);
  - turn `observed_at_s − 1` and just after `observed_at_s + freshness`;
  - allowance `sampled_at − 1` and just after `sampled_at + 600`;
  - `resets_at` and `reset_expires_at`.

  The deadline may be early, which costs one no-op projection. It must never be late.
- **`storeStep(store, event, nowMs)`**: the store logic, shared by `SnapshotStore` and the measurement harness.
  - `store` is a plain object `{raw, lastReceipt, view, signature}`. `storeStep` adds `deadline` lazily, so the harness's object works unchanged.
  - Events: `{type: "receipt", raw}` (raw is null for a malformed or oversized line, and then `lastReceipt` is unchanged, as at baseline), `{type: "update"}` (restart, refresh or collector exit, where the caller sets `store.raw` first), and `{type: "tick"}`.
  - A tick drops `raw` if `nowMs - lastReceipt > receiptTimeoutMs(raw)`. Otherwise it updates only when `deadline !== null && nowMs >= deadline`, and otherwise it does nothing.
  - An update projects, applies `stableThreads` against the previous view, recomputes `store.deadline` from the new raw (even when the signature is unchanged), and replaces `view` and `signature` only when the signature differs.
  - It returns true when the view was replaced.

`SnapshotStore.qml` calls `State.storeStep` for every path, so the harness's `view_replacements` measures production logic.

A QML object cannot gain properties at run time, so `SnapshotStore` keeps an internal plain object `{raw, lastReceipt, view, signature, deadline}` as the `store` argument. After each step it mirrors `raw`, `lastReceipt` and `view` into its existing public properties of the same names, which `tst_store`, `Panel` and `PopupContent` read. `projectedSignature` is removed. Callers only read those properties; every write goes through `storeStep`. On a receipt, `accept(line)` passes `raw: null` for an oversized or malformed line.

 The 1 s timer runs whether the popover is open or closed. While closed it therefore both drops a silent collector (as today) and applies freshness deadlines. Thresholds are unchanged; closed-state staleness can only become more prompt (within one tick instead of at the next receipt). The `now` update from A2 is unchanged.

Alternatives considered:

- Keep ticking projection but diff labels. This still rebuilds and stringifies every second, so it was rejected.
- Put host `sampled_at` into the view so host ages stay displayable. This forces a replacement on every receipt, and nothing displays host or thread age, so both fields were removed instead. The oracle comparison excludes them.

### D4. Oracles (coordinator, frozen at baseline)

- **`tests/fixtures/popover-time-oracle.json`.** It holds 17 synthetic snapshots with 112 projections from the `53f2407` `State.js`, each at instants on both sides of every threshold (generator `tests/capture_popover_oracles.cjs`).
  - Lane A's node test asserts that `readView(project(raw, t), t)` deep-equals each oracle view with `host.age` and `thread.age` removed.
  - It also asserts, for every case and for instants `t` in `[now, nextDeadlineMs)`, that `viewSignature(project(raw, t)) === viewSignature(project(raw, now))`.
- **`tests/fixtures/popover-diagnostics-oracle.json`.** It holds 13 cases, and `State.diagnostics(project(raw, t), t)` must equal each string exactly.
- **`tests/fixtures/popover-privacy-v2.ini` and `popover-privacy-v1.ini`.** These were written by the baseline `Core.Settings` block and the baseline write calls, with synthetic values. v1 has only `personalHidden`, `workHidden` and aliases.

### D5. `AntonTheme.qml` (lane B)

A `QtObject`:

- **Colour properties:** `ink` (`Color.popups.text`), `muted` (`alpha(ink, 0.62)`) and `line` (`alpha(ink, 0.12)`); `blue`, `cyan`, `green` and `yellow` (the palette value, or `Color.accent`); `red` (the palette value, or `Color.urgent`).
- **`face`:** a string input, which Panel binds to `bar ? bar.fontFamily : Style.font.family`.
- **`paletteUrl`:** defaults to `Color.currentThemePath + "/colors.toml"`.
- **Palette file:** an internal `FileView` with `watchChanges`, reload on change, and the baseline regular-expression parser.
- **Functions:** `reload()`, `alpha(colour, opacity)` (which stays here because it needs `Qt.rgba`), and `stateColour(state)`, which maps through `State.stateColourName`.

All colour properties are plain `property color` with default bindings, so tests can assign fixed values. A `Connections` on `Color` reloads on accent or background changes.

### D6. `AntonPreferences.qml` (lane B)

- **Settings.** It wraps a `Core.Settings` whose `location` comes from `required property string location`. The value must be supplied when the object is created, never assigned afterwards. Otherwise the inner Settings could load, and migrate, at Qt's default location, which is the real user configuration. Tests create preferences with `createObject(parent, {location: ...})` or a declaration that sets it inline. Panel passes the baseline expression `"file://" + (XDG_STATE_HOME || HOME + "/.local/state") + "/herdr.observatory/privacy.ini"`.
- **Stored properties.** The same stored property names, types and defaults: strings containing JSON, `namesHidden` as bool, and `personalAlias` and `workAlias`. Changing their type would change how QSettings encodes the file.
- **Migration.** `privacyVersion < 2` behaves identically.
- **Typed read-only views,** each parsed once per stored string change:
  - `hiddenStates`, `collapsedHosts`, `collapsedProviders` and `collapsedSections` (arrays);
  - `threadsCollapsed` and `allowancesCollapsed` (bool);
  - `namesHidden` (bool);
  - `accountAliases` and `acknowledgements` (objects);
  - `legacyAliases`: `{"codex:Personal": personalAlias, "codex:Work": workAlias}`.
- **Mutators,** each with the baseline `setValue` and `sync` sequence:
  - `toggle(listName, value)`;
  - `setAcknowledgements(object)`, which applies `State.boundAcknowledgements`;
  - `setIdentity(namesHidden, aliases)`, which writes aliases only when concealing, then `namesHidden`, `personalAlias` and `workAlias`, then syncs.

### D7. `AntonController.qml` (lane B)

A `QtObject`. Inputs:

- `view` (the structural view) and `preferences` (AntonPreferences). It has no `now` input: no controller action depends on the display instant, and the cards take `now` directly;
- `opened` and `motionEnabled` (bool);
- `runtimePath` (string, from `Qt.resolvedUrl("anton-runtime")` with `file://` stripped).

It owns:

- `focusedKey`, `navigationError` and `visualEpoch`;
- `threadGroups`, `threadKeys`, `providers` and `barState`, which ignores acknowledged done episodes;
- the navigation `Process` and its `StdioCollector`;
- the signals `newThreads(var keys)`, `observedChange(var changes)`, `refreshRequested()` and `closeRequested()`.

Functions:

- `openThread(key)`;
- `activate()`, which opens `State.activationKey(threadKeys, focusedKey)`;
- `moveFocus(delta)`;
- `toggleList(name, value)`: `visualEpoch++`, then the preferences toggle, then clear focus;
- `toggleIdentity()`: shuffle with `State.assignAliases` only when concealing;
- `refresh()`, which emits `refreshRequested`;
- `accountAlias(account)`.

View changes follow baseline `onOverviewChanged` exactly:

1. Compute transitions and arrivals against the previous view.
2. Reconcile and persist acknowledgements.
3. When open with motion enabled, emit `observedChange` and then `newThreads` through `Qt.callLater`, guarded by `visualEpoch`.

Focus reconciliation on `threadKeys` changes is unchanged. `Panel.qml` keeps the `IpcHandler`: `diagnostics` returns `State.diagnostics(store.view, Date.now())`, and `refresh` calls the store refresh and `theme.reload()`. Panel also keeps `BarIconButton`, which uses `barState` and the theme colours, `KeyboardPanel`, `PanelKeyCatcher`, the `.accounts.json` `FileView` and the wiring. Behaviour on open is unchanged: `visualEpoch++`, clear focus, reload the theme, `store.restart()`, then focus the key catcher.

### D8. Component properties (lane B)

No component declares `var ui`.

| Component | Inputs |
| --- | --- |
| `AntonText` | `theme: AntonTheme` |
| `AntonSurface` | `theme`, `tooltip: AntonToolTip`, `animate: bool` (opened and motion), plus its existing `hint`, `keyed`, `restingOpacity`, `selected`, `tint` and `tooltipSuppressed`; exposes `hovered`, `hoverX` and `hoverY` |
| `SectionHeader` | `theme`, `tooltip`, `animate`, `title`, `collapsed: bool`; signal `toggled()` |
| `ThreadCard` | `entry` (a structural thread), `now: double`, `focused: bool`, `last: bool`, `theme`, `tooltip`, `controller: AntonController` (for its signals, `opened`, `motionEnabled`, `visualEpoch` and `openThread`), `viewport: Flickable`, `contentItem: Item` |
| `AllowanceCard` | `entry` (a structural account), `now`, `theme`, `tooltip`, `opened`, `motionEnabled`, `namesHidden`, `aliasName`, `email`; signal `identityToggled()` |
| `MetricDial`, `ThreadSignal`, `SheenTitle`, `BurnEffect` | `theme` plus `animate` (or `opened` and `motionEnabled` where they differ today) |
| `PopupContent` | `theme`, `preferences`, `controller`, `view`, `now`, `accountEmails` |

Readings are computed inside the cards from `entry` and `now`: `State.usageReading`, `State.turnReading`, `State.completionReading` and `State.allowanceReading`.

### D9. Keyed delegates (lane B)

`AntonKeyedModel.qml` is a `ListModel` with a `key` role and a function `sync(keys)`, which applies `State.keyedEdits` using `remove`, `insert` and `move`. Four keyed levels, each a Repeater over one of these models:

- machine groups, keyed by host id;
- the threads of each group, from `group.keys`;
- provider groups, keyed by provider id;
- the accounts of each provider, from `provider.keys`.

Delegates declare `required property string key` and resolve their data through a per-group table that PopupContent rebuilds in the same sync: `entries` maps each thread key (with any occurrence suffix) to its view thread, and `accounts` maps each account key to its account.

Sync runs synchronously in the handler that reacts to a view or preference change, before the controller's `Qt.callLater` emission. A newly created delegate therefore exists when `newThreads` fires, and an existing delegate keeps its running animation.

Entrance is still triggered only by `newThreads` (`State.arrivals`, unchanged), so a delegate created for hydration, reconnect, filtering, expansion or sorting starts at full opacity.

The delegate-identity QML test is lane B's first keyed task, because Repeater's retention of items across `ListModel.move` and `remove` must be demonstrated rather than assumed.

A plain JS-array model was rejected: reassigning it recreates every delegate.

### D10. `AntonToolTip.qml` (lane B)

There is one instance, created by `PopupContent` and parented to it as the host.

- **API:** `host: Item` (bounds, plus `moving`), `show(source)`, `hide(source)`, and `source` (read-only).
- **Showing a hint.** Each `AntonSurface` computes `wantsTooltip = hovered && hint.length > 0 && !tooltipSuppressed` and calls `show` or `hide` when it changes. `show` from a different source closes the tooltip first, so the delay restarts.
- **Visibility:** `source !== null && !host.moving` together with `source.wantsTooltip`.
- **Geometry:** the cursor is `source.mapToItem(host, source.hoverX, source.hoverY)`, with the baseline `x`, `y` and `width` formulas.
- **Timing and styling:** `delay` 450, `timeout` 6000, `margins` `Style.space(8)` and `padding` `Style.space(7)`. The background is a `Rectangle` with radius 0 and `Color.tooltip` border and background colours. The content is `AntonText` with `Color.tooltip.text`, wrapped and without elision.

Omarchy `PanelToolTip` was rejected because its 400 ms delay, zero padding, border surface and corner radius would change behaviour and pixels.

### D11. Layout (lane B)

`PopupContent` uses a `ColumnLayout` with `spacing: 0`, and every child sets `Layout.fillWidth: true`. Named properties replace the literals:

- `chromeHeight`: the heading, threads header and allowances header heights, which sum to 95 logical pixels today;
- `threadReserve` 85, `threadShare` 0.55, `allowanceCap` 180 and `threadCap` 300.

The two Flickables take `Layout.preferredHeight` and `Layout.maximumHeight` from the unchanged formulas:

- `reservedThreadHeight`, `allowanceHeight` and the thread viewport height;
- `implicitHeight = chromeHeight + min(threads, threadCap) + min(allowances, allowanceCap) + notice`.

If `ColumnLayout` rounds fractional heights differently from `Column`, the screenshots will show it. The fallback is to pin `Layout.minimumHeight` and `Layout.maximumHeight` to the same computed values.

**Outcome.** `ColumnLayout` snaps each child's position to whole pixels: the notice moved from y 129.35 to 130 and every row below it moved 1 px. Pinning minimum and maximum heights did not change that. `PopupContent` therefore stays a `Column` whose heights come from the named properties (`chromeHeight`, `headingHeight`, `sectionHeight`, `threadReserve`, `threadShare`, `allowanceCap`, `threadCap`, `threadViewportHeight`, `noticeHeight`) with the formulas unchanged. Pixel identity takes precedence over the layout type (AGENTS.md presentation rules).

### D12. Tests, qmllint and CI (lane B unless noted)

**Screenshots.** `tst_popup.qml` renders the real `PopupContent` with the real theme, controller and preferences:

- the theme colours are assigned the former fixture values for dark and light, with `face` `monospace`;
- preferences use a temporary location;
- the view is `State.project(raw, now)` with a fixed `now`.

The seven `capture()` calls, their names and their scenes are unchanged.

**Reference correction.** `FixtureUi.qml` formatted tokens as `(n / 1000).toFixed(1) + 'K'`, which keeps a trailing `.0` (`34.0K`). Production `Panel.tokens`, now `State.tokens`, strips it (`34K`). Five archived hashes (dark, light, many-short, discovery-unavailable, short-with-notice) therefore certified the fixture's text, not the production popover. Rendering the unchanged `75d6548` tree with only the fixture formatter replaced by the production one yields a new set of five hashes; the restructured tree yields the same set, and the restructured tree with the fixture formatter patched in yields the archived set. The corrected set in evidence.md is the screenshot gate from task 3.4 onward. `connections-missing` and `discovery-setup` render no token counts and keep their archived hashes.

**New QML tests** cover:

- preferences loading both captured ini files byte-for-byte copied into a temporary location, with values checked and a toggle round trip compared against the expected file content;
- controller acknowledgement persistence and invalidation;
- focus movement and reconciliation;
- navigation arguments and route errors, using the `Process` stub (command equals `[runtimePath, "--open-thread", host, id, binding]`, invalid route error text, non-zero exit text, and no launch while one is running);
- theme palette parsing and fallback through a new `Quickshell.Io` `FileView` stub;
- the shared tooltip (delay, timeout, source switching, suppression while moving);
- delegate identity.

`run-qml.sh` may set `QML_XHR_ALLOW_FILE_READ` and `QML_XHR_ALLOW_FILE_WRITE` for file copies. `FixtureUi.qml` is deleted. `tst_metrics.qml` keeps its scene (two machines, three threads, two accounts) and its counting rule.

**qmllint.** `tests/run-qmllint.sh` runs Qt 6 qmllint with `-I tests/qml/anton` over every plugin QML file, fails on any warning in files other than `Panel.qml`, and prints the `Panel.qml` warnings. `Panel.qml` may keep only warnings caused by the missing `qs.Ui` stub: the `qs.Ui` import and its types, the unresolved base, `unqualified` on `anchors.fill: parent`, and (Qt 6.8 only) `missing-property` tied to a `qs.Ui` object by location as well as message. `Could not find property "<name>"` is accepted only on a line whose enclosing object is the `Panel` root, `BarIconButton`, `KeyboardPanel` or `PanelKeyCatcher`, and only for a property Panel writes on that type. `Cannot assign to non-existent default property` is accepted only on a line that opens a child object directly inside one of those types. A brace-tracking awk pass over `Panel.qml` gives each line's enclosing object. On Qt 6.11 or later every `missing-property` fails, since the clean tree reports none. Qt 6.9 and 6.10 report every read of a `qs.Ui` member as `missing-property` (on type `Panel`, `KeyboardPanel`, `BarIconButton` or `""`), so the script refuses them. Any other `missing-property` fails, because lint is `Panel.qml`'s only automated check. Examples are `Member "relaod" not found on type "AntonTheme"`, a `qs.Ui` property name written on `AntonController`, and a stray child inside it. CI runs the script in the `qml` job. Evidence also records a local run with a scratch `qs` symlink to the Omarchy shell, which covers `Panel.qml`.

The stubs change: `qs.Commons` `Color` gains `currentThemePath`, `foreground`, `urgent` and `popups`, and `Quickshell.Io` gains `FileView` and `StdioCollector`. The stubs only grow, but the harness qmllint delta between baseline and after is not strictly like-for-like; evidence says so.

Qmllint may resolve a system module instead of a stub. On the planning workstation, the baseline `SnapshotStore` warning names `QProcess::ExitStatus`, a type that only the installed Quickshell qmltypes define, so `-I tests/qml/anton` locally still resolved the installed Quickshell. CI has no Quickshell and runs Qt 6.8.3; local runs used 6.11.2. Lane B therefore:

- confirms which module qmllint resolves for each import (for example with `--verbose` or by temporarily hiding the system module path);
- keeps the non-Panel files importing only modules the repository stubs provide, plus Qt modules;
- treats the CI qmllint run as the authoritative verification, fixing or documenting any Qt 6.8-only warning.

### D13. Measurement (coordinator)

The coordinator reruns `tests/measure_anton_popover.mjs` (existing definitions unchanged; architecture metrics added in `a7ff5c0`) on the final head, with the same command and `--repeat 3`.

Expected results:

- `view_replacements` drops from 60 to the number of receipts or boundaries that change structure. With the harness fixture this is 1: the turn in `jsSnapshot` is observed 1 s before `NOW` with 12 s freshness, so it goes stale at 12 s, which coincides with a receipt. Receipts refresh the host sample time, so the host never goes stale.
- `clock_only_view_changes_60s` drops from 25 to 2: the turn goes stale at step 12 and the host stops reporting at step 25, because a single unchanged snapshot does not refresh `sampled_at`.
- `required_property_var_ui` and `ui_member_references` fall to 0.
- `preference_parse_calls` falls to the number of parse-once sites. The metric's regular expression also matches `State.parseList(`, so the target is one site per stored JSON string plus the `.accounts.json` parse, all outside bindings that re-evaluate per delegate.
- `allowance_contract` reads `timeRemaining` and `reset` directly from `project(...).allowances`. After A3 those fields are no longer in the view, so its three cases report null for them. This is expected and not a regression. The additive `allowance_readings` metric (commit `618d383`) applies `State.allowanceReading` when it exists, and the projected fields otherwise, to the same rows at the same instant. It carries the comparable values: at baseline it equals `allowance_contract`, and after A3 it must be unchanged.
- `projection.thread_fields` and `projection.allowance_fields` key and leaf counts change with the A3 shape.
- `tooltips_per_thread_card` falls to 0, and `rendered_tooltip_instances` (QML suite) to 1.
- `hardcoded_omarchy_state_paths` falls to 0.
- Runtime figures stay within noise, with an identical binary hash.

## Risks / Trade-offs

- **[Risk] Repeater may not keep items across `ListModel.move`.** Mitigation: D9 makes the identity test first. If moves recreate items, fall back to remove-free ordering in which `stableThreads` already keeps surviving rows in place, so moves are rare. Record the result.
- **[Risk] `ColumnLayout` geometry could differ by a subpixel from `Column`, breaking hashes.** Mitigation: the D11 fallback. Screenshots gate every lane B commit that touches layout. This happened, and the fallback did not help, so `PopupContent` keeps a `Column` with named height rules (D11 Outcome).
- **[Risk] Qt 6.8 type resolution.** Two plugin files that name each other's types (`AntonSurface` and `AntonToolTip`) load on Qt 6.11 but stall `qmltestrunner` on Qt 6.8.3 after a `qt.qml.typeresolution.cycle` warning. `AntonSurface.tooltip` is therefore typed as the base `ToolTip`.
- **[Risk] Early deadlines or missed boundaries.** A missed boundary would leave a stale flag late. Mitigation: brute-force node tests around every oracle threshold (D4), and the rule that a deadline may be early but never late.
- **[Risk] The lanes share a worktree.** Mitigation: explicit-path staging, phase gates (A3 after B7 and B8), and coordinator-only rebases.
- **[Trade-off] Host and thread age leave the view.** No surface displays them. Keeping them would force a replacement on every receipt.
- **[Trade-off] Closed-state staleness becomes more prompt,** at the next tick after a boundary instead of the next receipt, with identical thresholds.
- **[Risk] QSettings encoding drift.** Mitigation: stored property types are unchanged, and the captured-ini round-trip test runs.

## Migration Plan

- No data migration. Preferences, `.accounts.json`, the config, peers and the runtime are untouched.
- The parent updates the installed plugin payload (the new QML files included) after review. The installed `uninstall.sh` lists the new files, so a later uninstall still passes its allowlist.
- Rollback is reinstalling the previous payload. The preference file format is unchanged in both directions.

## Open Questions

None.

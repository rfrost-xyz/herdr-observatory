# Proposal

## Why

The Anton popover works, but its QML structure makes it expensive to change safely:

- `Panel.qml` holds the palette, preferences and their migration, privacy aliases, completion acknowledgements, navigation, focus, IPC and formatting.
- Every component takes `required property var ui` and reaches into all of it. qmllint therefore cannot check the popover (104 `unqualified` warnings at baseline).
- `tests/qml/anton/FixtureUi.qml` hand-copies that surface, so the real acknowledgement, focus, navigation and settings code is never tested.
- Time-dependent age labels are baked into `State.project`. While the popover is open, `SnapshotStore` rebuilds, stringifies and replaces the whole view every second: 60 replacements per simulated open minute.
- Delegates are indexed by position (`overview.threads[threadIndex]`), so a running flash stays with a row position instead of its thread.
- Each surface creates its own `ToolTip`, which makes 29 in the standard fixture.
- `PopupContent` sizes itself by summing hard-coded heights.

This is change 3 of 3 in the Anton popover programme. Change 1 (`fix-anton-popover-correctness`, `cc5f982`) fixed focus, refresh and theming. Change 2 (`generalise-anton-allowance-windows`, `53f2407`) made allowance rows provider-neutral. This change restructures the popover without any visible change.

## What Changes

- **Split `Panel.qml` into focused parts:**
  - a theme object: the palette from the shell's current theme directory, plus the derived ink, muted, line and state colours;
  - a preferences object: the same `Core.Settings` file and keys, each parsed once into typed read-only values, with the same migration and persistence;
  - a controller: navigation, acknowledgements, focus, refresh requests and the identity toggle.

  `Panel.qml` keeps the bar button, `KeyboardPanel`, the IPC handler and the wiring.
- **Typed properties instead of `ui`.** Components receive typed, narrow properties (theme, preferences or controller, and the data and callbacks each one needs). No `required property var ui` remains.
- **Pure helpers in `State.js`, with node tests.** Formatters and pure helpers move out of `Panel.qml`: tokens, percent reading, pace text, list and object parsing, state colour names, the acknowledgement rules and the IPC diagnostics builder.
- **Time separation.**
  - `State.project` produces a structural view with source times. It runs when a snapshot arrives, when inputs change, and when a measurement-freshness boundary passes, but not on every tick.
  - A separate `now` value ticks while the popover is open. Only relative-time labels and time-derived readings depend on it: ages, the active-turn stopwatch, the reset countdown, expected allowance and pace.
  - Every staleness rule and threshold is preserved, including while the popover is closed.
- **Keyed delegates.** Thread, machine, provider and account delegates are identified by stable keys. A delegate and its running animation stay with its thread across reordering and removal of others. Entrance still does not replay on hydration, reconnect, filtering, collapse or sorting.
- **One shared tooltip** for the popover. It keeps the current delay, timeout, pointer placement, clamping, suppression while scrolling or while a nested metric is hovered, and the theme tokens.
- **Layout.** `PopupContent` names its height rules (chrome, thread reserve and share, thread and allowance caps) instead of summing literals, and keeps today's sizes and the thread/allowance space split. A `ColumnLayout` was tried and rejected because it snaps child positions to whole pixels and moves rows by 1 px (design D11).
- **Tests.**
  - Tests exercise the real theme, preferences and controller. `FixtureUi.qml` is removed.
  - Coverage includes acknowledgement persistence, focus, navigation arguments and route errors, a settings round trip against captured `privacy.ini` files, IPC diagnostics against a captured oracle, and delegate identity.
  - qmllint must be clean for the plugin files under the repository's stub imports.
  - An optional harness (`tests/run-shell-harness.sh`) loads the popover against the installed Omarchy shell modules offscreen, through hot reloads and open/close cycles, and fails if a thread is opened without input. It is a local-only gate that CI does not run, and it skips where the shell is not installed.
- **Navigation only from an open popover.** The controller ignores thread activation unless the popover is open, so a tap or accessibility press during the closing fade cannot navigate (added after the live install check, evidence.md "Live install findings").
- **Visuals stay pixel-identical.** The seven screenshots must match the baseline rendering byte for byte. Five of the archived reference hashes were rendered with `FixtureUi.qml`'s own token formatter (`34.0K`) rather than the production one (`34K`), so the reference is corrected to the baseline tree rendered with the production formatter (design D12, evidence.md "Visual reference correction").

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `omarchy-companion`: adds six requirements.
  - Clock-separated view updates, with measurement freshness preserved while the popover is open and while it is closed.
  - Stable delegate identity for effects.
  - A single popover tooltip with unchanged behaviour.
  - Preserved local preferences and shell commands (the existing `privacy.ini`, and IPC functions and diagnostics output).
  - A theme palette taken from the shell's current theme location.
  - Thread navigation only from an explicit action while the popover is open.

  Existing requirements are unchanged.

## Impact

- **Presentation** (`omarchy/herdr.observatory`):
  - rewritten: `Panel.qml`, `PopupContent.qml`, `SnapshotStore.qml`, `State.js`, and every component (`AntonSurface`, `AntonText`, `SectionHeader`, `ThreadCard`, `AllowanceCard`, `MetricDial`, `SheenTitle`, `ThreadSignal`, `BurnEffect`);
  - new: `AntonTheme.qml`, `AntonPreferences.qml`, `AntonController.qml`, `AntonToolTip.qml` and `AntonKeyedModel.qml`;
  - `install.sh` and `uninstall.sh` list the new files.
- **Tests:**
  - `tests/test_omarchy_state.cjs` gains the formatter, time, keyed-edit and oracle tests;
  - `tests/qml/anton/**` moves to the real components, drops `FixtureUi.qml` and gains a `FileView` stub and more `qs.Commons` stub members;
  - `tests/test_native_distribution.mjs` follows navigation into the controller;
  - a qmllint script runs in CI;
  - `tests/run-shell-harness.sh` with `tests/shell/` runs the popover against the installed Omarchy shell modules when they are present (local only; CI does not run it).
- **Measurement:** `tests/measure_anton_popover.mjs` gains additive architecture metrics only (commit `a7ff5c0`). Existing definitions are unchanged. The replacement metric observes the production store logic through `State.storeStep`.
- **Compatibility:**
  - No runtime (Rust) change. The release binary must be byte-identical, so installed SSH peers and the snapshot contract are unaffected.
  - Existing `privacy.ini` files, IPC commands and diagnostics output are unchanged.
- **Out of scope:** runtime changes, the allowance wire contract, new features and new visible text.

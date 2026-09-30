# Spec Delta

## ADDED Requirements

### Requirement: Clock-separated view updates
The popover SHALL rebuild its projected view only when a snapshot arrives, when the collector is restarted or drops, or when a measurement-freshness boundary of the current snapshot passes. Except at such a boundary, the passage of time SHALL NOT rebuild or replace the view.

While the popover is open, relative-time labels and time-derived readings SHALL follow a separate clock that advances at least once per second. These are sample ages, the active-turn stopwatch, the reset countdown, and expected allowance and pace. Every existing measurement and transport freshness rule SHALL keep its threshold and its effect:

- a host sample stays current for its sampling interval plus 20 seconds;
- usage and subagent observations become last-known after 120 seconds;
- turn timing freezes after its stated freshness and accepts up to one second of source clock skew;
- an allowance observation is current for 600 seconds and accepts one second of skew;
- a passed reset invalidates the balance, and an expired reset pass invalidates the pass count;
- the collector is treated as disconnected after the receipt timeout derived from its heartbeat.

These rules SHALL apply whether the popover is open or closed, so the bar aggregate and its tooltip go stale no later than they do today. For the same snapshot and instant, the projected view combined with its time-derived readings SHALL equal the pre-change projection, except for the unused host and thread age labels. The displayed values SHALL lag the clock by no more than one tick, as before.

#### Scenario: Open popover with an unchanged snapshot
- **WHEN** the popover stays open for a minute while the collector re-sends an unchanged snapshot at each heartbeat
- **THEN** the view is replaced only when a snapshot's content or a freshness boundary changes it, not once per second, while displayed ages and the working-turn stopwatch still advance every second.

#### Scenario: Host stops reporting while the popover is closed
- **WHEN** a host's last sample passes its freshness limit while the popover is closed and the collector keeps sending snapshots
- **THEN** that host's threads leave the bar aggregate within one second of the limit, exactly as if the popover were open.

#### Scenario: Threshold instants
- **WHEN** the current snapshot is projected just before and just after the host, 120-second, turn freshness, 600-second, reset and pass-expiry thresholds, including sources stamped slightly in the future
- **THEN** every reporting flag, stale flag, balance, pass count, stopwatch value and label matches the pre-change projection at the same instants.

#### Scenario: Silent collector
- **WHEN** no snapshot arrives within the receipt timeout, whether the popover is open or closed
- **THEN** the snapshot is dropped and the popover and bar show sources as unavailable, as before.

### Requirement: Stable delegate identity
Machine groups, thread rows, provider groups and allowance rows SHALL each be bound to a stable key: machine id, host and thread identity, provider id, and provider and account id. A row SHALL keep its visual element, and any running highlight or entrance, while other rows appear, disappear or change order. A row's element SHALL be replaced only when its own key leaves the displayed set. Creating an element for a row that reappears through hydration, reconnect, filtering, expansion or sorting SHALL NOT play the entrance transition.

#### Scenario: Earlier thread disappears during a highlight
- **WHEN** a thread's state-change highlight is running and an earlier thread disappears from the next snapshot
- **THEN** the same row element continues the highlight for the same thread, and no other row starts or inherits it.

#### Scenario: Row reappears after filtering
- **WHEN** a filtered or collapsed thread becomes visible again
- **THEN** it appears at full opacity without the entrance transition.

### Requirement: Single popover tooltip
The popover SHALL use one shared tooltip for all hover hints. Hints SHALL keep today's behaviour:

- a 450 ms delay and a 6 s timeout;
- pointer-following placement 12 logical pixels right of and 18 below the pointer, flipped above it when there is no room;
- clamping to the popover bounds and a width of at most 280 logical pixels;
- suppression while a viewport scrolls, and suppression of a row's hint while a nested metric with its own hint is hovered;
- the shell's tooltip text, background and border colours with square corners.

Moving from one hinted element to another SHALL restart the delay.

#### Scenario: Hover a metric inside a thread row
- **WHEN** the pointer rests on a thread's turn stopwatch
- **THEN** after the delay the single tooltip shows the stopwatch hint rather than the row hint, and it follows the pointer within the popover bounds.

#### Scenario: Scroll while hovering
- **WHEN** a viewport scrolls under a resting pointer
- **THEN** no hint is shown until scrolling stops.

### Requirement: Preserved local preferences and shell commands
The popover SHALL read and write its local preferences in the same file, with the same keys, value encodings and defaults as before:

- hidden states, collapsed machines, collapsed providers and collapsed sections;
- concealment, saved aliases and legacy aliases;
- acknowledged completions, bounded to 256 entries.

It SHALL apply the same one-time migration from the earlier per-account concealment flags and persist every change immediately. The shell SHALL keep the `herdr.observatory` IPC target, with the functions `open`, `close`, `toggle`, `refresh`, `status` and `diagnostics`. `refresh` SHALL still refresh the collector and reload the palette, and `diagnostics` SHALL return the same fields and values for the same snapshot.

#### Scenario: Existing preferences after the update
- **WHEN** the updated popover starts with a preferences file written by the previous version
- **THEN** filters, collapsed groups, concealment, aliases and acknowledged completions are exactly as the operator left them, and later changes are written back in the same format.

#### Scenario: Preferences from before concealment was unified
- **WHEN** the preferences file has only the earlier per-account concealment flags
- **THEN** names start concealed if either flag was set, and the file records the migrated version once.

#### Scenario: Diagnostics command
- **WHEN** the operator runs the diagnostics IPC command
- **THEN** it returns the same JSON fields and values as the previous version for the same snapshot.

### Requirement: Shell theme palette source
The popover SHALL read its status palette from the colour file in the theme location that the shell reports as current, and SHALL NOT hard-code the theme directory. It SHALL reload the palette when the popover opens, when the shell accent or background changes, when the file changes and on the refresh command. Missing palette colours SHALL fall back to the shell accent, or to the urgent colour for red, as before.

#### Scenario: Theme switch
- **WHEN** the operator switches the Omarchy theme while the popover is open
- **THEN** status colours follow the new theme's palette without a restart, as before.

### Requirement: Navigation only from an open popover
The popover SHALL start thread navigation only in response to an explicit operator action while it is open: a key press in the popover, a tap on a thread row or an accessibility press action. Loading the plugin, hot reloads, opening and closing, focus changes and snapshot updates SHALL NOT start navigation. An action that arrives after the popover has closed, including during its closing fade, SHALL NOT start navigation or report an error.

#### Scenario: Tap during the closing fade
- **WHEN** the operator taps a thread row after the popover has been closed but while the row is still fading out
- **THEN** no navigation starts and no error is shown.

#### Scenario: Reload and reopen without input
- **WHEN** the plugin is reloaded several times, including while open, and the popover is then opened, closed and reopened without operator input
- **THEN** it stays open with its thread rows visible, and no navigation starts.

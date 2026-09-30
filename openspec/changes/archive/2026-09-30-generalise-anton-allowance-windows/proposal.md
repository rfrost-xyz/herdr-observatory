# Proposal

## Why

The allowance rows that the runtime sends to the popover follow Codex's shape: `weekly_remaining`, `weekly_resets_at`, a fixed `window_seconds` of 604800 and an `available` flag. `State.js` defaults a missing window to 604800, defaults a missing provider to `codex` and names Codex in a label fallback. `Panel.qml` also branches on `provider === "codex"`. The rejected Notion attempt (`d614094`, reverted in `e7dc154`) showed the cost of this shape. It had to add `provider === 'notion'` branches, `monthly_*` fields and a synthetic "Not connected" row built in the view, which broke the rule that missing data stays unknown.

This is change 2 of 3 in the Anton popover programme. Change 1 (`fix-anton-popover-correctness`, merged in `cc5f982`) fixed focus, refresh and theming. Change 3 handles the popover architecture. This change makes the allowance contract provider-neutral, so a later provider needs a collector and nothing in the presentation.

## What Changes

- The runtime-to-popover snapshot allowance row becomes provider-neutral. It carries:
  - identity: `provider`, `provider_label`, `account_id`, `label`;
  - a source status of `available`, `unavailable` or `auth_needed`, with optional bounded `status_text` supplied by the source;
  - `plan`, `sampled_at`, `reset_count` and `reset_expires_at`;
  - a bounded `windows` list of `{kind, label, used_percent, resets_at, duration_s, pacing}`, with exactly one pacing window.
- **BREAKING** (internal wire between the plugin's own runtime and QML, which are always installed together): the snapshot row no longer carries these fields. `weekly_remaining`, `weekly_resets_at` and `window_seconds` are replaced by the pacing window. `available` is replaced by `status`. `lifetime_tokens`, `peak_daily_tokens` and `daily_usage` are removed, because nothing has consumed them since change 1.
- The local runtime converts two legacy inputs into the new row: today's `--allowances-probe` peer rows and today's private allowance cache. Installed SSH peers are not updated. The new runtime's own peer probe output and cache format stay byte-compatible with today's, so older local runtimes can still read them.
- `State.js` projects balance, time remaining, pace and reset label generically from the pacing window. It has no provider branches and no Codex default durations. Unknown window kinds render generically. `auth_needed` and `unavailable` show the source's status text or the existing "Allowance unavailable" wording. No row is invented for an absent provider.
- `Panel.qml` replaces its Codex-specific legacy alias branch with data. Saved aliases stay keyed by `provider:id`, and the legacy Personal and Work preferences move to a data table keyed by `provider:label`, so existing aliases keep working.
- Codex presentation stays pixel-identical: the seven screenshot hashes recorded in change 1 must still match. A 30-day window uses the existing days-and-hours reset label.
- The design documents the mapping to Omarchy's agents `limits` record. No adapter is implemented.
- The measurement harness gains additive allowance metrics: wire bytes and keys per row, projection of neutral rows, and provider-coupling counts in the presentation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `account-allowances`:
  - Allowance observation is exported as provider-neutral rows with source status and bounded windows.
  - Legacy peer rows and cache entries are converted locally.
  - Token-activity summaries leave the popover snapshot.
- `omarchy-companion`:
  - "Truthful current state" projects allowance from the designated pacing window, not from Codex weekly fields.
  - "Provider and account collections" requires generic projection, source-supplied status text and no invented rows.

## Impact

- **Runtime** (`omarchy/anton-runtime`):
  - `src/allowances.rs`: conversion into public rows.
  - `src/model.rs`: `AllowanceRow` and a new `AllowanceWindow`.
  - `src/main.rs`: the snapshot row type.
  - Unit and process tests.
  - `--allowances-probe` output and `allowances.json` are unchanged.
- **Presentation** (`omarchy/herdr.observatory`):
  - `State.js`: allowance projection.
  - `AllowanceCard.qml`: unavailable hint text.
  - `Panel.qml`: the `accountAlias` legacy lookup.
  - JS and QML fixtures move to the new row shape.
- **Measurement**: `tests/measure_anton_popover.mjs` has additive metrics only. Existing metric definitions are unchanged.
- **Compatibility**:
  - Installed peers and existing caches keep working unchanged.
  - A popover QML older than the runtime cannot read the new rows. The plugin installs QML and runtime together, so this cannot happen in a supported installation.
- **Out of scope**:
  - Notion or any other new collector.
  - Consuming or emitting Omarchy agents records.
  - The Panel split and other change 3 work.

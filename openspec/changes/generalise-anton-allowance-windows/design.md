# Design

## Context

See proposal.md for motivation. Current state at `cc5f982`:

- **Runtime.**
  - `allowances::summarise` turns a Codex `account/rateLimits/read` reply into a source row keyed by account hash. It picks the weekly window as the single `windowDurationMins == 10080` window.
  - `summarise_usage` adds the token-activity fields.
  - `sanitise` bounds and expires a source row. `read_cache` and `remote` both go through it.
  - `snapshot()` recomputes public rows every 2 s from the cache plus the peer rows. It keeps the newest row per mapped account and emits one row per configured account, sorted by private account key (the configuration map is not order-preserving). It hardcodes `provider: "codex"`, `provider_label: "Codex"` and `window_seconds: 604800`.
  - `main.rs:737` deserialises those rows into `model::AllowanceRow` with `if let Ok(..)`. A shape mismatch would silently freeze `state.allowances`.
  - `AllowanceRow::expire` has no caller outside its own tests. Freshness is actually enforced by the 2 s recompute through `sanitise`.
- **Peer probe.** `--allowances-probe` prints `[row]` or `[]`. The row is the `summarise` output plus the `summarise_usage` output:
  - `account_key`, `sampled_at`, `plan`;
  - `weekly_remaining`, `weekly_resets_at`;
  - `reset_count`, `reset_expires_at`;
  - `lifetime_tokens`, `peak_daily_tokens`, `daily_usage`.
- **Private cache.** `allowances.json` holds at most four sanitised rows of the same shape.
- **Presentation.**
  - `State.project` reads `weekly_remaining`, `weekly_resets_at` and `window_seconds`, defaulting a missing duration to 604800.
  - It defaults `provider` to `codex` and synthesises the `Codex` label.
  - `Panel.accountAlias` branches on `provider === "codex"` to apply the legacy Personal and Work alias preferences.
  - `lifetime_tokens`, `peak_daily_tokens` and `daily_usage` have no reader in the presentation. The only reference is a JS test asserting they are ignored.
- **Omarchy agents plugin.** Its first-party plugin records a provider as `limits: [{label, percent, resetsAt, title?}]`, where `percent` is the fraction used from 0 to 1 and `resetsAt` is an ISO 8601 string. Alongside that it has `tierLabel`, `usageStatusText` and `authHelpText`, with one collector per provider.

The baseline measurement is in evidence.md. At baseline the presentation contains 2 `weekly_` occurrences, 1 `604800`, 3 `provider ==` comparisons and 5 quoted `codex` literals, in 2 files. A wire row is 409.5 bytes with 15 keys.

## Goals / Non-Goals

**Goals:**
- One provider-neutral row shape from the runtime to the popover, with source status and windows.
- A presentation that projects any provider's pacing window with no provider knowledge.
- Codex visuals byte-identical to the change 1 reference screenshots.
- Installed peers and the existing cache keep working with no update.

**Non-Goals:**
- A second collector.
- A change to `--allowances-probe` output or the cache format.
- An Omarchy adapter.
- Showing more than the pacing window on the card.
- Any change 3 work: Panel split, time separation, keyed models, shared tooltip, layout.

## Decisions

### D1. Wire contract: snapshot allowance row (interface between the lanes)

The `allowances` array in each snapshot line holds one object per configured account, in a stable order sorted by private account key. Every key is always present.

```json
{
  "provider": "codex",
  "provider_label": "Codex",
  "account_id": "Personal",
  "label": "Personal",
  "status": "available",
  "status_text": null,
  "plan": "pro",
  "sampled_at": 1800000000.25,
  "reset_count": 1,
  "reset_expires_at": null,
  "windows": [
    {"kind": "weekly", "label": "Weekly", "used_percent": 40.0,
     "resets_at": 1800302400, "duration_s": 604800, "pacing": true}
  ]
}
```

| Field | Type and bounds | Meaning |
| --- | --- | --- |
| `provider` | string, `^[a-z0-9][a-z0-9_.-]{0,31}$` | stable provider id, part of the account key and the provider group id |
| `provider_label` | string, 1 to 40 printable characters | group heading, supplied by the runtime; the view's only fallback is the row's own `provider` id, never a provider-specific name |
| `account_id` | string, `^[A-Za-z0-9_-]{1,40}$` | mapping id (today `Personal`, `Work` or a configured id) |
| `label` | string, 1 to 40 characters | configured label |
| `status` | `"available"`, `"unavailable"` or `"auth_needed"` | source status |
| `status_text` | null, or a string of 1 to 80 characters with no control characters | text supplied by the source; the view never writes one |
| `plan` | null or string (Codex: the existing `PLANS` allowlist) | counterpart of Omarchy `tierLabel`; not displayed today |
| `sampled_at` | null or float Unix seconds, the original source time | null unless `status` is `available` |
| `reset_count` | null or integer 0 to 10000 | native `availableCount` after expiry invalidation |
| `reset_expires_at` | null or integer Unix seconds | earliest pass expiry, if known |
| `windows` | array, 0 to 8 windows | `[]` unless `status` is `available` |

Each window object:

| Field | Type and bounds | Meaning |
| --- | --- | --- |
| `kind` | string, `^[a-z0-9_]{1,24}$` | e.g. `weekly`, `monthly`, `session`. The view never branches on it |
| `label` | string, 1 to 40 characters | display name, e.g. `Weekly` |
| `used_percent` | null or number 0 to 100 | percentage used. Remaining is `100 - used_percent` |
| `resets_at` | null or integer Unix seconds | null once past (see D2) |
| `duration_s` | integer 1 to 31622400 (366 days) | source-supplied window length |
| `pacing` | boolean | exactly one window in an available row is `true` |

Invariants the runtime guarantees and the view re-checks:

- `status != "available"` implies `windows == []` and `sampled_at == null`. `reset_count` and `reset_expires_at` are also null.
- In an available row, `used_percent` and `resets_at` are individually nullable. A null balance with a valid reset still shows the reset caption and reset count, as today.
- A window whose `resets_at` is at or before local time has both `used_percent` and `resets_at` set to null.

Top-level snapshot keys, host keys and every other field are unchanged.

### D2. Runtime: conversion at `snapshot()`, from legacy source rows

`sanitise`, `read_cache`, `receive`, `remote`, `summarise`, `summarise_usage` and `probe` keep their current input and output shape. `snapshot()` changes: it builds typed `model::AllowanceRow` values through one conversion function applied to the newest sanitised row per mapped account.

- **Identity.** `provider = "codex"` and `provider_label = "Codex"` stay in Rust. The runtime owns these as the Codex collector's identity. `account_id` and `label` come from `mapping()`.
- **Status.**
  - If a sanitised, mapped row exists, the status is `available`. Otherwise it is `unavailable`, with every other field null and `windows: []`.
  - `status_text` is always null for Codex.
  - Codex never yields `auth_needed`: the runtime cannot tell authentication failures from other RPC failures without parsing error text, which it must not do. `auth_needed` is exercised only by synthetic JS and QML fixtures until a collector can report it natively.
- **Windows.**
  - An available Codex row always has exactly one window: `{kind: "weekly", label: "Weekly", duration_s: 604800, pacing: true}`.
  - `used_percent = 100 - weekly_remaining` when `weekly_remaining` is present, otherwise null.
  - `resets_at` is `weekly_resets_at`.
  - `sanitise` has already nulled both when the reset has passed. Selection by duration stays in `summarise` (10080 minutes), so field or list order is never used.
- **Carried over.** `plan`, `sampled_at` (the original source time), `reset_count` and `reset_expires_at` come across unchanged. `sanitise` already invalidates the count on pass expiry.
- **Removed.** The token-activity fields are not copied (D4).
- **Typed rows.** `allowances::snapshot` returns `Vec<AllowanceRow>` directly, and `main.rs` assigns it. The `from_value` round trip, and with it the silent freeze, goes away.
- **Model.**
  - `model::AllowanceRow` becomes the D1 shape, with `AllowanceStatus` (serde `snake_case`) and `AllowanceWindow`.
  - `used_percent` is `Option<f64>`, so a future source may report fractions. Codex values are whole numbers, and serde emits them as, for example, `40.0`. The view treats both forms the same.
  - `AllowanceRow::expire` is deleted. Its expiry cases become tests of the conversion, and the 2 s recompute remains the freshness authority.
- **Status text.** Add a bounded `status_text` validator (1 to 80 characters, no control characters, else null) in `allowances.rs`, with unit tests, for future sources. Codex never calls it with a value.

**Alternatives considered.** One option was to emit windows from `summarise` and store them in the cache and peer output. It was rejected because an older local runtime reading a new peer, or reading after a downgrade, would lose the allowance: its `sanitise` only knows `weekly_remaining`. Keeping the source format fixed makes every combination of old and new runtimes work, with one conversion point to test.

### D3. Peer probe and cache compatibility

The new runtime's `--allowances-probe` output and `allowances.json` format stay byte-compatible with `cc5f982`, including the token-activity fields. The local side therefore only has to accept one peer shape, today's. `Sample`-style tolerance already applies: `sanitise` reads only known keys, so extra fields such as `theme`, `email` or future keys are dropped.

A later provider that needs a different source shape will add a new, versioned probe command, and the local runtime will keep accepting this one. That is out of scope here.

Legacy tests use a synthetic fixture file, `tests/fixtures/native-allowances-legacy.json`, holding:
- a cache array written in the `cc5f982` shape;
- peer probe outputs, including extra fields and token activity.

The tests assert the converted rows, and that `probe()` and `receive()` keep producing the legacy key set.

### D4. Token activity leaves the popover wire only

`lifetime_tokens`, `peak_daily_tokens` and `daily_usage` are removed from the snapshot row. The Codex `account/usage/read` call, `summarise_usage`, the cache fields and the peer fields remain. Removing them would change the peer output (D3) and the "Account token activity" collection requirement, for no gain in this change. The spec delta states that the popover snapshot carries none of them.

### D5. Presentation projection (`State.js`)

For each row, `project()` does the following.

1. **Validate identity.**
   - `provider` must match the D1 pattern.
   - `account_id` (or `label` when `account_id` is absent) must be 1 to 128 characters of `[A-Za-z0-9_.-]`.
   - The row is skipped if either is invalid, as today.
   - There are no defaults: a missing provider is invalid.
   - `providerLabel = label(row.provider_label, row.provider)`. That fallback is the provider id itself, never a provider-specific name, and is the only label the view may supply (D1, omarchy-companion delta).
2. **Status.**
   - `status` is one of the three values. Anything else, including a missing status, is `unavailable`. It gates freshness only and is not projected, because no card, Panel or IPC reader consumes it; the card gates on `remaining` and uses `statusText`.
   - `statusText` is `row.status_text` when it is a string of 1 to 80 characters with no control characters, otherwise null.
   - It applies to every status. Codex sends null.
3. **Freshness.** `fresh = status === "available" && age <= 600`, with the existing one-second future tolerance on `sampled_at`.
4. **Pacing window.**
   - When fresh, and `windows` is an array of at most 8 entries, collect the entries with `pacing === true`.
   - If exactly one exists, and it has a valid `duration_s` (integer, 1 to 31622400) and a valid `kind` and `label`, that is the pacing window.
   - Otherwise there is none.
   - Non-pacing windows are validated only for bounds and are not projected today.
5. **Values.**
   - `used` is valid when it is a finite number from 0 to 100.
   - `reset` is valid when it is a finite number greater than the current time.
   - `current = fresh && pacing && used valid && reset valid`.
   - `remaining = current ? 100 - used : null`.
   - `untilReset = fresh && pacing && reset valid ? reset - now : null`.
   - `timeRemaining = untilReset !== null && untilReset <= duration ? untilReset / duration * 100 : null`.
   - `paceDifference = current && timeRemaining !== null ? remaining - timeRemaining : null`.
   - `reset = untilReset !== null ? resetLabel(untilReset) : null`.
   - `resetCount` keeps today's rule (fresh, with a valid counter and an unexpired expiry).
   - `age` is as today.
6. **Projected allowance view** (11 keys): `id`, `provider`, `providerLabel`, `label`, `statusText`, `remaining`, `timeRemaining`, `paceDifference`, `resetCount`, `reset` and `age`. Nothing outside this list is added; the window list stays internal.

`resetLabel` already produces days and hours (for example `15d 0h` for a 30-day window with 15 days left), which suits long windows, so it is unchanged. The functions `allowancePaceReading`, `allowancePaceBand` and `providerGroups` are unchanged. A legacy-shaped row, with `weekly_remaining` and no `status`, projects as unavailable with no balance. That shape only reaches the view when the QML is newer than the runtime, which the installer does not allow.

### D6. Card and Panel

- `AllowanceCard.hint` becomes `known ? ui.paceText(entry) : (entry.statusText || "Allowance unavailable")`. `Accessible.name` already includes `hint`. The card gains no visible text, so screenshots are unaffected.
- `Panel.accountAlias` changes from `provider === "codex"` branches to a pure helper, `State.accountAlias(account, saved, legacy, pool)`:
  - `saved` is the parsed `accountAliases` object, keyed by `provider:id`, and still takes precedence.
  - `legacy` is `{"codex:Personal": personalAlias, "codex:Work": workAlias}`, keyed by `provider:label`, which keeps today's matching.
  - `pool` is the alias list, from which a deterministic hash choice is made, as today.
  - Panel passes the legacy table as data. The table is a settings migration, the only provider name left in the presentation, and not a projection branch.
  - `Panel.qml` is not loaded by the QML tests, so the helper is tested in Node.

### D7. Omarchy `limits` mapping (documentation only, no adapter)

| Anton window or row | Omarchy agents record | Conversion |
| --- | --- | --- |
| `windows[].label` | `limits[].label` (or `title`) | same string |
| `windows[].used_percent` | `limits[].percent` | `percent = used_percent / 100`; `used_percent = percent * 100`, clamped to 0 to 100, null if not finite |
| `windows[].resets_at` | `limits[].resetsAt` | Unix seconds to and from ISO 8601 UTC (`""` when null) |
| `windows[].duration_s`, `kind`, `pacing` | none | Omarchy carries no duration. Consuming it needs a source-supplied duration (a collector table or a new field); without one the window is non-pacing and pace stays unknown. Emitting drops these fields |
| `plan` | `tierLabel` | same string |
| `status_text` | `usageStatusText` | same bounded string |
| `status` | derived | `auth_needed` when the collector reports an authentication state (Omarchy shows `authHelpText`); `unavailable` when `limits` is empty with a status text; otherwise `available` |
| `reset_count`, `reset_expires_at` | none | not representable |

### D8. Fixtures and visual preservation

The QML fixtures in `tst_popup.qml` convert `account(id, balance, expected)` to D1 rows:

```text
provider: 'codex', provider_label: 'Codex', status: 'available', status_text: null
windows: [{kind: 'weekly', label: 'Weekly', used_percent: 100 - balance,
           resets_at: now / 1000 + 604800 * expected / 100, duration_s: 604800, pacing: true}]
```

`provider_label` must be supplied: the view no longer names Codex. For the fractional balance 0.2 in `connections-missing`, `100 - (100 - 0.2)` differs from 0.2 by about 3e-15. That rounds away in the percentage text, the one-decimal pace reading and pixel widths. The seven `anton-continuity-*.png` hashes in change 1's evidence (reproduced at `cc5f982` in this change's evidence) are the acceptance reference. `tst_components.qml` entries are already view-shaped, and need only the new keys where tests assert them.

### D9. Measurement

`tests/measure_anton_popover.mjs` gained additive metrics in commit `5bcfb18`. Existing definitions are unchanged.

- `allowance_wire` is a separate short run with a fake read-only Codex app-server and a fake-SSH peer answering as a second account. It reports rows, bytes per row, key names and windows per row.
- `allowance_contract` projects three neutral rows (Codex weekly, synthetic monthly, `auth_needed`) through the measured `State.js`, and reports the fields of the projected view.
- `provider_coupling` counts `weekly_`, `604800`, `provider ==` and quoted `codex` occurrences in `State.js` and `omarchy/herdr.observatory/*.qml`. It also reports the number of presentation files naming a provider, as a proxy for the files that must change to add a provider.

The existing `projection.allowance_fields` metric still projects the legacy-shaped rows in `jsSnapshot`. After this change those rows project as unavailable, so its value changes although its definition does not. Evidence reports it next to `allowance_contract.view_fields`.

## Risks / Trade-offs

- **QML newer than the runtime would show every account unavailable.** Mitigation: the installer copies QML and the runtime together, and the live check replaces both.
- **The Codex duration constant lives in the runtime.** Mitigation: that is the collector's knowledge, and `summarise` already selects the window by the same duration.
- **`used_percent` as `f64` changes Codex bytes** (`40` becomes `40.0`). Mitigation: this is harmless to the view, and the measurement records the per-row bytes.
- **`auth_needed` has no native producer yet.** Mitigation: the contract, view and tests support it, and no Codex error text is parsed to fake it.
- **Legacy alias table keeps two `codex:` keys in `Panel.qml`.** Mitigation: it is data for a preference migration and it is documented. The coupling metric's quoted-`codex` pattern does not match the `"codex:Personal"` and `"codex:Work"` keys, so it reports 0; evidence records them with a separate grep.
- **Two pre-existing lib test flakes** (recorded in change 1): `native::tests::checkpoint_startup_reconciles_expired_and_removed_hosts_without_empty_creation` and `allowances::tests::account_rpc_is_read_only_and_retains_quota_when_usage_unsupported`. Mitigation: the Rust lane reruns them and records whether they flake; it does not mask them.

## Migration Plan

- There is no configuration or cache migration. The parent installs the reviewed plugin (QML and runtime together) and warm-restarts Omarchy.
- Peers are not touched.
- Rollback is reinstalling the previous plugin payload. Because the cache and peer shapes are unchanged, the old runtime reads everything the new one wrote.

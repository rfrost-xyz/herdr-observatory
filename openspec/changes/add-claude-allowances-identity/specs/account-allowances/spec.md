# Delta for account-allowances

## MODIFIED Requirements

### Requirement: Account-bound allowance observation
The native plugin SHALL expose only explicitly labelled accounts, deduplicate the same account across machines, and use the actual source account identity rather than the source host as account authority. For Claude, that authority is the account Claude Code's provider state names when the local reporter runs, as Claude account observation defines. Ordinary telemetry SHALL contain only labels, plan, source status, bounded allowance window values and timestamps, with no raw account identifiers, email, credentials or session material. Collection SHALL be read-only and SHALL NOT redeem resets, change login, mount credentials or install a persistent host process.

A Codex source SHALL be a bounded native read-only account RPC, available independently of threads. A Claude source SHALL be the local reporter's private account state or Claude Code's account-matched usage cache, and SHALL depend on an active reporting session or a fresh cache. Remote sources SHALL invoke the marked native peer over SSH without a Docker exporter or Python helper.

Transport sanitisation and presentation SHALL tolerate a source allowance timestamp up to one second ahead of local time, preserving the original timestamp and strict cache ordering. Larger future offsets and observations older than ten minutes SHALL be rejected; a Claude observation's source time is the oldest stamp among its windows. Window reset and reset-credit expiry checks SHALL continue to use actual local time without a grace period.

#### Scenario: Account moves machine
- **WHEN** a Codex account is signed in on another configured host
- **THEN** the same configured Personal or Work label applies and duplicate observations produce one account panel.

#### Scenario: Unrecognised account
- **WHEN** a source reports an account not explicitly mapped in private configuration
- **THEN** its allowance data is absent from the popover and ordinary peer responses.

#### Scenario: Small peer clock offset
- **WHEN** a mapped native peer allowance is at most one second ahead of local time
- **THEN** it remains available with its original source time, while larger future offsets, stale samples and expired resets remain unavailable.

#### Scenario: Several mapped accounts across machines
- **WHEN** two mapped accounts are each observed on more than one configured source
- **THEN** the popover snapshot carries exactly one row per mapped account, each from its newest valid observation, in a stable order sorted by private account key.

### Requirement: Fleet-bound account sources
When fleet discovery is enabled, native remote allowance collection SHALL follow enabled saved machine profiles independently of whether their hosts have active threads. Explicit profile-bound source overrides SHALL retain private mappings and follow the profile's current target. A removed or disabled profile SHALL cease contributing an allowance source; an otherwise unreferenced target SHALL stop its probes and invalidate late results. Another enabled profile or explicit source for the same target SHALL retain that shared account worker. Label/session-only changes and unchanged discovery results SHALL preserve account collection cadence. Account identity SHALL remain authoritative: only explicitly mapped accounts SHALL appear, and observations of the same account on multiple machines SHALL produce one row. Discovery SHALL NOT create account mappings or expose account email in shared snapshots. Claude accounts SHALL be local-only: a peer SHALL NOT report a Claude observation or identity, and the local runtime SHALL NOT build a Claude row from a peer response.

#### Scenario: Newly discovered empty machine
- **WHEN** an enabled saved machine has no threads and its native peer reports an already mapped account
- **THEN** its valid allowance is eligible independently of thread activity and is deduplicated with other readings of that account.

#### Scenario: Removed source has an in-flight result
- **WHEN** a profile is disabled, removed or changes target during an allowance request, leaving its former target without any active reference
- **THEN** the retired result cannot restore its old source, while unrelated valid account readings remain available.

#### Scenario: Stable shared account route
- **WHEN** only a profile label/session changes, the inventory repeats unchanged, or another enabled profile still uses the same target
- **THEN** the account worker and its refresh cadence remain unchanged.

#### Scenario: Claude account used only on a peer
- **WHEN** a mapped Claude account is active only in Claude Code sessions on a configured peer
- **THEN** its row is unavailable in the local popover, the peer's allowance and identity probes carry no Claude data, and Codex rows from that peer are unaffected.

### Requirement: Provider-neutral allowance rows
Each allowance row in the popover snapshot SHALL describe its account without naming provider-specific fields. It SHALL carry:
- stable identity: provider, provider label, account id and label;
- a source status of available, unavailable or auth_needed;
- an optional source-supplied status text, bounded to a single printable line of at most 80 characters, or null;
- plan, source sample time, reset-pass count and reset-pass expiry, each null when unknown;
- a list of at most eight allowance windows.

Each window SHALL carry a kind, a display label, a used percentage from 0 to 100 or null, a reset time in Unix seconds or null, a positive duration in seconds of at most 366 days, and whether it is the pacing window. Every field SHALL be present in every row, with null where unknown. An unavailable or auth_needed row SHALL carry no windows, and neither SHALL a row whose source observation is stale. A row SHALL be available only while a valid, mapped, fresh source observation exists.

The runtime SHALL select the pacing window by duration or, for Claude, by window name, never by field or list order. A Codex observation SHALL yield exactly one pacing window of kind `weekly`. That window SHALL be chosen as the single source window whose duration is 10080 minutes, and SHALL carry a duration of 604800 seconds. A Claude observation SHALL map `five_hour` to a window of 18000 seconds that is not pacing and `seven_day` to the single pacing window of 604800 seconds, SHALL ignore every other window name, and MAY carry used values with one decimal. A window whose reset time has passed SHALL have both its used percentage and its reset time invalidated. Reset-pass expiry SHALL invalidate the reset-pass count without a refill. The reset-pass count SHALL be the source's native available count, and null for a source without reset passes, such as Claude. Bounds and original source times SHALL be enforced when rows are built. The runtime SHALL NOT synthesise status text, windows or rows for a provider or account that has no configured mapping.

#### Scenario: Codex account with a weekly allowance
- **WHEN** a mapped Codex account reports 40% used of its 10080-minute window, resetting in the future, and one available reset pass
- **THEN** its row has status available, null status text, and one window of kind `weekly` with used percentage 40, that reset time, duration 604800 and pacing true, plus reset count 1.

#### Scenario: Window order does not select the pacing window
- **WHEN** a Codex source lists a 300-minute window first and the 10080-minute window second
- **THEN** the pacing window is the 10080-minute window, with its own used percentage and reset time.

#### Scenario: Past reset invalidates the balance
- **WHEN** a mapped observation's weekly reset time is at or before local time
- **THEN** the pacing window's used percentage and reset time are both null, while a valid reset-pass count remains.

#### Scenario: Mapped account without a current observation
- **WHEN** a configured account has no valid, fresh, mapped observation
- **THEN** its row has status unavailable, null status text and no windows, with its identity preserved.

#### Scenario: Malformed or oversized values
- **WHEN** an observation carries a non-integer Codex used value, a reporter-supplied Claude used value with more than one decimal, an out-of-range used value, a negative reset count, more than the bounded number of rows, or unknown private fields
- **THEN** the affected values are null or the input is rejected, and no unknown field reaches the popover snapshot.

#### Scenario: Claude account with both windows
- **WHEN** a mapped Claude account has fresh `five_hour` 12.5% and `seven_day` 40% windows, both resetting in the future
- **THEN** its row has provider `claude`, status available, null plan and reset count, a `five_hour` window of 18000 seconds that is not pacing and a `seven_day` pacing window of 604800 seconds with used percentage 40.

#### Scenario: Five-hour-only Claude row
- **WHEN** a mapped Claude account's fresh observation has only a `five_hour` window
- **THEN** the row is available with that one window and no pacing window, so its balance and pace are unknown.

## ADDED Requirements

### Requirement: Claude account observation
The local runtime SHALL build a mapped Claude account's observation from the reporter's private account state and, as a fallback, from Claude Code's usage cache in `$HOME/.claude.json`, used only when the cache names the same account as the file's account profile. It SHALL read only `oauthAccount.accountUuid`, `oauthAccount.emailAddress` for identity refresh and `cachedUsageUtilization`, skip the whole file when `primaryApiKey` is present, read nothing from it when `CLAUDE_CONFIG_DIR` is set or a legacy `~/.claude/.config.json` exists in the reader's own environment and home, and keep Claude state out of the Codex allowance cache and probe. It SHALL NOT log, persist or retain any other value, and SHALL keep the account id only long enough to hash it. Cache utilisation SHALL be rounded half up to one decimal. The reporter's private account state SHALL be read only when it is a regular file owned by the user, private to the user, within its size bound and of a known version with no unknown keys; otherwise it contributes nothing.

#### Scenario: No active Claude session
- **WHEN** no Claude Code session has reported fresh rate limits for a mapped Claude account for more than ten minutes and the cache is older than ten minutes
- **THEN** the row is unavailable with no windows, and no window, balance or zero is invented.

#### Scenario: Cache fallback for the same account
- **WHEN** the cache names a mapped Claude account with a fetch time under ten minutes old that is newer than the reporter's stamp for a window, or the reporter has no stamp for that window
- **THEN** the cache supplies that window's utilisation and reset time, with the fetch time as its stamp, and the reporter supplies any window whose stamp is newer.

#### Scenario: Cache for another account or stale
- **WHEN** the cache's account differs from the account profile, or its fetch time is older than ten minutes or in the future beyond one second
- **THEN** the cache contributes nothing to any row.

#### Scenario: Ambiguous cache scale
- **WHEN** a cache utilisation value lies above 0 and at or below 1, where a fraction and a percentage cannot be told apart
- **THEN** that window's used value is unknown, while 0 and values above 1 up to 100 are read as percentages.

#### Scenario: API key in provider state
- **WHEN** `$HOME/.claude.json` contains `primaryApiKey`
- **THEN** the collector reads no identity or cache from it, the key command prints nothing, the reporter writes no account state, and the Claude row turns unavailable once its earlier stamps are older than ten minutes.

#### Scenario: Unsafe or unknown account state
- **WHEN** the reporter's account state file is a symbolic link, owned by another user, readable by group or others, oversized, of another version or holds an unknown key
- **THEN** it contributes nothing to any row, and a window whose used value breaks the one-decimal grammar is dropped from an otherwise valid file.

#### Scenario: Codex cache untouched
- **WHEN** Claude rows are present
- **THEN** the Codex allowance cache and the `--allowances-probe` output keep their existing shape and contain no Claude rows.

### Requirement: Claude allowance window stamps
Each Claude window SHALL carry its own source stamp. The local account state SHALL keep, per account and window, the observation with the newest stamp and ignore an older or equal one. A row's source time SHALL be the oldest stamp among its windows, so a row with any window older than ten minutes is stale. A window whose reset time has passed SHALL keep its stamp and lose its values.

#### Scenario: One fresh and one stale window
- **WHEN** a mapped Claude account's `five_hour` window was stamped one minute ago and its `seven_day` window eleven minutes ago
- **THEN** the row's source time is eleven minutes ago, so the row is unavailable with no windows until a fresh `seven_day` stamp arrives.

#### Scenario: Older report arrives late
- **WHEN** a report with an older stamp for a window arrives after a newer one for the same account
- **THEN** the newer window values and stamp remain.

#### Scenario: Reset passes without a new sample
- **WHEN** a fresh window's reset time passes and no new report arrives
- **THEN** its used value and reset time are null while its stamp is unchanged, and the stamp is never renewed by the reset.

### Requirement: Provider account mappings
Each account mapping SHALL have a provider, `codex` when the field is absent or `claude`. A Claude mapping SHALL be keyed by the SHA-256 hash of `observatory-claude-account-v1:` and the account id, so it cannot collide with a Codex key. The runtime SHALL offer a local command that prints only the hashed key of the current Claude account, and never its id or email. The mapping limit of four accounts SHALL be shared by all providers.

#### Scenario: Mapping with a provider field
- **WHEN** private configuration maps a Claude account key with provider `claude`, label and category, beside Codex mappings without a provider field
- **THEN** the configuration is valid, Codex rows are unchanged, and the Claude account appears as one row under provider `claude` once observed.

#### Scenario: Unknown provider or unmapped account
- **WHEN** a mapping names an unknown provider, or a Claude account reports without a mapping
- **THEN** the configuration is rejected, or the unmapped account's data is absent from the popover.

#### Scenario: Key command
- **WHEN** the user runs the key command with an attributable local Claude account
- **THEN** it prints only the 64-character hashed key; without an attributable account it prints nothing and exits non-zero.

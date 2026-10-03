# Delta for harness-telemetry

## MODIFIED Requirements

### Requirement: Supplementary harness reports
The installed Pi adapter SHALL report supported tool, model, phase and compaction events as expiring Herdr presentation metadata, without changing lifecycle authority, session references or sending agent input. Reports SHALL be limited to the matching native session in the configured local Herdr instance. The Claude Code mod, installed only by the local plugin and never on a peer, SHALL report only the Claude Code context window as pane metadata, without a display label, and MAY pass fresh subscription rate-limit windows to its reporter for the plugin's private local account state, under the same lifecycle, session and input restrictions.

#### Scenario: Matching session
- **WHEN** a supported Pi event occurs in the identified Herdr pane
- **THEN** Herdr exposes supplementary activity metadata and a concise display label while preserving its semantic state.

#### Scenario: Matching Claude Code session
- **WHEN** the Claude Code mod reports a window for the session bound to the identified Herdr pane
- **THEN** the pane's metadata carries the window only, and the pane's display label and semantic state are unchanged.

#### Scenario: Unavailable or replaced session
- **WHEN** the native helper or Herdr is unavailable, the adapter runs outside Herdr, or its native session no longer matches
- **THEN** reporting fails silently within a bounded time without blocking or changing the harness operation.

#### Scenario: Claude Code mod outside a bound pane
- **WHEN** the Claude Code mod observes a window outside Herdr, without a pane, before Herdr binds the pane to the reported session id, or after the session changed
- **THEN** no metadata is written, the event continues unchanged, and a later event in the bound session may report again.

#### Scenario: Claude Code rate limits stay off pane metadata
- **WHEN** the Claude Code mod reports a window together with fresh rate-limit windows for the session bound to the identified Herdr pane
- **THEN** the pane's metadata carries the window only, with no rate-limit, account or attribution field, and the rate limits reach only the private local account state.

### Requirement: Claude Code context window reporter
The plugin MAY install a Claude Code mod as a personal skills-directory plugin that reports the context window Claude Code uses for its current session. The mod SHALL observe events only, SHALL always pass each event on unchanged, SHALL never throw into or wait on the harness for the reporter process, and SHALL run the installed native reporter only inside a Herdr pane, with the pane, a strictly increasing epoch-based sequence, the Claude Code session id, a positive bounded integer window and, only from a session measurement with fresh rate-limit or cost evidence, at most two bounded rate-limit windows as arguments only, without setting the process-run standard input, environment or working-directory options. The mod SHALL start a reporter run only within a hook invocation, SHALL skip a sample while a recent run is in flight rather than holding it for later, and SHALL handle every rejection of the promises it creates. The mod SHALL skip a report when its clock does not read epoch milliseconds. The reporter SHALL NOT depend on its working directory and SHALL fail closed when it cannot resolve absolute home and state paths. It SHALL NOT send the model, prompts, messages, costs, spend limits or account data, and SHALL send rate limits only as those bounded windows. The native reporter SHALL require the plugin owner, a receipt recording the mod, exactly one configured local host and a Herdr pane whose agent is `claude` with a `herdr:claude` session of kind `id` equal to the reported id, and SHALL write one bound versioned metadata report for agent `claude` carrying only the window, with no display label, usage source time or totals. An unchanged bound window SHALL NOT be rewritten. Any failure SHALL write no metadata; without an earlier bound report for the same session the window stays unknown, and a stale window never hides replay context.

#### Scenario: First turn in a bound Claude Code session
- **WHEN** a Claude Code session that loaded the mod completes a turn in a Herdr pane bound to its session id
- **THEN** the pane receives one bound window report, and the local popover shows the context window and percentage for that thread.

#### Scenario: Mods unavailable or reporter refused
- **WHEN** mods are disabled, the mod is not loaded, the runtime or its mod receipt is missing, or Herdr reports another agent or session
- **THEN** the harness continues unchanged, no metadata is written, and the thread's window and percentage stay unknown unless an earlier bound report for the same session supplied them.

#### Scenario: Remote Claude Code thread
- **WHEN** a peer returns a Claude Code thread
- **THEN** its window and percentage are unknown in the local popover, whatever metadata the peer carries, and the peer envelope is unchanged.

#### Scenario: Unsafe arguments
- **WHEN** the reporter receives a value count other than four, seven or ten, an invalid pane, an unsafe session id, a sequence beyond the bound, a zero, signed, non-decimal or oversized window, or a rate-limit tail with an unknown or repeated kind, more than two windows, a used value outside 0 to 100 or with more than one decimal, or a reset time that is not an epoch second after the sequence's whole second and within the window's duration plus one hour of it
- **THEN** it exits without reading configuration, connecting to Herdr or writing metadata.

#### Scenario: Reporter without a usable environment
- **WHEN** the reporter runs with no usable home or a relative state location in its environment or arguments, from an arbitrary working directory
- **THEN** it ignores a relative environment state location, resolves the home from the user account, refuses a relative explicit state location or an unresolvable home, and never creates a file relative to its working directory.

#### Scenario: Report process that never finishes
- **WHEN** a reporter run started by the mod has not settled after a bounded time
- **THEN** events inside that time start no run and are not replayed later, a later event may start a new run, and the late result of the earlier run does not change the mod's state.

#### Scenario: Mod API failure during a report
- **WHEN** the mod's clock or process-run call throws or rejects while it starts or awaits a reporter run
- **THEN** the event continues unchanged, no unhandled rejection reaches the harness, and a later event can still report.

## ADDED Requirements

### Requirement: Claude Code rate-limit evidence
The Claude Code mod SHALL pass rate-limit windows only from a `session.measure` event that is fresh. A measurement SHALL be fresh only when the mod observed an earlier measurement for the same session id since it loaded or since the last session end, no `spend_limit` window was listed by it or by an earlier measurement since the last session end, and at least one of these holds: its changed units include the rate limits and a `five_hour` or `seven_day` window's used value differs from the previous measurement or a window appeared; or its changed units include the cost and the session's cost total, a finite non-negative US dollar figure, is strictly greater than the total of the previous measurement, which was itself a finite non-negative figure. The mod SHALL keep, per module load, only the latest measurement's session id, `five_hour` and `seven_day` used values and cost total as the comparison point, SHALL replace it on every `session.measure` for which it reads a session id, including measurements it skips, and SHALL clear it at session end. The mod SHALL hold the cost total only in memory and SHALL NOT pass, log or store it. The first measurement after the mod loads, after a session end or for another session id, session start and resume readings, a window that only left, a reset time that changed without a used value, a measurement whose only changed unit is the context, a cost named as changed without a strictly greater total, and a greater total without the cost named as changed SHALL NOT be fresh. A fresh measurement SHALL pass the `five_hour` and `seven_day` windows it lists, with their current used values and reset times. A measurement listing any `spend_limit` window SHALL pass no rate limits, and neither SHALL any later measurement before the session ends.

#### Scenario: Window moves a whole point
- **WHEN** a measurement in a bound session names the rate limits as changed and the `five_hour` used value moved from 12 to 13 since the previous measurement
- **THEN** the reporter receives both present windows, each with its used value, kind and reset time in epoch seconds, and the account state stamps them at the report's own time.

#### Scenario: Cost grows with unchanged percentages
- **WHEN** a measurement in a bound session names the context and the cost as changed, its cost total is strictly greater than the previous measurement's, and both windows keep the used values of the previous measurement
- **THEN** the measurement is fresh, the reporter receives both windows with their current used values and reset times, the account state stamps them at the report's own time, and the cost total is not passed.

#### Scenario: Cost named without growth
- **WHEN** a measurement names the cost as changed but its total is equal to or lower than the previous measurement's, or either total is absent or not a finite non-negative number, and no window moved or appeared
- **THEN** no rate limits are passed for that measurement, and it becomes the comparison point for the next one.

#### Scenario: First measurement after the mod loads or a session ends
- **WHEN** the mod loads into a running or resumed session, or a session ends and a new one starts, and the first measurement names every unit, including the rate limits and a non-zero cost total
- **THEN** no rate limits are passed for that measurement, and it only sets the comparison point, cost total included, for later measurements.

#### Scenario: Rewind
- **WHEN** a rewind changes the context fill, and the measurement names only the context as changed, with the same cost total and used values as the previous measurement
- **THEN** no rate limits are passed and no account window stamp is renewed.

#### Scenario: Compaction without cost growth
- **WHEN** a compaction changes the context fill and the measurement shows no cost growth over the previous measurement and no window that moved or appeared
- **THEN** no rate limits are passed and no account window stamp is renewed.

#### Scenario: Cost grows during a rewind or compaction
- **WHEN** a measurement taken for a rewind or compaction names the cost as changed and its cost total is strictly greater than the previous measurement's, for example because a priced call completed since then
- **THEN** the measurement is fresh, and the reporter receives the `five_hour` and `seven_day` windows it lists with their current used values and reset times.

#### Scenario: Reset time passes
- **WHEN** a window's reset time passes, or the window leaves the rate-limit list, without a new value for any remaining window and without cost growth
- **THEN** nothing is passed as a new sample, and the earlier stamp and the collector's past-reset invalidation decide what is shown.

#### Scenario: Spend limit present
- **WHEN** a measurement lists a `spend_limit` window, whatever else it lists and whether or not the cost grew
- **THEN** the mod passes no rate limits for that measurement or any later one until the session ends, even when a later measurement omits `spend_limit`, the window report still proceeds, and no account state is written.

#### Scenario: Evidence during an in-flight run
- **WHEN** a measurement with fresh rate-limit or cost evidence arrives while a recent reporter run is in flight
- **THEN** the measurement is skipped as in change 3, it still becomes the comparison point, cost total included, and its evidence is not replayed later.

### Requirement: Claude Code account attribution
The reporter SHALL attribute passed rate limits to the account that `$HOME/.claude.json` names when it runs, and only after the pane binding and sequence checks and after the window report's outcome is decided, and only when the window report did not fail, so attribution can never change that outcome. It SHALL read that file bounded, owner-checked, without following links, extracting only `oauthAccount.accountUuid` and the presence of `primaryApiKey`. It SHALL refuse attribution whenever the environment, the configuration location or user settings could select another credential or API endpoint, and SHALL never retain, log or store any other value. A refused attribution SHALL write no account state and SHALL NOT change the window report or its exit status.

#### Scenario: Window report fails
- **WHEN** a report carries rate limits and its pane metadata write fails
- **THEN** the reporter exits with its failure status, does not attempt attribution and writes no account state.

#### Scenario: Credential variable present
- **WHEN** any variable name in the reporter's environment is not valid UTF-8, or a name matches `ANTHROPIC_*KEY*`, `ANTHROPIC_*TOKEN*`, `ANTHROPIC_CUSTOM_HEADERS`, `ANTHROPIC_BASE_URL`, `CLAUDE_CODE_*TOKEN*`, `CLAUDE_CODE_*_FILE_DESCRIPTOR`, `CLAUDE_CODE_HOST_*`, `CLAUDE_CODE_USE_*`, `CCR_OAUTH_TOKEN_FILE` or `CLAUDE_CODE_CUSTOM_OAUTH_URL`, where `*` matches zero or more characters, and the name is not on the closed exemption list
- **THEN** no account state is written, and the decision reads variable names only, never their values.

#### Scenario: Exempt session variable
- **WHEN** the only matching name is `CLAUDE_CODE_MESSAGING_TOKEN`, the single name on the closed exemption list
- **THEN** attribution proceeds; any other matching name, including another `CLAUDE_CODE_*TOKEN*` name beside it, still refuses.

#### Scenario: API base URL set
- **WHEN** `ANTHROPIC_BASE_URL` is set in the reporter's environment to any value, including an empty one
- **THEN** no account state is written, the window report and its exit status are unchanged, and the decision reads the name only, never the value.

#### Scenario: Configuration directory set
- **WHEN** `CLAUDE_CONFIG_DIR` is set to any value, including one naming `~/.claude`
- **THEN** no account state is written.

#### Scenario: API key in provider state
- **WHEN** `~/.claude.json` contains `primaryApiKey`, whatever its value
- **THEN** the whole file is skipped, nothing is attributed, and no other value from it is kept.

#### Scenario: API key helper or legacy configuration
- **WHEN** user settings set `apiKeyHelper`, or a legacy `~/.claude/.config.json` exists
- **THEN** no account state is written, and nothing else is read from the settings file.

#### Scenario: Unsafe provider state
- **WHEN** `~/.claude.json` is a symbolic link, is owned by another user, is readable or writable by group or others, exceeds its size bound, or has no valid account id
- **THEN** no account state is written.

#### Scenario: Account changes mid-session
- **WHEN** a session's rate limits were attributed to one account and a later report for the same session finds another account in `~/.claude.json`
- **THEN** that report writes nothing, and no later report for that session is attributed to any account.

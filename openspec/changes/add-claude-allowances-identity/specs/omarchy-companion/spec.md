# Delta for omarchy-companion

## MODIFIED Requirements

### Requirement: Private account identity display
Allowance rows SHALL display the verified account email instead of category labels. Clicking anywhere on any allowance row SHALL toggle all account emails together, using distinct randomly chosen Silicon Valley character aliases persisted locally. Names SHALL NOT be separate controls. Concealed email SHALL NOT appear in tooltip or accessibility text. Native read-only identity RPCs and, for Claude, the local provider-state account profile SHALL be matched against existing account mappings of the same provider by hashed account key before an email is stored or shown, with emails kept out of the shared API and source checkout. Uninstall SHALL remove the private identity file and preferences.

#### Scenario: Conceal account identity
- **WHEN** the operator clicks any allowance row
- **THEN** character aliases replace all emails and remain after a shell restart, until any row is clicked again.

#### Scenario: Claude email after the mapping check
- **WHEN** identity refresh reads a local Claude account profile whose hashed account id matches a Claude mapping
- **THEN** that email is stored only in the private identity file under the mapping and shown on the Claude row, and a profile that matches no mapping, a file holding `primaryApiKey`, or an identity row from a peer or the Codex RPC carrying a Claude key stores no email.

### Requirement: Account refresh independent of agents
While enabled, the plugin SHALL periodically request bounded account refreshes using its native local account reader and explicitly configured SSH peer sources, even when no Herdr threads exist. Claude accounts are the exception: their allowance depends on an active local Claude Code session reporting through the mod, or on Claude Code's own fresh usage cache, so a Claude row becomes unavailable when neither is current. This SHALL retain the sanitised cache/collector path and SHALL NOT start agents or install another persistent service. Missing readings SHALL remain unknown until measured data arrives.

#### Scenario: Remote host has no threads
- **WHEN** ws-255 is reachable but has no active Herdr thread
- **THEN** its account allowance is refreshed and displayed independently of agent hooks.

#### Scenario: No active Claude session
- **WHEN** a mapped Claude account has no Claude Code session reporting and its cache is stale
- **THEN** its row turns unavailable once its oldest window stamp and the cache are more than ten minutes old, while Codex rows keep refreshing independently of threads.

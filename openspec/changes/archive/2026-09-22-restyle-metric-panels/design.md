## Context
The current card repeats native state and uses last-response numbers that differ from the user's cumulative Codex statusbar. The requested visual reference is btop's compact labelled borders, gauges and measured history.

## Decisions
Keep rendering browser-native and themed. Replace the inspector with card-local hover/focus disclosure, and use a bounded local click impulse without a text-effect library on cards. Keep summary boxes persistent and hide configuration labels from the header. Bound host histories and append only distinct measured timestamps; clear history on source expiry.

Add optional numeric fields through the existing helper, reporter and feed allowlist. Codex totals come directly from total_token_usage, not sums of the tail. Codex context percentage uses the pinned 0.155.1 statusbar formula including its 12000-token baseline. Its UsedTokens statusbar item is uncached cumulative input plus output, not current context. Pi normalises provider cache accounting through its documented session APIs. Compaction counts require full coverage; truncated files/collections yield unknown.

## Boundaries and risks
No session directory mount, new daemon, raw message export or provider API request. Existing privacy/session checks remain. Codex schema is version-specific and may change; fail closed. Pi provider omissions remain unknown. A compaction count cannot be inferred from a truncated tail. Labels must distinguish session totals, latest context and last response.

## Delivery
Test both adapters and all browser modes; independently review, build one immutable image and deploy it and its payloads to both hosts. Preserve rollback image/config, verify Work disclosure and real numeric coverage, update documentation, synchronise/archive and publish the existing unmerged PR.

## Native metadata limit found during deployment
Herdr rejects more than 16 token updates per report, including null clears. Expanded metrics therefore use an atomic versioned numeric encoding with four bounded groups alongside identity/event fields. Each group remains within the native 80-character value limit; the public normalised schema remains individual numeric fields. Version 1 reading remains supported during migration, while version 2 ignores retained legacy numeric tokens and rejects malformed/missing groups. Never split a sample across reports sharing a sequence.

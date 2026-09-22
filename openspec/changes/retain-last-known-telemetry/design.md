## Decision
Omit the native metadata TTL so Herdr retains the last report for the open pane. Keep the existing session binding and timestamp validation. Preserve source times in the browser and label observations over two minutes old as last known. A newer report replaces the old values, including unavailable numeric fields.

Derive the cache percentage from cumulative cache-read tokens divided by cumulative input tokens only when the cache-read, uncached and optional write components reconcile. Keep token counts visible and exact counts available to assistive technology. Do not infer request-level cache hits or misses from token totals.

## Risk and verification
Retained data must not be presented as current activity or cross a session boundary. Verify source loss, session replacement, a newer report with unavailable usage, incomplete balances, zero input and cache writes. Preserve Work filtering and rollback images during deployment.

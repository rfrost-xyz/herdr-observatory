#!/bin/sh
# herdr-observatory adapter v1; installed from the running image
[ "$HERDR_ENV" = 1 ] && [ -n "$HERDR_PANE_ID" ] || exit 0
# No output: hooks must never inject context or affect tool permission decisions.
timeout 2s docker exec -i __CONTAINER__ python3 -m observatory.telemetry codex "$HERDR_PANE_ID" "$(date +%s%6N)" >/dev/null 2>&1 || :
exit 0

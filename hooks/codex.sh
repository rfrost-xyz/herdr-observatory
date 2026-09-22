#!/bin/sh
# herdr-observatory adapter v1; installed from the running image
[ "$HERDR_ENV" = 1 ] && [ -n "$HERDR_PANE_ID" ] || exit 0
# No output: hooks must never inject context or affect tool permission decisions.
timeout 8s python3 "$(dirname "$0")/codex_usage.py" >/dev/null 2>&1 || :
exit 0

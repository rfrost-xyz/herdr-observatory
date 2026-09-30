#!/usr/bin/env bash
# Runs the Anton QML tests. Fails on any test failure and on binding errors
# (TypeError, ReferenceError, "Unable to assign"), which qmltestrunner only
# logs as warnings, so a component tree that does not resolve cannot pass.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
# QMLTESTRUNNER selects a specific Qt 6 build (CI sets it to the installed Qt).
runner=${QMLTESTRUNNER:-/usr/lib/qt6/bin/qmltestrunner}
[[ -x $runner ]] || runner=$(command -v qmltestrunner6 || command -v qmltestrunner || true)
[[ -n $runner && -x $runner ]] || { echo 'Qt 6 qmltestrunner is required for the QML checks' >&2; exit 1; }
cd -- "$root"
log=$(mktemp)
# Preference and palette tests write only inside this private directory; the
# FileView stub and preference copies read and write it through XMLHttpRequest.
scratch=$(mktemp -d)
trap 'rm -f -- "$log"; rm -rf -- "$scratch"' EXIT
status=0
TMPDIR=$scratch QML_XHR_ALLOW_FILE_READ=1 QML_XHR_ALLOW_FILE_WRITE=1 \
  QT_QPA_PLATFORM=offscreen QT_QUICK_CONTROLS_STYLE=Basic "$runner" -import tests/qml/anton -input tests/qml/anton -o -,txt 2>&1 | tee "$log" || status=$?
errors=$(grep -cE 'TypeError|ReferenceError|Unable to assign' "$log" || true)
if (( errors > 0 )); then
  echo "QML binding errors: $errors (see the log above)" >&2
  (( status != 0 )) || status=1
fi
exit "$status"

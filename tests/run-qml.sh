#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
runner=/usr/lib/qt6/bin/qmltestrunner
[[ -x $runner ]] || runner=$(command -v qmltestrunner6 || command -v qmltestrunner || true)
[[ -x $runner ]] || { echo 'Qt 6 qmltestrunner is required for the QML checks' >&2; exit 1; }
cd -- "$root"
QT_QPA_PLATFORM=offscreen QT_QUICK_CONTROLS_STYLE=Basic "$runner" -import tests/qml/anton -input tests/qml/anton -o -,txt

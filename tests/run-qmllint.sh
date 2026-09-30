#!/usr/bin/env bash
# Lints the plugin QML with Qt 6 qmllint against the repository's stub modules
# (tests/qml/anton: qs.Commons, Quickshell, Quickshell.Io) and Qt's own modules
# only. A scratch import directory links just the Qt modules, and --bare drops
# the default import paths, so an installed Quickshell or Omarchy shell is never
# resolved in place of a stub.
#
# Every plugin file except Panel.qml must report no warnings. Panel.qml extends
# qs.Ui Panel, which has no stub, so it may report only the consequences of that
# missing module: the qs.Ui import and its types (import), the unresolved base
# and grouped types (unresolved-type, inheritance-cycle) and members of those
# unresolved types (unqualified; Qt 6.8 reports them as missing-property and the
# unresolved base as an import inheritance cycle). A missing-property warning is
# accepted only as a qs.Ui consequence: a child assigned to an unresolved
# parent's default property, or a property written on the unresolved Panel,
# BarIconButton, KeyboardPanel or PanelKeyCatcher. Any other missing-property
# warning, such as `Member "relaod" not found on type "AntonTheme"`, fails. Its
# warnings are printed for review.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
# QMLLINT selects a specific Qt 6 build (CI sets it to the installed Qt).
lint=${QMLLINT:-/usr/lib/qt6/bin/qmllint}
[[ -x $lint ]] || lint=$(command -v qmllint6 || command -v qmllint || true)
[[ -n $lint && -x $lint ]] || { echo 'Qt 6 qmllint is required for the lint checks' >&2; exit 1; }
qml_dir=${QT_QML_DIR:-$(cd -- "$(dirname -- "$lint")/../qml" 2>/dev/null && pwd -P || true)}
[[ -n $qml_dir && -d $qml_dir/QtQuick ]] || { echo "Qt QML modules not found; set QT_QML_DIR" >&2; exit 1; }
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
mkdir -- "$scratch/qt"
for module in QtQml QtQuick QtCore QtTest builtins.qmltypes jsroot.qmltypes; do
  [[ ! -e $qml_dir/$module ]] || ln -s -- "$qml_dir/$module" "$scratch/qt/$module"
done
cd -- "$root"
"$lint" --version
status=0
# Properties Panel.qml writes on its unresolved qs.Ui types (Panel, BarIconButton,
# KeyboardPanel, PanelKeyCatcher) and on their anchors group.
ui_properties='implicitHeight|implicitWidth|ipcTarget|manageIpc|moduleName|anchorItem|bar|contentHeight|contentWidth|focusTarget|open|owner|padding|fill|foreground|text|tooltipText|useActiveColor'
for file in omarchy/herdr.observatory/*.qml; do
  output=$("$lint" --bare -I tests/qml/anton -I "$scratch/qt" "$file" 2>&1 || true)
  warnings=$(grep -E '^Warning' <<<"$output" || true)
  if [[ ${file##*/} == Panel.qml ]]; then
    echo "Panel.qml warnings (qs.Ui has no stub):"
    if [[ -n $warnings ]]; then
      grep -oE '\[[a-z-]+\]$' <<<"$warnings" | sort | uniq -c
      unexpected=$(grep -vE '\[(import|unresolved-type|inheritance-cycle|unqualified|missing-property)\]$' <<<"$warnings" || true)
      unrelated=$(grep -E '\[import\]$' <<<"$warnings" | grep -vE 'qs\.Ui|(KeyboardPanel|PanelKeyCatcher|BarIconButton) was not found|Panel is part of an inheritance cycle' || true)
      unqualified=$(grep -A1 -E '\[unqualified\]$' <<<"$output" | grep -vE '^(Warning|--)' | grep -vE '^\s*anchors\.fill: parent\s*$' || true)
      missing=$(grep -E '\[missing-property\]$' <<<"$warnings" | grep -vE ': (Cannot assign to non-existent default property|Could not find property "('"$ui_properties"')"\.) \[missing-property\]$' || true)
      if [[ -n $unexpected$unrelated$unqualified$missing ]]; then
        echo "Unexpected Panel.qml warnings:" >&2
        printf '%s\n' "$unexpected" "$unrelated" "$unqualified" "$missing" | sed '/^$/d' >&2
        status=1
      fi
    fi
  elif [[ -n $warnings ]]; then
    echo "$output" >&2
    status=1
  fi
done
(( status == 0 )) && echo "qmllint: no warnings outside Panel.qml"
exit "$status"

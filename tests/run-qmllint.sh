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
# unresolved base as an import inheritance cycle).
#
# On Qt 6.8, a missing-property warning is accepted only where it is tied to a
# qs.Ui object by location as well as by message:
# - `Could not find property "<name>"` on a line whose enclosing object is the
#   Panel root, BarIconButton, KeyboardPanel or PanelKeyCatcher, for a property
#   Panel writes on that type;
# - `Cannot assign to non-existent default property` on a line that opens a
#   child object directly inside one of those types.
# A brace-tracking awk pass over Panel.qml finds each line's enclosing object.
# Qt 6.11 reports none of these, so there every missing-property fails. Qt 6.9
# and 6.10 are refused (see below). Anything else fails, such as a qs.Ui
# property name or a stray child written inside AntonController, or
# `Member "relaod" not found on type "AntonTheme"`. The Panel.qml warnings are
# printed for review.
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
version=$("$lint" --version)
echo "$version"
[[ $version =~ ([0-9]+)\.([0-9]+) ]] || { echo "Cannot read the qmllint version" >&2; exit 1; }
major=${BASH_REMATCH[1]} minor=${BASH_REMATCH[2]}
# Qt 6.9 and 6.10 also report every read of a qs.Ui member as missing-property,
# on type "Panel", "KeyboardPanel", "BarIconButton" or "", which this gate
# cannot tie to qs.Ui reliably, so only 6.8 and 6.11 or later are supported.
if (( major == 6 && minor == 8 )); then
  strict_missing=0
elif (( major > 6 || (major == 6 && minor >= 11) )); then
  strict_missing=1
else
  echo "Unsupported qmllint $major.$minor: use Qt 6.8 or Qt 6.11 or later" >&2
  exit 1
fi
status=0
# Properties Panel.qml writes on each unresolved qs.Ui type, including the
# anchors group (fill).
declare -A ui_properties=(
  [Panel]='implicitHeight|implicitWidth|ipcTarget|manageIpc|moduleName'
  [BarIconButton]='fill|bar|foreground|text|tooltipText|useActiveColor'
  [KeyboardPanel]='anchorItem|bar|contentHeight|contentWidth|focusTarget|open|owner|padding'
  [PanelKeyCatcher]='fill'
)
ui_types='Panel|BarIconButton|KeyboardPanel|PanelKeyCatcher'
# Prints "<line> <enclosing object type> <object type opened on the line>" for a
# qmlformat-style file, using - where there is none. Only a line holding just
# `Type {` opens an object; other braces (functions, handlers, JS blocks) are
# counted so that the closing brace of each object is found.
enclosing_objects() {
  awk '{
    enclosing = top > 0 ? type[top] : "-"; opened = "-"; pushed = 0
    if ($0 ~ /^[ \t]*[A-Z][A-Za-z0-9_.]*[ \t]*\{[ \t]*$/) { opened = $0; gsub(/[ \t{]/, "", opened) }
    for (i = 1; i <= length($0); i++) {
      c = substr($0, i, 1)
      if (c == "{") { depth++; if (opened != "-" && !pushed) { type[++top] = opened; at[top] = depth; pushed = 1 } }
      else if (c == "}") { if (top > 0 && at[top] == depth) top--; depth-- }
    }
    print NR, enclosing, opened
  }' "$1"
}
# Prints each missing-property warning that is not tied to a qs.Ui object.
untied_missing() {
  local file=$1 warnings=$2 owners line message enclosing opened name
  owners=$(enclosing_objects "$file")
  while IFS= read -r warning; do
    [[ -n $warning ]] || continue
    if (( strict_missing )) || ! [[ $warning =~ :([0-9]+):[0-9]+:\ (.*)\ \[missing-property\]$ ]]; then
      echo "$warning"; continue
    fi
    line=${BASH_REMATCH[1]} message=${BASH_REMATCH[2]}
    enclosing= opened=
    read -r _ enclosing opened < <(awk -v n="$line" '$1 == n' <<<"$owners") || true
    [[ $enclosing =~ ^($ui_types)$ ]] || { echo "$warning"; continue; }
    if [[ $message == 'Cannot assign to non-existent default property' ]]; then
      [[ $opened != - ]] || echo "$warning"
    elif [[ $message =~ ^Could\ not\ find\ property\ \"([A-Za-z]+)\"\.$ ]]; then
      name=${BASH_REMATCH[1]}
      [[ $name =~ ^(${ui_properties[$enclosing]})$ ]] || echo "$warning"
    else
      echo "$warning"
    fi
  done < <(grep -E '\[missing-property\]$' <<<"$warnings" || true)
}
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
      missing=$(untied_missing "$file" "$warnings")
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

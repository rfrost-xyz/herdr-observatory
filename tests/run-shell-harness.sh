#!/usr/bin/env bash
# Runs the Anton popover against the installed Omarchy shell modules
# (qs.Commons and qs.Ui) in Quickshell, offscreen. The QML suite in
# tests/qml/anton uses stub modules, so it cannot show how Panel.qml behaves
# with the real KeyboardPanel, PanelKeyCatcher, BarIconButton and Color; this
# harness does. See tests/shell/shell.qml for the checks.
#
# The offscreen platform has no layer-shell backend, so a scratch copy of the
# installed qs.Ui replaces KeyboardPanel's PanelWindow with a FloatingWindow and
# drops its layer-shell properties, mask and per-monitor dismissal windows. The
# card, content holder, key catcher and every other qs.Ui and qs.Commons file
# are the installed ones. Compositor focus and pointer delivery are therefore
# not exercised.
#
# Nothing outside a private temporary directory is read or written, apart from
# the installed shell modules, which are only read: HOME, XDG_* and the theme
# palette are synthetic, and anton-runtime is tests/shell/fake-runtime.mjs,
# which logs its arguments and launches nothing. The run fails if any
# --open-thread invocation is logged.
#
# It skips (exit 0) when quickshell or the Omarchy shell modules are absent,
# as in CI. OMARCHY_SHELL_DIR selects the shell directory; otherwise
# $OMARCHY_PATH/shell, /usr/share/omarchy/shell and
# ~/.local/share/omarchy/shell are tried in turn. ANTON_PLUGIN_SOURCE selects
# another plugin tree (default: omarchy/herdr.observatory).
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
quickshell=$(command -v quickshell || true)
node=$(command -v node || true)
shell_dir=${OMARCHY_SHELL_DIR:-}
if [[ -z $shell_dir ]]; then
  for candidate in "${OMARCHY_PATH:+$OMARCHY_PATH/shell}" /usr/share/omarchy/shell "$HOME/.local/share/omarchy/shell"; do
    [[ -n $candidate && -f $candidate/Ui/KeyboardPanel.qml && -f $candidate/Commons/qmldir ]] && { shell_dir=$candidate; break; }
  done
fi
if [[ -z $quickshell || -z $shell_dir || ! -f $shell_dir/Ui/KeyboardPanel.qml ]]; then
  echo 'skip: quickshell or the Omarchy shell modules are not installed'
  exit 0
fi
[[ -n $node ]] || { echo 'node is required for the fake runtime' >&2; exit 1; }
plugin_source=${ANTON_PLUGIN_SOURCE:-$root/omarchy/herdr.observatory}
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
mkdir -p -- "$scratch/config" "$scratch/plugin" "$scratch/home/.local/state/omarchy/current/theme" "$scratch/runtime"
chmod 700 -- "$scratch/runtime"

# The qs import root: the installed Commons, and a copy of Ui with the
# offscreen KeyboardPanel.
ln -s -- "$shell_dir/Commons" "$scratch/config/Commons"
cp -R -- "$shell_dir/Ui" "$scratch/config/Ui"
"$node" - "$scratch/config/Ui/KeyboardPanel.qml" <<'JS'
const fs = require('node:fs');
const file = process.argv[2];
let text = fs.readFileSync(file, 'utf8');
const original = text;
text = text.replace(/^import Quickshell\.Wayland\s*\n/m, '');
text = text.replace(/^\s*model: root\.open \? Quickshell\.screens : \[\]\s*$/m, '    model: []');
text = text.replace(/\bPanelWindow \{/g, 'FloatingWindow {');
text = text.replace(/^\s*WlrLayershell\.keyboardFocus:[^\n]*\n(?:[^\n]*\n)*?[^\n]*WlrKeyboardFocus\.None[^\n]*\n/gm, '');
text = text.replace(/^\s*WlrLayershell\.[A-Za-z]+:[^\n]*\n/gm, '');
text = text.replace(/^\s*exclusionMode: ExclusionMode\.Ignore\s*\n/gm, '');
text = text.replace(/^\s*mask: Region \{[^}]*\}\s*\n/m, '');
text = text.replace(/^(\s*)anchors \{\s*\n\s*top: true\s*\n\s*bottom: true\s*\n\s*left: true\s*\n\s*right: true\s*\n\s*\}\s*\n/gm, '$1implicitWidth: 1280\n$1implicitHeight: 800\n');
const code = text.split('\n').filter((line) => !/^\s*\/\//.test(line)).join('\n');
if (text === original || /PanelWindow|WlrLayershell|WlrKeyboardFocus|ExclusionMode|Region \{/.test(code)) {
  console.error('KeyboardPanel.qml no longer matches the offscreen patch; update tests/run-shell-harness.sh');
  process.exit(1);
}
fs.writeFileSync(file, text);
JS
cp -- "$root/tests/shell/shell.qml" "$scratch/config/shell.qml"

cp -- "$plugin_source"/*.qml "$plugin_source"/State.js "$scratch/plugin/"
cp -- "$root/tests/shell/fake-runtime.mjs" "$scratch/plugin/anton-runtime"
chmod +x -- "$scratch/plugin/anton-runtime"
# A synthetic theme palette (colors.toml keys the popover and shell read).
cat > "$scratch/home/.local/state/omarchy/current/theme/colors.toml" <<'TOML'
accent = "#d7af68"
background = "#191510"
foreground = "#d7d6cd"
blue = "#739fae"
cyan = "#7fb4a8"
green = "#9dc473"
red = "#ed7968"
yellow = "#d7af68"
TOML
log=$scratch/runtime-calls.log
: > "$log"

status=0
output=$(env -i PATH="$(dirname -- "$node"):/usr/bin:/bin" HOME="$scratch/home" \
  XDG_STATE_HOME="$scratch/home/.local/state" XDG_CONFIG_HOME="$scratch/home/.config" \
  XDG_CACHE_HOME="$scratch/home/.cache" XDG_DATA_HOME="$scratch/home/.local/share" \
  XDG_RUNTIME_DIR="$scratch/runtime" QT_QPA_PLATFORM=offscreen LANG=C.UTF-8 \
  ANTON_PLUGIN_DIR="$scratch/plugin" ANTON_FAKE_LOG="$log" \
  ANTON_FIXTURE="$root/tests/fixtures/popover-time-oracle.json" \
  timeout 120 "$quickshell" --no-color -p "$scratch/config/shell.qml" 2>&1) || status=$?
grep -E 'HARNESS|(TypeError|ReferenceError|Unable to assign).*(plugin/|shell\.qml)' <<<"$output" || true
calls=$(wc -l < "$log")
launches=$(grep -c -- '--open-thread' "$log" || true)
echo "runtime invocations: $calls, --open-thread: $launches"
grep -q 'HARNESS done' <<<"$output" || { echo 'the harness did not finish' >&2; (( status != 0 )) || status=1; }
errors=$(grep -E 'TypeError|ReferenceError|Unable to assign' <<<"$output" | grep -c -E 'plugin/|shell\.qml' || true)
if (( errors > 0 )); then
  echo "plugin or harness QML binding errors: $errors" >&2
  (( status != 0 )) || status=1
fi
if (( launches > 0 )); then
  echo 'a thread was opened without user input' >&2
  (( status != 0 )) || status=1
fi
(( calls > 0 )) || { echo 'the collector never started' >&2; (( status != 0 )) || status=1; }
exit "$status"

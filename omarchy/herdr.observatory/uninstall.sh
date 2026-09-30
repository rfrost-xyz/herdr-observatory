#!/usr/bin/env bash
set -euo pipefail

id=herdr.observatory
target=${XDG_CONFIG_HOME:-$HOME/.config}/omarchy/plugins/$id
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
[[ ! -L $target && $script_dir == "$target" ]] || { echo "Run the installed $target/uninstall.sh" >&2; exit 1; }
[[ -f $target/.herdr-observatory-install && ! -L $target/.herdr-observatory-install && -O $target/.herdr-observatory-install ]] || { echo 'Unsafe owner marker' >&2; exit 1; }
[[ $(stat -c %s -- "$target/.herdr-observatory-install") -le 80 ]] || { echo 'Oversized owner marker' >&2; exit 1; }
marker=$(cat "$target/.herdr-observatory-install" 2>/dev/null)
[[ $marker == "$id" || $marker == "$id:retired" ]] || { echo 'Installation marker missing' >&2; exit 1; }
if [[ -f $target/manifest.json ]]; then
  [[ $(jq -r .id "$target/manifest.json") == "$id" ]] || { echo 'Plugin id changed; refusing to remove it' >&2; exit 1; }
elif [[ $marker != "$id:retired" ]]; then
  echo 'Active installation manifest missing; refusing removal' >&2; exit 1
fi
for entry in "$target"/* "$target"/.[!.]* "$target"/..?*; do
  [[ -e $entry || -L $entry ]] || continue
  [[ -f $entry && ! -L $entry && -O $entry ]] || { echo 'Unsafe plugin payload; preserving installation' >&2; exit 1; }
  case ${entry##*/} in
    .hooks-receipt.json|.hooks-before-native.json|.peers.json|anton-runtime|.config.json|manifest.json|Panel.qml|PopupContent.qml|SectionHeader.qml|AntonText.qml|AntonSurface.qml|ThreadSignal.qml|SheenTitle.qml|BurnEffect.qml|MetricDial.qml|ThreadCard.qml|AllowanceCard.qml|AntonTheme.qml|AntonToolTip.qml|AntonPreferences.qml|AntonController.qml|.accounts.json|SnapshotStore.qml|State.js|README.md|uninstall.sh|.herdr-observatory-install) ;;
    *) echo "Unknown plugin file remains: $entry" >&2; exit 1 ;;
  esac
done

# Complete recorded peer removal before deleting the local receipt. An unreachable
# peer keeps this installation available for a safe retry.
if [[ -f $target/anton-runtime ]]; then
  "$target/anton-runtime" --remove-peers
  "$target/anton-runtime" --uninstall-hooks
elif [[ $marker != "$id:retired" ]]; then
  echo 'Active installation runtime missing; refusing removal' >&2; exit 1
fi
result=$(omarchy-shell shell setPluginEnabled "$id" false)
if [[ $result != ok ]]; then
  # A retry may follow payload removal and a shell rescan. An already absent or
  # disabled retired widget is safe to finish removing.
  [[ $marker == "$id:retired" ]] && omarchy-shell shell listPlugins | jq -e --arg id "$id" 'all(.[]; .id != $id or .enabled == false)' >/dev/null || { echo "Could not disable $id: $result" >&2; exit 1; }
fi
# Retire the native owner's checkpoint lease before deleting its runtime. The
# helper serialises any in-flight flush and prevents late shutdown/startup writes.
if [[ -f $target/anton-runtime ]]; then
  "$target/anton-runtime" --retire-checkpoints
fi
state_dir=${XDG_STATE_HOME:-$HOME/.local/state}/herdr.observatory
exec 9<> "$target/.herdr-observatory-install"
flock -x -w 3 9 || { echo 'Plugin owner still busy' >&2; exit 1; }
parent=$state_dir
while [[ $parent != / ]]; do
  [[ ! -L $parent ]] || { echo 'Unsafe plugin state path' >&2; exit 1; }
  parent=${parent%/*}
  [[ -n $parent ]] || parent=/
done
if [[ -e $state_dir ]]; then
  [[ -d $state_dir && -O $state_dir ]] || { echo 'Unsafe plugin state directory' >&2; exit 1; }
  owned=()
  for entry in "$state_dir"/* "$state_dir"/.[!.]* "$state_dir"/..?*; do
    [[ -e $entry || -L $entry ]] || continue
    name=${entry##*/}
    case $name in
      privacy.ini|allowances.json|allowances.json.lock|allowances-refresh.lock|hook.lock|sessions.json|replay-checkpoints.json|replay-checkpoints.lock) ;;
      .replay-checkpoints-*) [[ $name =~ ^\.replay-checkpoints-[0-9a-f]{16}$ ]] || continue ;;
      *) continue ;;
    esac
    [[ -f $entry && ! -L $entry && -O $entry ]] || { echo 'Unsafe plugin state file' >&2; exit 1; }
    owned+=("$entry")
  done
  ((${#owned[@]} == 0)) || rm -f -- "${owned[@]}"
  rmdir --ignore-fail-on-non-empty -- "$state_dir"
fi
for file in Panel.qml PopupContent.qml SectionHeader.qml AntonText.qml AntonSurface.qml ThreadSignal.qml SheenTitle.qml BurnEffect.qml MetricDial.qml ThreadCard.qml AllowanceCard.qml AntonTheme.qml AntonToolTip.qml AntonPreferences.qml AntonController.qml SnapshotStore.qml State.js README.md; do
  rm -f -- "$target/$file"
done
rm -f -- "$target/.accounts.json" "$target/.config.json" "$target/anton-runtime" "$target/.hooks-receipt.json" "$target/.hooks-before-native.json" "$target/.peers.json" "$target/manifest.json"
rm -f -- "$target/.herdr-observatory-install" "$target/uninstall.sh"
rmdir -- "$target"
omarchy-shell shell rescanPlugins >/dev/null
echo "Removed $id and its bar entry"

#!/usr/bin/env bash
set -euo pipefail
umask 077

id=herdr.observatory
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
plugins_dir=${XDG_CONFIG_HOME:-$HOME/.config}/omarchy/plugins
target=$plugins_dir/$id
stage=$plugins_dir/.$id.install.$$
files=(manifest.json Panel.qml PopupContent.qml SectionHeader.qml AntonText.qml AntonSurface.qml ThreadSignal.qml SheenTitle.qml BurnEffect.qml MetricDial.qml ThreadCard.qml AllowanceCard.qml AntonTheme.qml AntonToolTip.qml SnapshotStore.qml State.js README.md uninstall.sh)

[[ -n ${ANTON_CONFIG:-} && -f $ANTON_CONFIG && ! -L $ANTON_CONFIG ]] || { echo "Set ANTON_CONFIG to a regular private config file" >&2; exit 1; }
[[ ! -e $target && ! -L $target ]] || { echo "Plugin already installed: $target" >&2; exit 1; }
[[ $(jq -r .id "$source_dir/manifest.json") == "$id" ]] || { echo 'Unexpected plugin id' >&2; exit 1; }
omarchy-plugin-validate "$source_dir"
omarchy-shell shell listPlugins >/dev/null || { echo 'Omarchy shell must be running to enable the plugin' >&2; exit 1; }
mkdir -p -- "$plugins_dir"
mkdir -- "$stage"
trap 'rm -rf -- "$stage"' EXIT
for file in "${files[@]}"; do
  [[ -f $source_dir/$file && ! -L $source_dir/$file ]] || { echo "Missing regular file: $file" >&2; exit 1; }
  cp -- "$source_dir/$file" "$stage/$file"
done
printf '%s\n' "$id" > "$stage/.herdr-observatory-install"
chmod 755 "$stage/uninstall.sh"
"$source_dir/build-native.sh" "$stage/anton-runtime"
if [[ -n ${ANTON_CONFIG:-} ]]; then
  "$stage/anton-runtime" --migrate-config < "$ANTON_CONFIG" > "$stage/.config.json"
  chmod 600 "$stage/.config.json"
fi
omarchy-plugin-validate "$stage"
mv -- "$stage" "$target"
trap - EXIT

if ! omarchy-shell shell rescanPlugins >/dev/null || ! omarchy plugin enable "$id"; then
  echo "Plugin copied but could not be enabled. Start Omarchy, then run: omarchy plugin enable $id" >&2
  exit 1
fi
if [[ -f $target/.config.json ]]; then
  "$target/anton-runtime" --install-hooks
fi
omarchy restart shell
omarchy-shell shell ping >/dev/null
omarchy-shell shell listPlugins | jq -e --arg id "$id" 'any(.[]; .id == $id and .enabled == true)' >/dev/null
echo "Installed $id at $target"
echo "Remove later with: $target/uninstall.sh"

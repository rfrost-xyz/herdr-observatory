#!/usr/bin/env bash
set -euo pipefail

id=herdr.observatory
target=${XDG_CONFIG_HOME:-$HOME/.config}/omarchy/plugins/$id
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
[[ ! -L $target && $script_dir == "$target" ]] || { echo "Run the installed $target/uninstall.sh" >&2; exit 1; }
[[ $(cat "$target/.herdr-observatory-install" 2>/dev/null) == "$id" ]] || { echo 'Installation marker missing' >&2; exit 1; }
[[ $(jq -r .id "$target/manifest.json") == "$id" ]] || { echo 'Plugin id changed; refusing to remove it' >&2; exit 1; }
for entry in "$target"/* "$target"/.[!.]* "$target"/..?*; do
  [[ -e $entry || -L $entry ]] || continue
  case ${entry##*/} in
    manifest.json|Panel.qml|Companion.qml|SnapshotStore.qml|State.js|README.md|uninstall.sh|.herdr-observatory-install) ;;
    *) echo "Unknown plugin file remains: $entry" >&2; exit 1 ;;
  esac
done

result=$(omarchy-shell shell setPluginEnabled "$id" false)
[[ $result == ok ]] || { echo "Could not disable $id: $result" >&2; exit 1; }
for file in manifest.json Panel.qml Companion.qml SnapshotStore.qml State.js README.md uninstall.sh .herdr-observatory-install; do
  rm -- "$target/$file"
done
rmdir -- "$target"
omarchy-shell shell rescanPlugins >/dev/null
echo "Removed $id and its bar entry"

#!/usr/bin/env bash
set -euo pipefail
root=$HOME/.local/share/herdr.observatory-peer
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
[[ $here == "$root" && ! -L $root ]] || { echo 'Run the installed native peer uninstaller' >&2; exit 1; }
[[ -f $root/.herdr-observatory-install && ! -L $root/.herdr-observatory-install && -O $root/.herdr-observatory-install ]] || { echo 'Unsafe owner marker' >&2; exit 1; }
[[ $(stat -c %s -- "$root/.herdr-observatory-install") -le 80 ]] || { echo 'Oversized owner marker' >&2; exit 1; }
marker=$(cat "$root/.herdr-observatory-install" 2>/dev/null)
[[ $marker == herdr.observatory || $marker == herdr.observatory:retired ]] || { echo 'Peer ownership marker missing' >&2; exit 1; }
for entry in "$root"/* "$root"/.[!.]* "$root"/..?*; do
  [[ -e $entry || -L $entry ]] || continue
  [[ -f $entry && ! -L $entry && -O $entry ]] || { echo 'Unsafe peer payload; preserving it' >&2; exit 1; }
  case ${entry##*/} in
    anton-runtime|uninstall.sh|.config.json|.herdr-observatory-install|.peer-receipt.json|.hooks-receipt.json|.hooks-before-native.json) ;;
    *) echo 'Unknown peer payload; preserving installation' >&2; exit 1 ;;
  esac
done
if [[ $marker == herdr.observatory ]]; then
  [[ -x $root/anton-runtime ]] || { echo 'Peer runtime missing' >&2; exit 1; }
  exec "$root/anton-runtime" --uninstall-peer
fi
# Retirement prevents new writers. Retry also finishes state deletion if the
# first native removal was interrupted between retirement and payload removal.
exec 9<> "$root/.herdr-observatory-install"
flock -x -w 3 9 || { echo 'Peer owner still busy' >&2; exit 1; }
state=$HOME/.local/state/herdr.observatory-peer
parent=$state
while [[ $parent != / ]]; do
  [[ ! -L $parent ]] || { echo 'Unsafe peer state path' >&2; exit 1; }
  parent=${parent%/*}
  [[ -n $parent ]] || parent=/
done
if [[ -e $state ]]; then
  [[ -d $state && -O $state ]] || { echo 'Unsafe peer state directory' >&2; exit 1; }
  owned=()
  for entry in "$state"/* "$state"/.[!.]* "$state"/..?*; do
    [[ -e $entry || -L $entry ]] || continue
    name=${entry##*/}
    case $name in
      allowances.json|allowances.json.lock|allowances-refresh.lock|hook.lock|sessions.json|replay-checkpoints.json|replay-checkpoints.lock) ;;
      .replay-checkpoints-*) [[ $name =~ ^\.replay-checkpoints-[0-9a-f]{16}$ ]] || continue ;;
      *) continue ;;
    esac
    [[ -f $entry && ! -L $entry && -O $entry ]] || { echo 'Unsafe peer state file' >&2; exit 1; }
    owned+=("$entry")
  done
  ((${#owned[@]} == 0)) || rm -f -- "${owned[@]}"
  rmdir --ignore-fail-on-non-empty -- "$state"
fi
for name in .config.json .hooks-receipt.json .hooks-before-native.json anton-runtime .peer-receipt.json; do
  rm -f -- "$root/$name"
done
rm -f -- "$root/.herdr-observatory-install" "$root/uninstall.sh"
rmdir -- "$root"

#!/usr/bin/env python3
"""Install only Observatory-owned adapters from a local running container."""
import argparse
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import tempfile

MARKER = 'herdr-observatory adapter v1'
EVENTS = ('SessionStart', 'UserPromptSubmit', 'PreToolUse', 'PostToolUse', 'PreCompact', 'PostCompact', 'Stop', 'Interrupt', 'SessionEnd')


def atomic(path, data, mode=0o600):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp = tempfile.mkstemp(prefix='.observatory-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            stream.write(data); stream.flush(); os.fsync(stream.fileno())
        os.chmod(temp, mode)
        os.replace(temp, path)
    finally:
        if os.path.exists(temp): os.unlink(temp)


def merge_hooks(config, command, uninstall=False):
    # JSON round trip keeps caller's object unchanged on a validation failure.
    result = json.loads(json.dumps(config))
    if not isinstance(result, dict) or not isinstance(result.get('hooks', {}), dict):
        raise ValueError('Invalid Codex hook configuration')
    hooks = result.setdefault('hooks', {})
    for event in EVENTS:
        entries = hooks.get(event, [])
        if not isinstance(entries, list): raise ValueError('Invalid hook event entries')
        kept = []
        for entry in entries:
            if not isinstance(entry, dict) or not isinstance(entry.get('hooks'), list):
                raise ValueError('Invalid hook entry')
            remaining = [h for h in entry['hooks'] if not (isinstance(h, dict) and h.get('type') == 'command' and h.get('command') == command)]
            if remaining or not entry['hooks']:
                kept.append({**entry, 'hooks': remaining})
        if not uninstall:
            kept.append({'hooks': [{'type': 'command', 'command': command, 'timeout': 3}]})
        if kept: hooks[event] = kept
        else: hooks.pop(event, None)
    return result


def install(home, container, uninstall=False):
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.-]{0,80}', container):
        raise ValueError('Invalid container name')
    root = home / '.local/share/herdr-observatory/hooks'
    shell = root / 'codex.sh'
    extension = home / '.pi/agent/extensions/observatory.ts'
    config = home / '.codex/hooks.json'
    paths = (shell, extension, config)
    chezmoi = shutil.which('chezmoi')
    for path in paths:
        if path.is_symlink(): raise ValueError(f'Refusing symlink: {path}')
        if chezmoi and subprocess.run([chezmoi, 'source-path', str(path)], capture_output=True).returncode == 0:
            raise ValueError(f'Edit managed configuration through chezmoi: {path}')
    for path in (shell, extension):
        if path.exists() and MARKER not in '\n'.join(path.read_text().splitlines()[:2]):
            raise ValueError(f'Conflicting adapter: {path}')
    existing = json.loads(config.read_text()) if config.exists() else {}
    command = 'sh ' + shlex.quote(str(shell))
    updated = merge_hooks(existing, command, uninstall)
    payloads = {}
    if not uninstall:
        for name in ('codex.sh', 'observatory.ts'):
            payload = subprocess.check_output(['docker', 'exec', container, 'cat', '/app/hooks/' + name], timeout=5).decode()
            if MARKER not in payload[:160]: raise ValueError('Unrecognised image payload')
            payloads[name] = payload.replace('__CONTAINER__', container)
    if config.exists() and existing != updated:
        # One private rollback copy, never timestamps accumulating across updates.
        backup = root / 'hooks.before-install.json'
        if not backup.exists(): atomic(backup, json.dumps(existing, indent=2) + '\n')
    if not uninstall:
        atomic(shell, payloads['codex.sh'], 0o700)
        atomic(extension, payloads['observatory.ts'])
    if existing != updated:
        atomic(config, json.dumps(updated, indent=2) + '\n')
    if uninstall:
        for path in (shell, extension): path.unlink(missing_ok=True)
    return paths


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--container', default='herdr-observatory')
    parser.add_argument('--uninstall', action='store_true')
    args = parser.parse_args()
    try:
        paths = install(Path.home(), args.container, args.uninstall)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, str(error) + '\n')
    print('Removed' if args.uninstall else 'Installed', 'Observatory adapters; existing sessions need restart/reload.')
    for path in paths: print(path)


if __name__ == '__main__':
    main()

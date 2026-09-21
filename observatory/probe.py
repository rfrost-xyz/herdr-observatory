"""One read-only sample, also executable over SSH stdin without installation."""
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
import tomllib


def palette(directory=None):
    bases = (Path(directory),) if directory else (Path.home() / '.local/state/omarchy/current', Path.home() / '.config/omarchy/current')
    for base in bases:
        try:
            colours = tomllib.loads((base / 'theme/colors.toml').read_text())
            return {'name': (base / 'theme.name').read_text().strip(), 'colours': colours}
        except (OSError, ValueError):
            continue
    return None


def metrics(disk_path=None):
    result = {'scope': 'Container / Linux kernel' if Path('/.dockerenv').exists() else 'Linux / WSL kernel',
              'at': time.time(), 'cpu': None, 'memory': None, 'disk': None, 'network': None, 'gpu': None}
    try:
        counters = list(map(int, Path('/proc/stat').read_text().splitlines()[0].split()[1:9]))
        result['cpu'] = {'total': sum(counters), 'idle': counters[3] + counters[4]}
        mem = {line.split(':')[0]: int(line.split()[1]) * 1024 for line in Path('/proc/meminfo').read_text().splitlines()}
        result['memory'] = {'used': mem['MemTotal'] - mem['MemAvailable'], 'total': mem['MemTotal']}
        usage = shutil.disk_usage(disk_path or Path.home())
        result['disk'] = {'used': usage.used, 'total': usage.total}
        rx = tx = 0
        for line in Path('/proc/net/dev').read_text().splitlines()[2:]:
            iface, values = line.split(':', 1)
            if iface.strip() != 'lo':
                fields = values.split()
                rx += int(fields[0]); tx += int(fields[8])
        result['network'] = {'rx': rx, 'tx': tx}
    except (OSError, ValueError, KeyError, IndexError):
        pass
    gpu_bin = shutil.which('nvidia-smi')
    if gpu_bin:
        try:
            output = subprocess.run([gpu_bin, '--query-gpu=utilization.gpu,memory.used,memory.total', '--format=csv,noheader,nounits'], capture_output=True, text=True, timeout=2, check=True)
            rows = [list(map(float, line.split(','))) for line in output.stdout.splitlines()]
            if rows and all(len(row) == 3 for row in rows):
                result['gpu'] = {'percent': sum(r[0] for r in rows) / len(rows), 'used': sum(r[1] for r in rows) * 1048576, 'total': sum(r[2] for r in rows) * 1048576}
        except (OSError, ValueError, subprocess.SubprocessError):
            pass
    return result


def socket_snapshot(path):
    request_id = 'observatory-snapshot'
    limit = 4 * 1024 * 1024
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(6)
        client.connect(str(Path(path).expanduser()))
        client.sendall(json.dumps({'id': request_id, 'method': 'session.snapshot', 'params': {}}).encode() + b'\n')
        deadline = time.monotonic() + 6
        data = bytearray()
        while b'\n' not in data:
            client.settimeout(max(0.001, deadline - time.monotonic()))
            chunk = client.recv(min(65536, limit + 1 - len(data)))
            if not chunk:
                raise ValueError('Incomplete snapshot frame')
            data.extend(chunk)
            if len(data) > limit or time.monotonic() >= deadline:
                raise ValueError('Snapshot response exceeds limits')
        response = json.loads(data.split(b'\n', 1)[0])
        if not isinstance(response, dict) or response.get('id') != request_id:
            raise ValueError('Mismatched snapshot response')
        return response['result']['snapshot']


def sample(binary='herdr', session=None, socket_path=None, theme_path=None, disk_path=None):
    resolved = shutil.which(binary) or str(Path(binary).expanduser())
    if binary == 'herdr' and not shutil.which(binary):
        resolved = str(Path.home() / '.local/bin/herdr')
    command = [resolved] + (['--session', session] if session else []) + ['api', 'snapshot']
    result = {'metrics': metrics(disk_path) if disk_path else metrics(), 'theme': palette(theme_path) if theme_path else palette(), 'snapshot': None, 'error': None}
    try:
        if socket_path:
            raw = socket_snapshot(socket_path)
        else:
            process = subprocess.run(command, capture_output=True, text=True, timeout=6, check=True)
            raw = json.loads(process.stdout)['result']['snapshot']
        if not isinstance(raw.get('agents'), list) or not isinstance(raw.get('workspaces'), list):
            raise ValueError('Invalid snapshot')
        # Do not export terminal buffers, process arguments or native session IDs.
        result['snapshot'] = {'version': raw.get('version', 'unknown'), 'protocol': raw.get('protocol'),
            'agents': [{k: a.get(k) for k in ('pane_id', 'workspace_id', 'agent', 'agent_status', 'cwd', 'terminal_title_stripped', 'revision', 'state_change_seq', 'focused', 'interactive_ready', 'launch_pending')} for a in raw['agents']],
            'workspaces': [{k: w.get(k) for k in ('workspace_id', 'label')} for w in raw['workspaces']]}
    except (OSError, ValueError, KeyError, TypeError, AttributeError, subprocess.SubprocessError):
        result['error'] = 'Herdr unavailable or incompatible'
    return result


if __name__ == '__main__':
    print(json.dumps(sample()))

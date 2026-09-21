"""One read-only sample, also executable over SSH stdin without installation."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import tomllib


def palette():
    for base in (Path.home() / '.local/state/omarchy/current', Path.home() / '.config/omarchy/current'):
        try:
            colours = tomllib.loads((base / 'theme/colors.toml').read_text())
            return {'name': (base / 'theme.name').read_text().strip(), 'colours': colours}
        except (OSError, ValueError):
            continue
    return None


def metrics():
    result = {'scope': 'Container / Linux kernel' if Path('/.dockerenv').exists() else 'Linux / WSL kernel',
              'at': time.time(), 'cpu': None, 'memory': None, 'disk': None, 'network': None, 'gpu': None}
    try:
        counters = list(map(int, Path('/proc/stat').read_text().splitlines()[0].split()[1:9]))
        result['cpu'] = {'total': sum(counters), 'idle': counters[3] + counters[4]}
        mem = {line.split(':')[0]: int(line.split()[1]) * 1024 for line in Path('/proc/meminfo').read_text().splitlines()}
        result['memory'] = {'used': mem['MemTotal'] - mem['MemAvailable'], 'total': mem['MemTotal']}
        usage = shutil.disk_usage(Path.home())
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
            if rows:
                result['gpu'] = {'percent': sum(r[0] for r in rows) / len(rows), 'used': sum(r[1] for r in rows) * 1048576, 'total': sum(r[2] for r in rows) * 1048576}
        except (OSError, ValueError, subprocess.SubprocessError):
            pass
    return result


def sample(binary='herdr', session=None):
    resolved = shutil.which(binary) or str(Path(binary).expanduser())
    if binary == 'herdr' and not shutil.which(binary):
        resolved = str(Path.home() / '.local/bin/herdr')
    command = [resolved] + (['--session', session] if session else []) + ['api', 'snapshot']
    result = {'metrics': metrics(), 'theme': palette(), 'snapshot': None, 'error': None}
    try:
        process = subprocess.run(command, capture_output=True, text=True, timeout=6, check=True)
        raw = json.loads(process.stdout)['result']['snapshot']
        if not isinstance(raw.get('agents'), list) or not isinstance(raw.get('workspaces'), list):
            raise ValueError('Invalid snapshot')
        # Do not export terminal buffers, process arguments or native session IDs.
        result['snapshot'] = {'version': raw.get('version', 'unknown'),
            'agents': [{k: a.get(k) for k in ('pane_id', 'workspace_id', 'agent', 'agent_status', 'cwd', 'terminal_title_stripped')} for a in raw['agents']],
            'workspaces': [{k: w.get(k) for k in ('workspace_id', 'label')} for w in raw['workspaces']]}
    except (OSError, ValueError, KeyError, TypeError, AttributeError, subprocess.SubprocessError):
        result['error'] = 'Herdr unavailable or incompatible'
    return result


if __name__ == '__main__':
    print(json.dumps(sample()))

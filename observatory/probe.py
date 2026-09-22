"""One read-only sample, also executable over SSH stdin without installation."""
import json
import hashlib
import re
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
import tomllib


# This code also travels with the read-only SSH probe: no package imports here.
TELEMETRY_EVENTS = {'session', 'turn', 'tool-start', 'tool-end', 'thinking', 'output',
                    'compact-start', 'compact-end', 'compact-failed', 'idle', 'interrupt', 'end', 'model', 'subagent-start', 'subagent-stop'}
TELEMETRY_PHASES = {'ready', 'working', 'tool', 'thinking', 'output', 'compacting', 'idle', 'interrupted', 'ended'}
TELEMETRY_NUMBERS = ('input', 'output_tokens', 'cache_read', 'cache_write', 'context', 'window', 'usage_seq', 'total_input', 'total_output', 'total_cache_read', 'total_cache_write', 'total_uncached_input', 'compactions', 'context_percent')
# Wire v2 order is immutable. A new field/order requires a new wire version.
TELEMETRY_V2_GROUPS = (
    ('input', 'output_tokens', 'cache_read', 'cache_write'),
    ('context', 'window', 'usage_seq', 'total_input'),
    ('total_output', 'total_cache_read', 'total_cache_write', 'total_uncached_input'),
    ('compactions', 'context_percent'),
)


def session_binding(agent):
    ref = agent.get('agent_session')
    if not isinstance(ref, dict) or ref.get('agent') != agent.get('agent') or ref.get('source') != 'herdr:' + str(agent.get('agent')):
        return None
    if ref.get('kind') not in ('id', 'path') or not isinstance(ref.get('value'), str) or not ref['value']:
        return None
    return hashlib.sha256((str(agent.get('agent')) + ':' + ref['kind'] + ':' + ref['value']).encode()).hexdigest()


def telemetry_view(raw, now=None):
    """Second disclosure boundary, also used for already-normalised Work feeds."""
    if not isinstance(raw, dict):
        return None
    now = time.time() if now is None else now
    seq = raw.get('seq')
    if type(seq) is not int or not 0 <= seq <= 9007199254740991 or now - seq / 1_000_000 < 0:
        return None
    if raw.get('event') not in TELEMETRY_EVENTS or raw.get('phase') not in TELEMETRY_PHASES:
        return None
    result = {k: raw[k] for k in ('seq', 'event', 'phase')}
    tool = raw.get('tool')
    known = {'Bash', 'bash', 'apply_patch', 'read', 'write', 'edit', 'grep', 'find', 'ls', 'exec_command', 'write_stdin', 'exec', 'web', 'mcp-tool', 'custom-tool'}
    result['tool'] = (tool if tool in known else ('mcp-tool' if tool.startswith('mcp') else 'custom-tool')) if isinstance(tool, str) and tool else None
    model = raw.get('model')
    result['model'] = model if isinstance(model, str) and len(model) <= 64 and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.:-]*(/[A-Za-z0-9][A-Za-z0-9_.:-]*)?', model) and '..' not in model and not re.match(r'^[A-Za-z]:/', model) else None
    result['result'] = raw.get('result') if raw.get('result') in ('finished', 'error', 'cancelled') else None
    for key in TELEMETRY_NUMBERS:
        value = raw.get(key)
        result[key] = value if type(value) is int and 0 <= value <= 9007199254740991 else None
    result['usage_source'] = raw.get('usage_source') if raw.get('usage_source') in ('codex-rollout','pi-extension') else None
    supplied_usage_time = raw.get('usage_seq') is not None or raw.get('usage_source') is not None
    if supplied_usage_time and (result['usage_seq'] is None or now - result['usage_seq'] / 1_000_000 < 0):
        for key in TELEMETRY_NUMBERS: result[key] = None
        result['usage_source'] = None
    if result['context_percent'] is not None and result['context_percent'] > 100: result['context_percent'] = None
    if result['total_input'] is not None and result['total_cache_read'] is not None and result['total_cache_read'] > result['total_input']:
        result['total_input'] = result['total_cache_read'] = result['total_uncached_input'] = None
    # Cross-field consistency: invalid estimates are unavailable, not clamped.
    if result['window'] == 0 or (result['context'] is not None and result['window'] is not None and result['context'] > result['window']):
        result['context'] = result['window'] = result['context_percent'] = None
    return result


def telemetry_from_agent(agent, now=None):
    tokens = agent.get('tokens')
    if not isinstance(tokens, dict) or tokens.get('obs_v') not in ('1', '2') or not session_binding(agent) or tokens.get('obs_bind') != session_binding(agent):
        return None
    raw = {k: tokens.get('obs_' + k) for k in ('seq', 'event', 'phase', 'tool', 'model', 'result', 'usage_source')}
    value = raw['seq']
    raw['seq'] = int(value) if isinstance(value, str) and re.fullmatch(r'[0-9]{1,16}', value) else None
    if tokens['obs_v'] == '2':
        # A partial or malformed atomic record never falls back to retained v1 keys.
        for index, fields in enumerate(TELEMETRY_V2_GROUPS):
            packed = tokens.get('obs_n' + str(index))
            if not isinstance(packed, str) or len(packed)>67:
                return None
            values = packed.split(',')
            if len(values) != len(fields):
                return None
            for key, value in zip(fields, values):
                if value == '':
                    raw[key] = None
                elif re.fullmatch(r'[0-9]{1,16}', value) and int(value)<=9007199254740991:
                    raw[key] = int(value)
                else:
                    return None
    else:
        for key in TELEMETRY_NUMBERS:
            value = tokens.get('obs_' + key)
            raw[key] = int(value) if isinstance(value, str) and re.fullmatch(r'[0-9]{1,16}', value) else None
    return telemetry_view(raw, now)


def checkout_label(value):
    """Export one directory leaf, never a full native checkout path."""
    if not isinstance(value, str) or not value.startswith('/') or '..' in value.split('/'):
        return ''
    return re.sub(r'[\x00-\x1f\x7f-\x9f]', '', value.rstrip('/').rsplit('/', 1)[-1])[:80]


def workspace_view(workspace):
    worktree = workspace.get('worktree')
    path = worktree.get('checkout_path') if isinstance(worktree, dict) else None
    return {k: workspace.get(k) for k in ('workspace_id', 'label')} | {'checkout_path': path if isinstance(path, str) else None}


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
            'agents': [{k: a.get(k) for k in ('pane_id', 'workspace_id', 'agent', 'agent_status', 'cwd', 'terminal_title_stripped', 'revision', 'state_change_seq', 'focused', 'interactive_ready', 'launch_pending')} | {'telemetry': telemetry_from_agent(a)} for a in raw['agents']],
            'workspaces': [workspace_view(w) for w in raw['workspaces']]}
    except (OSError, ValueError, KeyError, TypeError, AttributeError, subprocess.SubprocessError):
        result['error'] = 'Herdr unavailable or incompatible'
    return result


if __name__ == '__main__':
    print(json.dumps(sample()))

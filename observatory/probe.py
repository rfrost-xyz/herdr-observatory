"""One read-only sample, also executable over SSH stdin without installation."""
import json
import hashlib
import math
import re
import os
import signal
from pathlib import Path
import shutil
import socket
import subprocess
import time
import tomllib
import urllib.request
import urllib.error


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
    children = [raw.get(key) for key in ('subagent_starts', 'subagent_stops')]
    child_time = raw.get('subagent_seq')
    valid_children = all(type(value) is int and 0 <= value <= 999 for value in children)
    valid_child_time = child_time is None or (type(child_time) is int and 0 <= child_time <= seq)
    if not valid_children or not valid_child_time or (any(children) and child_time is None):
        children, child_time = [None, None], None
    result['subagent_starts'], result['subagent_stops'] = children
    result['subagent_seq'] = child_time
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
        children = tokens.get('obs_children')
        match = re.fullmatch(r'([0-9]{1,3}),([0-9]{1,3}),([0-9]{0,16})', children) if isinstance(children, str) else None
        if match:
            raw['subagent_starts'] = int(match[1])
            raw['subagent_stops'] = int(match[2])
            raw['subagent_seq'] = int(match[3]) if match[3] else None
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


def xe_gpu(path='/gpu/metrics.json', now=None):
    """Consume only a fresh, aggregate sample from the isolated XE monitor."""
    now = time.time() if now is None else now
    try:
        source = Path(path)
        if source.stat().st_size > 512:
            return None
        sample = json.loads(source.read_text())
        at, percent = sample['at'], sample['percent']
        if (type(at) not in (int, float) or abs(at) > 1e12 or not math.isfinite(at) or
                type(percent) not in (int, float) or not 0 <= percent <= 100 or not math.isfinite(percent) or
                not 0 <= now - at <= 10 or
                sample.get('source') != 'intel-xe-pmu'):
            return None
        return {'percent': percent, 'source': 'intel-xe-pmu'}
    except (OSError, ValueError, KeyError, TypeError, OverflowError):
        return None


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, new_url):
        return None


def dashboard_gpu(port, host_id, now=None):
    """Read only the named host's current GPU from its loopback dashboard."""
    now = time.time() if now is None else now
    if type(port) is not int or not 1 <= port <= 65535 or not isinstance(host_id, str) or not host_id:
        return None
    try:
        # The SSH probe is a short-lived main-thread process. A socket timeout
        # alone does not bound a peer that trickles one byte at a time.
        prior_handler = signal.getsignal(signal.SIGALRM)
        prior_timer = signal.getitimer(signal.ITIMER_REAL)
        started = time.monotonic()
        def deadline(_signum, _frame):
            raise TimeoutError('Loopback GPU read deadline')
        signal.signal(signal.SIGALRM, deadline)
        signal.setitimer(signal.ITIMER_REAL, 2)
        try:
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
            with opener.open(f'http://127.0.0.1:{port}/api/state', timeout=2) as response:
                raw = response.read(1048577)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, prior_handler)
            if prior_timer[0] > 0:
                signal.setitimer(signal.ITIMER_REAL, max(0.000001, prior_timer[0] - (time.monotonic() - started)), prior_timer[1])
        if len(raw) > 1048576:
            return None
        for host in json.loads(raw).get('hosts', []):
            if not isinstance(host, dict) or host.get('id') != host_id or host.get('online') is not True:
                continue
            metrics = host.get('metrics') or {}
            sampled_at = host.get('sampled_at')
            gpu = metrics.get('gpu')
            if (type(sampled_at) not in (int, float) or abs(sampled_at) > 1e12 or
                    not math.isfinite(sampled_at) or not 0 <= now - sampled_at <= 20 or
                    not isinstance(gpu, dict) or gpu.get('source') != 'nvidia-visible'):
                return None
            percent, used, total = (gpu.get(key) for key in ('percent', 'used', 'total'))
            if any(type(value) not in (int, float) or abs(value) > 1e15 or not math.isfinite(value)
                   for value in (percent, used, total)) or not 0 <= percent <= 100 or not 0 <= used <= total or total <= 0:
                return None
            return {'percent': percent, 'used': used, 'total': total, 'source': 'nvidia-visible'}
    except urllib.error.HTTPError as error:
        error.close()
    except (OSError, ValueError, TypeError, AttributeError, OverflowError, RecursionError):
        pass
    return None


def metrics(disk_path=None, gpu_state_port=None, gpu_host_id=None):
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
            if rows and all(len(row) == 3 and all(math.isfinite(value) for value in row) and 0 <= row[0] <= 100 and 0 <= row[1] <= row[2] and row[2] > 0 for row in rows):
                result['gpu'] = {'percent': sum(r[0] for r in rows) / len(rows), 'used': sum(r[1] for r in rows) * 1048576, 'total': sum(r[2] for r in rows) * 1048576, 'source': 'nvidia-visible'}
        except (OSError, ValueError, subprocess.SubprocessError):
            pass
    if result['gpu'] is None:
        result['gpu'] = xe_gpu()
    if result['gpu'] is None and gpu_state_port is not None:
        result['gpu'] = dashboard_gpu(gpu_state_port, gpu_host_id)
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


def sample(binary='herdr', session=None, socket_path=None, theme_path=None, disk_path=None, gpu_state_port=None, gpu_host_id=None):
    resolved = shutil.which(binary) or str(Path(binary).expanduser())
    if binary == 'herdr' and not shutil.which(binary):
        resolved = str(Path.home() / '.local/bin/herdr')
    command = [resolved] + (['--session', session] if session else []) + ['api', 'snapshot']
    result = {'metrics': metrics(disk_path, gpu_state_port, gpu_host_id) if disk_path or gpu_state_port is not None else metrics(), 'theme': palette(theme_path) if theme_path else palette(), 'snapshot': None, 'error': None}
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

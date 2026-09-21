"""Collection, disclosure and bounded observation history."""
from concurrent.futures import ThreadPoolExecutor
import copy
import json
import math
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import threading
import time

from . import probe

FALLBACK = {'background': '#1a1b26', 'foreground': '#c0caf5', 'accent': '#7aa2f7', 'green': '#9ece6a', 'red': '#f7768e', 'yellow': '#e0af68', 'muted': '#565f89', 'lighter_background': '#24283b'}
STATUSES = {'working', 'blocked', 'done', 'idle', 'unknown'}


def clean(value, fallback=''):
    return re.sub(r'[\x00-\x1f\x7f]', '', value)[:160] if isinstance(value, str) else fallback


def theme(raw):
    colours = dict(FALLBACK)
    if isinstance(raw, dict) and isinstance(raw.get('colours'), dict):
        for key in colours:
            value = raw['colours'].get(key)
            if isinstance(value, str) and re.fullmatch(r'#[0-9a-fA-F]{6}', value):
                colours[key] = value
        return {'name': clean(raw.get('name'), 'Omarchy'), 'colours': colours}
    return {'name': 'Tokyo Night · fallback', 'colours': colours}


def validate_config(config):
    hosts = config.get('hosts')
    if not isinstance(hosts, list) or not 1 <= len(hosts) <= 16:
        raise ValueError('Configure between 1 and 16 hosts')
    ids = set()
    for host in hosts:
        identifier = host.get('id', '')
        if not isinstance(identifier, str) or not re.fullmatch(r'[a-zA-Z0-9_-]{1,40}', identifier) or identifier in ids:
            raise ValueError('Host IDs must be unique letters, digits, underscores or hyphens')
        ids.add(identifier)
        if host.get('transport', 'local') not in ('local', 'ssh'):
            raise ValueError('Transport must be local or ssh')
        if host.get('transport') == 'ssh':
            target = host.get('target', '')
            if not isinstance(target, str) or not re.fullmatch(r'[a-zA-Z0-9_.@:-]+', target) or target.startswith('-'):
                raise ValueError('SSH target must be an SSH alias or user@host')
        for key in ('work_roots', 'personal_roots'):
            roots = host.get(key, [])
            if not isinstance(roots, list) or any(not isinstance(r, str) or not r.startswith('/') or '..' in PurePosixPath(r).parts for r in roots):
                raise ValueError('Project roots must be absolute POSIX paths without parent traversal')
        for key in ('herdr', 'session'):
            if key in host and (not isinstance(host[key], str) or not host[key]):
                raise ValueError(f'{key} must be a nonempty string')
    interval = config.get('interval', 5)
    if isinstance(interval, bool) or not isinstance(interval, (int, float)) or not 2 <= interval <= 60:
        raise ValueError('Interval must be between 2 and 60 seconds')
    if config.get('theme_host', hosts[0]['id']) not in ids:
        raise ValueError('Theme host must name a configured host')
    return config


def classification(cwd, host):
    if not isinstance(cwd, str) or not cwd.startswith('/') or '..' in PurePosixPath(cwd).parts:
        return 'personal'
    path = PurePosixPath(cwd)
    # Personal exclusions take precedence, even inside a work root.
    for category in ('personal', 'work'):
        if any(path.is_relative_to(PurePosixPath(root)) for root in host.get(category + '_roots', [])):
            return category
    return 'personal'


def normalise(snapshot, host, profile):
    if not isinstance(snapshot, dict) or not isinstance(snapshot.get('agents'), list) or not isinstance(snapshot.get('workspaces'), list):
        raise ValueError('Invalid Herdr snapshot')
    spaces = {w['workspace_id']: clean(w.get('label'), 'Untitled') for w in snapshot['workspaces'] if isinstance(w, dict) and isinstance(w.get('workspace_id'), str)}
    agents = []
    for entry in snapshot['agents']:
        if not isinstance(entry, dict) or not isinstance(entry.get('pane_id'), str):
            raise ValueError('Invalid agent record')
        category = classification(entry.get('cwd'), host)
        if profile == 'work' and category != 'work':
            continue
        status = entry.get('agent_status')
        agents.append({'id': host['id'] + ':' + clean(entry['pane_id']), 'host': host['id'], 'category': category,
            'project': spaces.get(entry.get('workspace_id'), 'Untitled'), 'harness': clean(entry.get('agent'), 'unknown'),
            'status': status if status in STATUSES else 'unknown', 'title': clean(entry.get('terminal_title_stripped'), 'No task title reported')})
    return agents


def sanitise_metrics(raw):
    if not isinstance(raw, dict):
        raise ValueError('Invalid metrics')
    def number(value):
        return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0
    at = raw.get('at')
    if not number(at):
        raise ValueError('Invalid metric timestamp')
    result = {'at': at, 'scope': clean(raw.get('scope'), 'Unknown scope')}
    for key, fields in {'cpu': ('total', 'idle'), 'memory': ('used', 'total'), 'disk': ('used', 'total'), 'network': ('rx', 'tx'), 'gpu': ('percent', 'used', 'total')}.items():
        item = raw.get(key)
        result[key] = {f: item[f] for f in fields} if isinstance(item, dict) and all(number(item.get(f)) for f in fields) else None
        if result[key] and 'total' in fields and (item['total'] <= 0 or item.get('used', item.get('idle', 0)) > item['total']):
            result[key] = None
        if key == 'gpu' and result[key] and item['percent'] > 100:
            result[key] = None
    return result


def rates(current, previous):
    result = copy.deepcopy(current)
    result['cpu_percent'] = None
    result['rx_rate'] = result['tx_rate'] = None
    if previous:
        seconds = current.get('at', 0) - previous.get('at', 0)
        if current.get('cpu') and previous.get('cpu'):
            total = current['cpu']['total'] - previous['cpu']['total']
            idle = current['cpu']['idle'] - previous['cpu']['idle']
            if total > 0 and 0 <= idle <= total:
                result['cpu_percent'] = round(100 * (total - idle) / total, 1)
        if seconds > 0 and current.get('network') and previous.get('network'):
            for field in ('rx', 'tx'):
                delta = current['network'][field] - previous['network'][field]
                result[field + '_rate'] = delta / seconds if delta >= 0 else None
    return result


def collect(host):
    options = {'binary': host.get('herdr', 'herdr'), 'session': host.get('session')}
    script = Path(probe.__file__).read_text().split("if __name__ == '__main__':")[0]
    script += '\nprint(json.dumps(sample(**' + repr(options) + ')))\n'
    command = [sys.executable, '-']
    if host.get('transport') == 'ssh':
        command = ['ssh', '-T', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5', '-o', 'ServerAliveInterval=5', '-o', 'ServerAliveCountMax=1', '--', host['target'], 'python3 -']
    process = subprocess.run(command, input=script, capture_output=True, text=True, timeout=15, check=True)
    if len(process.stdout) > 4 * 1024 * 1024:
        raise ValueError('Collector response too large')
    result = json.loads(process.stdout)
    if not isinstance(result, dict) or not isinstance(result.get('metrics'), dict):
        raise ValueError('Invalid collector response')
    return result


class Observatory:
    def __init__(self, config, profile='work', collector=collect):
        if profile not in ('personal', 'work'):
            raise ValueError('Profile must be personal or work')
        self.config = validate_config(config)
        self.profile = profile
        self.collector = collector
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.pool = None
        self.history = []
        self.previous_metrics = {}
        self.palette = theme(None)
        self.hosts = {h['id']: {'id': h['id'], 'label': clean(h.get('label', h['id'])), 'online': False, 'error': 'Awaiting first sample', 'sampled_at': None, 'agents': [], 'metrics': None, 'trend': []} for h in config['hosts']}

    def poll(self, host):
        now = time.time()
        try:
            raw = self.collector(host)
            if not isinstance(raw, dict):
                raise ValueError('Invalid sample')
            measured_raw = sanitise_metrics(raw.get('metrics'))
            agents = normalise(raw['snapshot'], host, self.profile) if raw.get('snapshot') is not None else []
            error = raw.get('error')
            if raw.get('snapshot') is None:
                error = 'Herdr unavailable or incompatible'
            with self.lock:
                state = self.hosts[host['id']]
                previous = {a['id']: a for a in state['agents']} if state['online'] else {}
                for agent in agents:
                    old = previous.get(agent['id'])
                    agent['since'] = old['since'] if old and old['status'] == agent['status'] else now
                    if old is None or old['status'] != agent['status']:
                        self.history.insert(0, dict(agent, at=now, observation='discovered' if old is None else 'changed'))
                self.history = self.history[:100]
                measured = rates(measured_raw, self.previous_metrics.get(host['id']))
                self.previous_metrics[host['id']] = measured_raw
                trend = (state['trend'] + [{'at': now, 'working': None if error else sum(a['status'] == 'working' for a in agents)}])[-60:]
                state.update(online=not bool(error), error='Herdr unavailable or incompatible' if error else None, sampled_at=now, agents=agents if not error else [], metrics=measured, trend=trend, version=clean(raw.get('snapshot', {}).get('version') if raw.get('snapshot') else None, 'unknown'))
                if host['id'] == self.config.get('theme_host', self.config['hosts'][0]['id']):
                    self.palette = theme(raw.get('theme'))
        except (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError):
            with self.lock:
                state = self.hosts[host['id']]
                state.update(online=False, error='Collector unreachable or invalid response', agents=[], metrics=None, trend=(state['trend'] + [{'at': now, 'working': None}])[-60:])
                self.previous_metrics.pop(host['id'], None)

    def worker(self, host):
        while not self.stop.is_set():
            self.poll(host)
            self.stop.wait(self.config.get('interval', 5))

    def start(self):
        self.pool = ThreadPoolExecutor(max_workers=len(self.config['hosts']))
        for host in self.config['hosts']:
            self.pool.submit(self.worker, host)

    def close(self):
        self.stop.set()
        if self.pool:
            self.pool.shutdown(wait=True)

    def snapshot(self):
        with self.lock:
            return copy.deepcopy({'profile': self.profile, 'at': time.time(), 'interval': self.config.get('interval', 5), 'theme': self.palette, 'hosts': list(self.hosts.values()), 'history': self.history})

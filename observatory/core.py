"""Collection, disclosure and bounded observation history."""
from concurrent.futures import ThreadPoolExecutor
import copy
import json
import math
import os
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
        if host.get('transport', 'local') not in ('local', 'ssh', 'file'):
            raise ValueError('Transport must be local, ssh or file')
        if host.get('transport') == 'ssh':
            target = host.get('target', '')
            if not isinstance(target, str) or not re.fullmatch(r'[a-zA-Z0-9_.@:-]+', target) or target.startswith('-'):
                raise ValueError('SSH target must be an SSH alias or user@host')
        if host.get('transport') == 'file' and (not isinstance(host.get('path'), str) or not host['path']):
            raise ValueError('File transport needs a feed path')
        for key in ('work_roots', 'personal_roots'):
            roots = host.get(key, [])
            if not isinstance(roots, list) or any(not isinstance(r, str) or not r.startswith('/') or '..' in PurePosixPath(r).parts for r in roots):
                raise ValueError('Project roots must be absolute POSIX paths without parent traversal')
        if host.get('socket_path') and host.get('session'):
            raise ValueError('Select a socket path or a CLI session, not both')
        for key in ('herdr', 'session', 'socket_path', 'theme_path', 'disk_path'):
            if key in host and (not isinstance(host[key], str) or not host[key]):
                raise ValueError(f'{key} must be a nonempty string')
    interval = config.get('interval', 5)
    if isinstance(interval, bool) or not isinstance(interval, (int, float)) or not 2 <= interval <= 60:
        raise ValueError('Interval must be between 2 and 60 seconds')
    if config.get('theme_host', hosts[0]['id']) not in ids:
        raise ValueError('Theme host must name a configured host')
    publication = config.get('publish')
    if publication is not None:
        if not isinstance(publication, dict) or publication.get('host_id') not in ids:
            raise ValueError('Publisher needs a configured host_id')
        target = publication.get('target', '')
        if not isinstance(target, str) or not re.fullmatch(r'[a-zA-Z0-9_.@:-]+', target) or target.startswith('-'):
            raise ValueError('Invalid publisher SSH target')
        container = publication.get('container')
        if container is not None and (not isinstance(container, str) or not re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9_.-]*', container)):
            raise ValueError('Invalid receiver container')
        fields = ('path',) if container else ('directory', 'path')
        if any(not isinstance(publication.get(k), str) or not publication[k].startswith('/') for k in fields):
            raise ValueError('Publisher directory and path must be absolute')
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


def counter(value):
    return value if type(value) is int and 0 <= value <= 9007199254740991 else None


def technical(raw):
    raw = raw if isinstance(raw, dict) else {}
    result = {key: counter(raw.get(key)) for key in ('revision', 'state_change_seq')}
    result.update({key: raw[key] if type(raw.get(key)) is bool else None
                   for key in ('focused', 'interactive_ready', 'launch_pending')})
    return result


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
            'technical': technical(entry),
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


def ssh_command():
    config = os.environ.get('OBSERVATORY_SSH_CONFIG')
    return ['ssh'] + (['-F', config] if config else [])


def collect(host):
    if host.get('transport') == 'file':
        from .feed import read_feed
        return read_feed(host)
    options = {'binary': host.get('herdr', 'herdr'), 'session': host.get('session')}
    options.update({key: host[key] for key in ('socket_path', 'theme_path', 'disk_path') if key in host})
    script = Path(probe.__file__).read_text().split("if __name__ == '__main__':")[0]
    script += '\nprint(json.dumps(sample(**' + repr(options) + ')))\n'
    command = [sys.executable, '-']
    if host.get('transport') == 'ssh':
        command = ssh_command() + ['-T', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5', '-o', 'ServerAliveInterval=5', '-o', 'ServerAliveCountMax=1', '--', host['target'], 'python3 -']
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
        self.publisher = None
        self.publication = None
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
            is_feed = host.get('transport') == 'file'
            measured_raw = sanitise_metrics(raw.get('metrics')) if raw.get('metrics') is not None else None
            if measured_raw is None and not is_feed:
                raise ValueError('Missing metrics')
            agents = raw['agent_views'] if is_feed else (normalise(raw['snapshot'], host, self.profile) if raw.get('snapshot') is not None else [])
            sampled_at = raw.get('sampled_at', now) if is_feed else now
            error = raw.get('error')
            if raw.get('snapshot') is None:
                error = 'Herdr unavailable or incompatible'
            with self.lock:
                state = self.hosts[host['id']]
                if is_feed and state['sampled_at'] is not None:
                    if sampled_at < state['sampled_at']:
                        return
                    if sampled_at == state['sampled_at']:
                        if not (state['online'] and error):
                            return
                if is_feed and sampled_at != state['sampled_at'] and host['id'] == self.config.get('theme_host', self.config['hosts'][0]['id']):
                    self.palette = theme(raw.get('theme'))
                previous = {a['id']: a for a in state['agents']} if state['online'] else {}
                for agent in agents:
                    old = previous.get(agent['id'])
                    agent['since'] = old['since'] if old and old['status'] == agent['status'] else now
                    if old is None or old['status'] != agent['status']:
                        self.history.insert(0, dict(agent, at=now, observation='discovered' if old is None else 'changed'))
                self.history = self.history[:100]
                measured = rates(measured_raw, self.previous_metrics.get(host['id'])) if measured_raw is not None else None
                self.previous_metrics[host['id']] = measured_raw
                trend = (state['trend'] + [{'at': now, 'working': None if error else sum(a['status'] == 'working' for a in agents)}])[-60:]
                state.update(online=not bool(error), error='Herdr unavailable or incompatible' if error else None, sampled_at=sampled_at, agents=agents if not error else [], metrics=measured, trend=trend, protocol=counter((raw.get('snapshot') or {}).get('protocol')), version=clean(raw.get('snapshot', {}).get('version') if raw.get('snapshot') else None, 'unknown'))
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
        if self.config.get('publish'):
            from .feed import Publisher
            self.publisher = Publisher(self, self.config['publish'])
            self.publisher.start()

    def close(self):
        self.stop.set()
        if self.pool:
            self.pool.shutdown(wait=True)
        if self.publisher:
            self.publisher.close()

    def snapshot(self):
        with self.lock:
            result = copy.deepcopy({'profile': self.profile, 'at': time.time(), 'interval': self.config.get('interval', 5), 'theme': self.palette, 'hosts': list(self.hosts.values()), 'history': self.history, 'publication': self.publication})
            feeds = {h['id'] for h in self.config['hosts'] if h.get('transport') == 'file'}
            for host in result['hosts']:
                if host['id'] in feeds and host['sampled_at'] is not None and time.time() - host['sampled_at'] > 30:
                    host.update(online=False, agents=[], metrics=None, error='Feed expired')
            return result

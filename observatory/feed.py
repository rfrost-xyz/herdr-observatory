"""Work-only feed publication over SSH and bounded private-file receipt."""
import argparse
import fcntl
import json
import math
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import threading
import time

from .core import STATUSES, clean, sanitise_metrics, theme

MAX_BYTES = 1024 * 1024
MAX_AGE = 30


def validate_feed(raw, expected_host=None):
    if not isinstance(raw, dict) or raw.get('schema') != 'herdr-work-v1' or raw.get('profile') != 'work':
        raise ValueError('Expected a Work feed')
    host_id = raw.get('host_id')
    if not isinstance(host_id, str) or not host_id or (expected_host and host_id != expected_host):
        raise ValueError('Unexpected source host')
    at = raw.get('captured_at')
    if isinstance(at, bool) or not isinstance(at, (int, float)) or not math.isfinite(at) or at <= 0 or at > time.time() + 5:
        raise ValueError('Invalid capture time')
    if not isinstance(raw.get('agents'), list) or len(raw['agents']) > 1000:
        raise ValueError('Invalid agents')
    agents = []
    ids = set()
    for item in raw['agents']:
        if not isinstance(item, dict) or item.get('category') != 'work' or item.get('host') != host_id:
            raise ValueError('Only source Work agents are permitted')
        if not isinstance(item.get('id'), str) or not item['id'] or item['id'] in ids or item.get('status') not in STATUSES:
            raise ValueError('Invalid agent identity or state')
        ids.add(item['id'])
        agents.append({key: clean(item.get(key)) for key in ('id', 'host', 'category', 'project', 'harness', 'status', 'title')})
    raw_theme = raw.get('theme')
    palette = theme(raw_theme)
    metrics = sanitise_metrics(raw['metrics']) if raw.get('metrics') is not None else None
    return {'schema': 'herdr-work-v1', 'profile': 'work', 'host_id': host_id, 'captured_at': at,
            'agents': agents if raw.get('online') is True else [], 'online': raw.get('online') is True,
            'metrics': metrics, 'theme': palette, 'version': clean(raw.get('version'), 'unknown')}


def project_work(snapshot, host_id):
    host = next(h for h in snapshot['hosts'] if h['id'] == host_id)
    if host['sampled_at'] is None:
        raise ValueError('No source sample yet')
    return validate_feed({'schema': 'herdr-work-v1', 'profile': 'work', 'host_id': host_id,
        'captured_at': host['sampled_at'], 'online': host['online'], 'version': host.get('version'),
        'agents': [a for a in host['agents'] if a['category'] == 'work'],
        'metrics': host['metrics'], 'theme': snapshot['theme']}, host_id)


def atomic_receive(path, data):
    if len(data) > MAX_BYTES:
        raise ValueError('Feed exceeds size limit')
    raw = validate_feed(json.loads(data))
    path = Path(path).expanduser()
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    # Compare and replace under one cross-process lock: SSH invocations may overlap.
    lock_descriptor = os.open(str(path) + '.lock', os.O_CREAT | os.O_RDWR, 0o600)
    with os.fdopen(lock_descriptor, 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if path.exists():
            with path.open('rb') as stream:
                previous_data = stream.read(MAX_BYTES + 1)
            if len(previous_data) > MAX_BYTES:
                raise ValueError('Existing feed exceeds size limit')
            previous = validate_feed(json.loads(previous_data), raw['host_id'])
            if raw['captured_at'] < previous['captured_at']:
                return False
            if raw['captured_at'] == previous['captured_at']:
                if previous['online'] and not raw['online']:
                    raw = dict(previous, online=False, agents=[], metrics=None)
                else:
                    return False
        descriptor, name = tempfile.mkstemp(prefix='.feed-', dir=path.parent)
        try:
            with os.fdopen(descriptor, 'w') as stream:
                json.dump(raw, stream, allow_nan=False)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(name, path)
        finally:
            if os.path.exists(name):
                os.unlink(name)
    return True


def read_feed(host):
    path = Path(host['path']).expanduser()
    with path.open('rb') as stream:
        data = stream.read(MAX_BYTES + 1)
    if len(data) > MAX_BYTES:
        raise ValueError('Feed exceeds size limit')
    feed = validate_feed(json.loads(data), host['id'])
    fresh = time.time() - feed['captured_at'] <= MAX_AGE
    return {'snapshot': {'version': feed['version']}, 'agent_views': feed['agents'] if fresh else [],
            'metrics': feed['metrics'], 'theme': feed['theme'], 'sampled_at': feed['captured_at'],
            'error': None if fresh and feed['online'] else 'Source unavailable or feed expired'}


class Publisher:
    def __init__(self, observatory, config):
        self.observatory = observatory
        self.config = config
        self.thread = threading.Thread(target=self.run, name='work-feed-publisher', daemon=True)

    def once(self):
        payload = json.dumps(project_work(self.observatory.snapshot(), self.config['host_id']), allow_nan=False)
        if len(payload.encode()) > MAX_BYTES:
            raise ValueError('Feed exceeds size limit')
        # Only configured operator values enter the remote shell, individually quoted.
        remote = 'cd ' + shlex.quote(self.config['directory']) + ' && python3 -m observatory.feed ' + shlex.quote(self.config['path'])
        command = ['ssh', '-T', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=5', '-o', 'ServerAliveInterval=5', '-o', 'ServerAliveCountMax=1', '--', self.config['target'], remote]
        subprocess.run(command, input=payload, text=True, capture_output=True, check=True, timeout=15)

    def run(self):
        while not self.observatory.stop.is_set():
            try:
                self.once()
                self.observatory.publication = {'ok': True, 'at': time.time()}
            except (OSError, ValueError, KeyError, StopIteration, subprocess.SubprocessError):
                self.observatory.publication = {'ok': False, 'at': time.time()}
            self.observatory.stop.wait(self.observatory.config.get('interval', 5))

    def start(self):
        self.thread.start()

    def close(self):
        self.thread.join(timeout=16)


def main():
    parser = argparse.ArgumentParser(description='Receive an allowlisted Work feed on stdin')
    parser.add_argument('path')
    args = parser.parse_args()
    try:
        atomic_receive(args.path, sys.stdin.buffer.read(MAX_BYTES + 1))
    except (OSError, ValueError, TypeError, KeyError) as error:
        parser.error(str(error))


if __name__ == '__main__':
    main()

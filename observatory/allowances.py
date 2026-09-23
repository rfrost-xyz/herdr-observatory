"""Opt-in, account-bound allowance cache. No credentials or account IDs are stored."""
import fcntl
import datetime
import json
import math
import os
from pathlib import Path
import re
import selectors
import shlex
import subprocess
import sys
import tempfile
import threading
import time

from .allowances_probe import PLANS, integer

CACHE = '/tmp/observatory-allowances.json'
TTL = 600
LIMIT = 16384


def validate_config(config):
    if config is None:
        return
    if not isinstance(config, dict) or set(config) - {'accounts', 'sources', 'publish'}:
        raise ValueError('Invalid allowances configuration')
    accounts = config.get('accounts')
    if not isinstance(accounts, dict) or not 1 <= len(accounts) <= 4:
        raise ValueError('Allowances needs one to four account mappings')
    if any(not isinstance(key, str) or not re.fullmatch(r'[a-f0-9]{64}', key) or label not in ('Personal', 'Work') for key, label in accounts.items()):
        raise ValueError('Allowance accounts need hashed keys and explicit Personal/Work labels')
    if len(set(accounts.values())) != len(accounts):
        raise ValueError('Allowance labels must be unique')
    if type(config.get('publish', False)) is not bool:
        raise ValueError('Allowances publication must be explicit')
    sources = config.get('sources', [])
    if not isinstance(sources, list) or len(sources) > 4:
        raise ValueError('Too many allowance sources')
    for source in sources:
        if not isinstance(source, dict) or set(source) != {'target', 'container'}:
            raise ValueError('Invalid allowance source')
        for key, pattern in [('target', r'[A-Za-z0-9_.@:-]+'), ('container', r'[A-Za-z0-9][A-Za-z0-9_.-]{0,80}')]:
            value = source[key]
            if not isinstance(value, str) or value.startswith('-') or not re.fullmatch(pattern, value):
                raise ValueError('Invalid allowance source address')


def sanitise(raw, now=None):
    now = time.time() if now is None else now
    if not isinstance(raw, dict) or not isinstance(raw.get('account_key'), str) or not re.fullmatch(r'[a-f0-9]{64}', raw['account_key']):
        return None
    at = raw.get('sampled_at')
    if type(at) not in (float, int) or not 0 < at <= 9007199254740991 or not math.isfinite(at) or not 0 <= now - at <= TTL:
        return None
    out = {'account_key': raw['account_key'], 'sampled_at': at,
           'plan': raw.get('plan') if isinstance(raw.get('plan'), str) and raw['plan'] in PLANS else None}
    for key, maximum in [('weekly_remaining', 100), ('weekly_resets_at', 9007199254740991),
                         ('reset_count', 10000), ('reset_expires_at', 9007199254740991)]:
        out[key] = integer(raw.get(key), maximum)
    if out['weekly_resets_at'] is None or out['weekly_resets_at'] <= now:
        out['weekly_remaining'] = out['weekly_resets_at'] = None
    if out['reset_expires_at'] is not None and out['reset_expires_at'] <= now:
        out['reset_count'] = out['reset_expires_at'] = None
    for key in ('lifetime_tokens', 'peak_daily_tokens'):
        out[key] = integer(raw.get(key))
    buckets = raw.get('daily_usage')
    out['daily_usage'] = None
    if isinstance(buckets, list) and len(buckets) <= 30:
        daily = []
        for item in buckets:
            if not isinstance(item, dict) or not isinstance(item.get('date'), str) or integer(item.get('tokens')) is None:
                daily = None; break
            try:
                date = datetime.date.fromisoformat(item['date'])
                valid = date.isoformat() == item['date'] and date <= datetime.date.fromtimestamp(now)
            except (ValueError, OverflowError):
                valid = False
            if not valid:
                daily = None; break
            daily.append({'date': item['date'], 'tokens': item['tokens']})
        if daily is not None and len({item['date'] for item in daily}) == len(daily):
            out['daily_usage'] = sorted(daily, key=lambda item: item['date'])
    return out


def read_cache(path=CACHE, now=None):
    try:
        with open(path, 'rb') as stream:
            data = stream.read(LIMIT + 1)
        if len(data) > LIMIT:
            return []
        rows = json.loads(data)
        return [row for item in rows if (row := sanitise(item, now))] if isinstance(rows, list) and len(rows) <= 4 else []
    except (OSError, ValueError, TypeError, OverflowError):
        return []


def receive(raw, config, path=CACHE, now=None):
    row = sanitise(raw, now)
    if not config or not row or row['account_key'] not in config['accounts']:
        return False
    with open(path + '.lock', 'a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return False
        previous = {r['account_key']: r for r in read_cache(path, now)}
        old = previous.get(row['account_key'])
        if old and old['sampled_at'] >= row['sampled_at']:
            return False
        previous[row['account_key']] = row
        rows = sorted(previous.values(), key=lambda r: r['sampled_at'], reverse=True)[:4]
        fd, temporary = tempfile.mkstemp(prefix='.allowances-', dir=str(Path(path).parent))
        try:
            with os.fdopen(fd, 'w') as stream:
                json.dump(rows, stream, allow_nan=False)
            os.replace(temporary, path)
        finally:
            if os.path.exists(temporary): os.unlink(temporary)
    return True



def bounded_command(command):
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    result = bytearray()
    deadline = time.monotonic() + 6
    try:
        while time.monotonic() < deadline:
            if not selector.select(max(0, deadline - time.monotonic())): break
            chunk = os.read(process.stdout.fileno(), LIMIT + 1)
            if not chunk:
                return bytes(result) if process.wait(timeout=.5) == 0 else None
            result.extend(chunk)
            if len(result) > LIMIT: break
        return None
    finally:
        selector.close()
        if process.poll() is None: process.kill()
        process.wait(); process.stdout.close()


class Allowances:
    def __init__(self, config):
        validate_config(config)
        self.config = config
        self.remote = []
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.thread = None

    def records(self, received=()):
        if not self.config:
            return []
        with self.lock:
            candidates = read_cache() + list(self.remote) + list(received)
        newest = {}
        for item in candidates:
            row = sanitise(item)
            if row and row['account_key'] in self.config['accounts']:
                old = newest.get(row['account_key'])
                if not old or old['sampled_at'] < row['sampled_at']:
                    newest[row['account_key']] = row
        return list(newest.values())

    def snapshot(self, received=()):
        if not self.config:
            return []
        rows = {r['account_key']: r for r in self.records(received)}
        result = []
        for key, label in self.config['accounts'].items():
            row = rows.get(key)
            public = {field: row.get(field) if row else None for field in
                      ('plan', 'weekly_remaining', 'weekly_resets_at', 'reset_count', 'reset_expires_at',
                       'sampled_at', 'lifetime_tokens', 'peak_daily_tokens', 'daily_usage')}
            result.append(dict(public, label=label, available=row is not None))
        return result

    def poll(self):
        from .core import ssh_command
        collected = []
        for source in self.config.get('sources', []):
            remote = 'docker exec ' + shlex.quote(source['container']) + ' python3 -m observatory.allowances --export'
            command = ssh_command() + ['-T', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=3', '--', source['target'], remote]
            try:
                # Remote output is the bounded image cache, never the native account response.
                data = bounded_command(command)
                if data is not None:
                    rows = json.loads(data)
                    if isinstance(rows, list) and len(rows) <= 4:
                        collected.extend(row for item in rows if (row := sanitise(item)))
            except (OSError, ValueError, TypeError, subprocess.SubprocessError):
                pass
        with self.lock:
            self.remote = collected

    def run(self):
        while not self.stop.is_set():
            self.poll()
            self.stop.wait(60)

    def start(self):
        if self.config and self.config.get('sources'):
            self.thread = threading.Thread(target=self.run, name='allowances-collector', daemon=True)
            self.thread.start()

    def close(self):
        self.stop.set()
        if self.thread: self.thread.join(timeout=25)


def export_rows(config_path='/config/config.json'):
    try:
        config = json.loads(Path(config_path).read_text()).get('allowances')
        validate_config(config)
        return [row for row in read_cache() if config and row['account_key'] in config['accounts']]
    except (OSError, ValueError, TypeError, KeyError):
        return []


if __name__ == '__main__':
    if sys.argv[1:] == ['--receive']:
        try:
            config = json.loads(Path('/config/config.json').read_text()).get('allowances')
            validate_config(config)
            data = sys.stdin.buffer.read(LIMIT + 1)
            if len(data) <= LIMIT: receive(json.loads(data), config)
        except (OSError, ValueError, TypeError, KeyError): pass
    elif sys.argv[1:] == ['--export']:
        print(json.dumps(export_rows(), allow_nan=False))

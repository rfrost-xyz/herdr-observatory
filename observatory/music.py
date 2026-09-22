"""Bounded, read-only cliamp playback observations and ephemeral forwarding."""
import argparse
import copy
import json
import math
import os
from pathlib import Path
import re
import select
import shlex
import socket
import subprocess
import sys
import tempfile
import threading
import time

TTL = 3.0
LIMIT = 4096


def unavailable():
    return {'available': False, 'state': 'unavailable', 'title': '', 'artist': '', 'bands': [], 'captured_at': None}


def validate_config(config):
    if config is None:
        return
    if not isinstance(config, dict) or set(config) - {'socket_path', 'path', 'publish'}:
        raise ValueError('Invalid music configuration')
    if sum(key in config for key in ('socket_path', 'path')) != 1:
        raise ValueError('Music needs one socket or feed path')
    for key in ('socket_path', 'path'):
        if key in config and (not isinstance(config[key], str) or not config[key].startswith('/') or '\x00' in config[key] or '..' in Path(config[key]).parts):
            raise ValueError('Music paths must be absolute without parent traversal')
    publish = config.get('publish')
    if publish is not None:
        if 'socket_path' not in config or not isinstance(publish, dict) or set(publish) != {'target', 'container', 'path'}:
            raise ValueError('Only local music can publish to a configured receiver')
        for key, pattern in [('target', r'[a-zA-Z0-9_.@:-]+'), ('container', r'[a-zA-Z0-9][a-zA-Z0-9_.-]*')]:
            value = publish[key]
            if not isinstance(value, str) or value.startswith('-') or not re.fullmatch(pattern, value):
                raise ValueError('Invalid music publisher destination')
        path = publish['path']
        if not isinstance(path, str) or not path.startswith('/') or '\x00' in path or '..' in Path(path).parts:
            raise ValueError('Invalid music receiver path')


def _finite(value):
    if isinstance(value, bool) or not isinstance(value, (float, int)):
        return False
    try:
        return math.isfinite(value)
    except OverflowError:
        return False


def _label(value):
    return re.sub(r'[\x00-\x1f\x7f-\x9f]', '', value)[:160] if isinstance(value, str) else ''


def sanitise(raw, now=None):
    now = time.time() if now is None else now
    if not isinstance(raw, dict) or raw.get('available') is not True:
        return unavailable()
    at, bands, state = raw.get('captured_at'), raw.get('bands'), raw.get('state')
    if not _finite(at) or not 0 <= now - at <= TTL or state not in ('playing', 'paused', 'stopped'):
        return unavailable()
    if not isinstance(bands, list) or not 1 <= len(bands) <= 64 or any(not _finite(v) or not 0 <= v <= 1 for v in bands):
        return unavailable()
    return {'available': True, 'state': state, 'title': _label(raw.get('title')), 'artist': _label(raw.get('artist')),
            'bands': bands[:] if state == 'playing' else [0.0] * len(bands), 'captured_at': at}


class Music:
    def __init__(self, config=None):
        validate_config(config)
        self.config = config
        self._value = unavailable()
        self._lock = threading.Lock()
        self._stop = threading.Event()
        self._thread = None
        self._socket = None
        self._buffer = b''
        self._publisher = None
        self._retry_publish = 0.0

    def start(self):
        if self.config and self._thread is None:
            self._thread = threading.Thread(target=self._run, name='music-observer', daemon=True)
            self._thread.start()

    def close(self):
        self._stop.set()
        if self._thread:
            self._thread.join(timeout=3)
        self._disconnect()
        self._close_publisher()

    def snapshot(self):
        with self._lock:
            return sanitise(copy.deepcopy(self._value))

    def _disconnect(self):
        if self._socket:
            self._socket.close()
        self._socket = None
        self._buffer = b''

    def _request(self, method):
        if method not in ('state.get', 'spectrum.get'):
            raise ValueError('Read-only music method required')
        if self._socket is None:
            self._socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            self._socket.settimeout(.4)
            self._socket.connect(self.config['socket_path'])
        deadline = time.monotonic() + .4
        self._socket.settimeout(.4)
        self._socket.sendall((json.dumps({'version': 2, 'id': method, 'method': method}) + '\n').encode())
        while b'\n' not in self._buffer:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError('Music frame deadline')
            self._socket.settimeout(remaining)
            chunk = self._socket.recv(min(4096, 65537 - len(self._buffer)))
            if not chunk or len(self._buffer) + len(chunk) > 65536:
                raise ValueError('Invalid music frame')
            self._buffer += chunk
        line, self._buffer = self._buffer.split(b'\n', 1)
        result = json.loads(line)
        if not isinstance(result, dict) or result.get('version') != 2 or result.get('id') != method or result.get('ok') is not True:
            raise ValueError('Invalid music response')
        return result

    def _sample(self):
        if 'path' in self.config:
            with open(self.config['path'], 'rb') as source:
                raw = source.read(LIMIT + 1)
            if len(raw) > LIMIT:
                raise ValueError('Oversized music feed')
            return sanitise(json.loads(raw))
        state = self._request('state.get').get('snapshot')
        spectrum = self._request('spectrum.get').get('result')
        if not isinstance(state, dict) or not isinstance(spectrum, dict) or spectrum.get('ok') is not True:
            raise ValueError('Missing music fields')
        track = state.get('track') or {}
        if not isinstance(track, dict):
            raise ValueError('Invalid music track')
        return sanitise({'available': True, 'state': state.get('state'), 'title': track.get('title'),
                         'artist': track.get('artist'), 'bands': spectrum.get('bands'), 'captured_at': time.time()})

    def _close_publisher(self):
        process, self._publisher = self._publisher, None
        if process:
            process.stdin.close()
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=.3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=.3)

    def _publish(self, value):
        config = self.config.get('publish')
        if not config or time.monotonic() < self._retry_publish:
            return
        try:
            if self._publisher is None:
                from .core import ssh_command
                remote = shlex.join(['docker', 'exec', '-i', config['container'], 'python3', '-m', 'observatory.music', '--receive', config['path']])
                self._publisher = subprocess.Popen(ssh_command() + ['-T', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=2', '-o', 'ServerAliveInterval=2', '-o', 'ServerAliveCountMax=1', '--', config['target'], remote], stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, bufsize=0)
                os.set_blocking(self._publisher.stdin.fileno(), False)
            payload = (json.dumps(value, ensure_ascii=False, separators=(',', ':')) + '\n').encode()
            if len(payload) > LIMIT:
                payload = (json.dumps(unavailable()) + '\n').encode()
            descriptor = self._publisher.stdin.fileno()
            if not select.select([], [descriptor], [], 0)[1] or os.write(descriptor, payload) != len(payload):
                raise OSError('Music publication backpressure')
        except (OSError, ValueError):
            self._close_publisher()
            self._retry_publish = time.monotonic() + 2

    def _run(self):
        while not self._stop.is_set():
            started = time.monotonic()
            try:
                value = self._sample()
            except (OSError, ValueError, TypeError, RecursionError):
                self._disconnect()
                value = unavailable()
            with self._lock:
                self._value = value
            self._publish(value)
            self._stop.wait(max(.01, 1 / 15 - (time.monotonic() - started)))


def receive(path, stream=None):
    """Replace only the explicitly configured ephemeral latest-frame file."""
    stream = sys.stdin.buffer if stream is None else stream
    destination = Path(path)
    while True:
        line = stream.readline(LIMIT + 1)
        if not line:
            return
        if len(line) > LIMIT or not line.endswith(b'\n'):
            return
        try:
            value = sanitise(json.loads(line))
        except (ValueError, TypeError, RecursionError):
            value = unavailable()
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(mode='w', dir=destination.parent, prefix='.music-', delete=False) as target:
                temporary = target.name
                json.dump(value, target, separators=(',', ':'))
            os.replace(temporary, destination)
            temporary = None
        finally:
            if temporary:
                os.unlink(temporary)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--receive', required=True)
    args = parser.parse_args()
    receive(args.receive)

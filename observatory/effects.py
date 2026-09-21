"""Bounded ttfx library adapter; input is exclusively the server's filtered snapshot."""
import hashlib
import io
import os
import subprocess
import threading
import time


def lines_for(state, now):
    lines = []
    captures = []
    for host in state['hosts']:
        at = host.get('sampled_at')
        if not host['online'] or not isinstance(at, (int, float)) or now-at >= state['interval']+20:
            continue
        captures.append((host['id'], at))
        metrics = host.get('metrics') or {}
        cpu = metrics.get('cpu_percent')
        lines.append(f"recv {host['id']} cpu={cpu if cpu is not None else 'unavailable'}% panes={len(host['agents'])}")
        for agent in host['agents']:
            tech = agent.get('technical') or {}
            lines.append(f"{agent['host']}/{agent['project']} {agent['status'].upper()} {agent['harness']} rev={tech.get('revision') if tech.get('revision') is not None else 'unavailable'} seq={tech.get('state_change_seq') if tech.get('state_change_seq') is not None else 'unavailable'} {agent['title']}")
    # Restrict adapter input to printable ASCII cells, never ANSI terminal controls.
    safe = [''.join(c if 32 <= ord(c) < 127 else '?' for c in line)[:100] for line in lines]
    if not safe:
        return '', 'No fresh source observations.'
    key = hashlib.sha256(repr(captures).encode()).hexdigest()[:20]
    offset = int(max(at for _, at in captures)) // max(1, int(state['interval'])) % max(1, (len(safe)+5)//6)*6
    return key, '\n'.join(safe[offset:offset+6])


def parse_frames(raw):
    if len(raw) > 1024*1024:
        raise ValueError('oversized frames')
    stream = io.BytesIO(raw)
    frames = []
    while length := stream.readline(12):
        size = int(length)
        if not 0 <= size <= 8192 or len(frames) >= 120:
            raise ValueError('invalid frame size')
        frame = stream.read(size)
        if len(frame) != size or stream.read(1) != b'\n':
            raise ValueError('truncated frame')
        decoded = frame.decode('utf-8')
        if any(ord(c) < 32 and c != '\n' for c in decoded):
            raise ValueError('unexpected terminal control')
        if len(decoded.splitlines()) > 6 or any(len(row) > 100 for row in decoded.splitlines()):
            raise ValueError('invalid frame dimensions')
        frames.append(decoded)
    return frames


class TextEffects:
    def __init__(self):
        self.lock = threading.Lock()
        self.cached = None
        self.last_attempt = 0

    def snapshot(self, state):
        now = time.time()
        key, text = lines_for(state, now)
        sources = [{'id': h['id'], 'at': h['sampled_at']} for h in state['hosts'] if h['online'] and isinstance(h.get('sampled_at'), (int,float)) and now-h['sampled_at'] < state['interval']+20]
        empty = {'key': key, 'text': text, 'frames': [], 'effect': 'plain', 'fps': 30, 'sources': sources}
        if not key:
            return empty
        with self.lock:
            if self.cached and self.cached['key'] == key:
                return self.cached
            if now-self.last_attempt < 1:
                return empty
            self.last_attempt = now
            effect = ('decrypt', 'vhstape', 'crumble')[int(key[:6], 16) % 3]
            try:
                result = subprocess.run([os.environ.get('OBSERVATORY_TEXT_RENDERER', '/usr/local/bin/observatory-text'), effect],
                    input=text.encode(), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=2, check=True)
                frames = parse_frames(result.stdout)
                value = dict(empty, frames=frames, effect=effect)
            except (OSError, subprocess.SubprocessError, ValueError, UnicodeError):
                value = empty
            self.cached = value
            return value

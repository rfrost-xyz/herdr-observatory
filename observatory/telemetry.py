"""Short-lived hook reporter. No daemon, raw transcript storage or HTTP intake."""
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import socket
import sys
import time

from .probe import TELEMETRY_NUMBERS, session_binding, telemetry_view

CODEX_EVENTS = {'SessionStart': ('session', 'ready'), 'UserPromptSubmit': ('turn', 'working'),
                'PreToolUse': ('tool-start', 'tool'), 'PostToolUse': ('tool-end', 'working'),
                'PreCompact': ('compact-start', 'compacting'), 'PostCompact': ('compact-end', 'working'),
                'Stop': ('idle', 'idle'), 'Interrupt': ('interrupt', 'interrupted'), 'SessionEnd': ('end', 'ended'),
                'SubagentStart': ('subagent-start', 'working'), 'SubagentStop': ('subagent-stop', 'working')}
KEYS = ('v', 'bind', 'seq', 'event', 'phase', 'tool', 'model', 'result', 'usage_source') + TELEMETRY_NUMBERS


def rpc(path, method, params):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(.4)
        client.connect(path)
        client.sendall(json.dumps({'id': 'observatory-hook', 'method': method, 'params': params}).encode() + b'\n')
        data = bytearray()
        while b'\n' not in data and len(data) <= 1024 * 1024:
            chunk = client.recv(65536)
            if not chunk:
                raise ValueError('Incomplete frame')
            data.extend(chunk)
        if len(data) > 1024 * 1024:
            raise ValueError('Oversized frame')
        result = json.loads(data.split(b'\n', 1)[0])
        if result.get('id') != 'observatory-hook' or 'error' in result:
            raise ValueError('Rejected report')
        return result['result']


def event_view(harness, raw, seq):
    if not isinstance(raw, dict):
        return None
    if harness == 'codex':
        event = CODEX_EVENTS.get(raw.get('hook_event_name'))
        if not event:
            return None
        value = {'seq': seq, 'event': event[0], 'phase': event[1], 'tool': raw.get('tool_name'), 'model': raw.get('model'),
                 'result': 'finished' if event[0] == 'tool-end' else None}
        usage = raw.get('observatory_usage')
        if isinstance(usage, dict) and usage.get('usage_source') == 'codex-rollout' and type(usage.get('usage_seq')) is int:
            value.update({key: usage.get(key) for key in TELEMETRY_NUMBERS + ('usage_source',)})
    elif harness == 'pi':
        value = {**raw, 'seq': seq}
    else:
        return None
    # MCP server names and custom extension names can disclose private details.
    tool = value.get('tool')
    if tool:
        known = {'Bash', 'bash', 'apply_patch', 'read', 'write', 'edit', 'grep', 'find', 'ls', 'exec_command', 'write_stdin', 'exec', 'web'}
        value['tool'] = tool if tool in known else ('mcp-tool' if isinstance(tool, str) and tool.startswith('mcp') else 'custom-tool')
    return telemetry_view(value)


def report(harness, raw, pane_id, seq, config_path='/config/config.json'):
    event = event_view(harness, raw, seq)
    if event is None or not isinstance(pane_id, str) or not re.fullmatch(r'[A-Za-z0-9:_-]{1,80}', pane_id):
        return False
    config = json.loads(Path(config_path).read_text())
    hosts = [h for h in config['hosts'] if h.get('transport', 'local') == 'local']
    if len(hosts) != 1 or not hosts[0].get('socket_path'):
        return False
    path = hosts[0]['socket_path']
    # One lock file per container; no per-session files or unbounded spool.
    with open('/tmp/observatory-hook.lock', 'a') as lock:
        deadline = time.monotonic() + .4
        while True:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    return False
                time.sleep(.01)
        agent = rpc(path, 'pane.get', {'pane_id': pane_id})['pane']
        ref = agent.get('agent_session', {})
        expected = raw.get('session_path') if ref.get('kind') == 'path' and harness == 'pi' else raw.get('session_id')
        if agent.get('agent') != harness or not session_binding(agent) or not expected or ref.get('value') != expected:
            return False
        tokens = agent.get('tokens', {})
        old = tokens.get('obs_seq', '')
        if isinstance(old, str) and old.isdigit() and int(old) >= seq:
            return False
        owned = {k: event.get(k) for k in KEYS}
        owned.update(v='1', bind=session_binding(agent))
        params = {'pane_id': pane_id, 'source': 'user:observatory', 'agent': harness,
                  'seq': seq, 'ttl_ms': 120000,
                  'tokens': {'obs_' + k: str(v) if v is not None else None for k, v in owned.items()}}
        # Guarded display name is supplemental; it never replaces semantic state.
        label = f"{harness} · {event['phase']}"
        if event.get('tool'):
            label += ' · ' + event['tool']
        if event.get('context') is not None and event.get('window'):
            label += f" · ctx~{round(100 * event['context'] / event['window'])}%"
        params['display_agent'] = label[:80]
        rpc(path, 'pane.report_metadata', params)
        return True


def main():
    # Hard process deadline also bounds a malicious slow/trickling socket or stdin.
    def expired(*_args):
        raise TimeoutError()
    signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, 1.5)
    try:
        harness, pane, seq = sys.argv[1:4]
        data = sys.stdin.buffer.read(1024 * 1024 + 1)
        if len(data) <= 1024 * 1024:
            report(harness, json.loads(data), pane, int(seq))
    except (OSError, ValueError, TypeError, KeyError, AttributeError, OverflowError):
        pass
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()

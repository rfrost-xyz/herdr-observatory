# herdr-observatory adapter v1; installed from the running image
"""Ephemeral host-side Codex quota read. Only an allowlisted summary leaves stdout."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import selectors
import signal
import subprocess
import time

MAX_BYTES = 256 * 1024
PLANS = {'free', 'go', 'plus', 'pro', 'prolite', 'team', 'self_serve_business_prolite',
         'self_serve_business_usage_based', 'business', 'ent26', 'enterprise_cbp_automation',
         'enterprise_cbp_usage_based', 'enterprise', 'edu', 'edu_plus', 'edu_pro', 'unknown'}


def integer(value, maximum=9007199254740991):
    return value if type(value) is int and 0 <= value <= maximum else None


def summarise(raw, now=None):
    now = time.time() if now is None else now
    if not isinstance(raw, dict):
        return None
    identity = raw.get('accountId')
    if not isinstance(identity, str) or not 1 <= len(identity) <= 256:
        return None
    bucket = raw.get('rateLimits')
    buckets = raw.get('rateLimitsByLimitId')
    if isinstance(buckets, dict) and 'codex' in buckets:
        bucket = buckets['codex']
    if not isinstance(bucket, dict):
        return None
    # Other metered model buckets must not masquerade as the general Codex quota.
    if bucket.get('limitId') not in (None, 'codex'):
        return None
    weekly = [bucket[k] for k in ('primary', 'secondary')
              if isinstance(bucket.get(k), dict) and bucket[k].get('windowDurationMins') == 10080]
    remaining = resets = None
    if len(weekly) == 1:
        used = integer(weekly[0].get('usedPercent'), 100)
        resets = integer(weekly[0].get('resetsAt'))
        if used is not None and resets is not None and resets > now:
            remaining = 100 - used
        else:
            resets = None
    credits = raw.get('rateLimitResetCredits')
    count = expiry = None
    if isinstance(credits, dict):
        count = integer(credits.get('availableCount'), 10000)
        details = credits.get('credits')
        if isinstance(details, list) and len(details) <= 1000:
            # A capped list cannot establish the earliest expiry of all passes.
            available = [row for row in details if isinstance(row, dict)
                         and row.get('status') == 'available' and row.get('resetType') == 'codexRateLimits']
            ids = [row.get('id') for row in available]
            complete = (count is not None and len(available) == count
                        and all(isinstance(key, str) and key for key in ids) and len(set(ids)) == len(ids))
            if complete:
                expiries = [row.get('expiresAt') for row in available]
                valid = all(at is None or integer(at) is not None for at in expiries)
                if valid and any(at is not None and at <= now for at in expiries):
                    count = None
                elif valid:
                    future = [at for at in expiries if at is not None]
                    expiry = min(future) if future else None
    return {'account_key': hashlib.sha256(('observatory-codex-account-v1:' + identity).encode()).hexdigest(),
            'plan': bucket.get('planType') if isinstance(bucket.get('planType'), str) and bucket['planType'] in PLANS else None,
            'weekly_remaining': remaining, 'weekly_resets_at': resets,
            'reset_count': count, 'reset_expires_at': expiry, 'sampled_at': now}


def codex_binary():
    binary = shutil.which('codex')
    if binary:
        return binary
    # The existing user-managed shim is also used by noninteractive SSH sessions.
    fallback = Path.home() / '.local/bin/codex'
    return str(fallback) if fallback.is_file() and os.access(fallback, os.X_OK) else None


def read_account(timeout=5):
    """Only initialise and read limits. Never start threads, tools or redemption RPCs."""
    binary = codex_binary()
    if binary is None:
        return None
    process = subprocess.Popen([binary, '-c', 'analytics.enabled=false', '-c', 'mcp_servers={}',
                                'app-server', '--stdio'], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, start_new_session=True)
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    def send(value):
        process.stdin.write(json.dumps(value).encode() + b'\n'); process.stdin.flush()
    deadline = time.monotonic() + timeout
    buffer = b''
    size = 0
    requested = False
    try:
        send({'id': 1, 'method': 'initialize', 'params': {
            'clientInfo': {'name': 'herdr_observatory_allowances', 'version': '1'},
            'capabilities': {'experimentalApi': True}}})
        while time.monotonic() < deadline:
            if not selector.select(max(0, deadline - time.monotonic())):
                break
            chunk = os.read(process.stdout.fileno(), 65536)
            if not chunk:
                break
            size += len(chunk)
            if size > MAX_BYTES:
                break
            buffer += chunk
            while b'\n' in buffer:
                line, buffer = buffer.split(b'\n', 1)
                item = json.loads(line)
                if not isinstance(item, dict):
                    return None
                if item.get('id') == 1 and not requested:
                    if 'error' in item:
                        return None
                    send({'method': 'initialized'})
                    send({'id': 2, 'method': 'account/rateLimits/read'})
                    requested = True
                elif item.get('id') == 2 and requested:
                    return summarise(item.get('result')) if 'error' not in item else None
        return None
    finally:
        selector.close()
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
        process.stdin.close(); process.stdout.close()


if __name__ == '__main__':
    try:
        result = read_account()
    except (OSError, ValueError, TypeError, OverflowError, RecursionError, subprocess.SubprocessError):
        result = None
    print(json.dumps(result, allow_nan=False))

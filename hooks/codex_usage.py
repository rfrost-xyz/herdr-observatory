#!/usr/bin/env python3
# herdr-observatory adapter v1; installed from the running image
"""Bounded, hook-time numeric enrichment. No transcript content leaves this process."""
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import time

TAIL_BYTES = 512 * 1024
LINE_BYTES = 64 * 1024
MAX_INPUT = 1024 * 1024
FIELDS = {'input': 'input_tokens', 'output_tokens': 'output_tokens',
          'cache_read': 'cached_input_tokens', 'cache_write': 'cache_write_input_tokens',
          'context': 'total_tokens'}


def safe_number(value):
    return value if type(value) is int and 0 <= value <= 9007199254740991 else None


def open_session(path, root):
    """Walk beneath the trusted session root using no-follow directory handles."""
    path, root = Path(path), Path(root)
    if not path.is_absolute() or not root.is_absolute() or '..' in path.parts or '..' in root.parts:
        raise ValueError('Invalid path')
    relative = path.relative_to(root)
    if not relative.parts or path.suffix != '.jsonl':
        raise ValueError('Invalid session file')
    # Reject symlinks anywhere in the root too; no implicit expansion to other trees.
    descriptor = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
    try:
        for part in root.parts[1:] + relative.parts[:-1]:
            next_fd = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=descriptor)
            os.close(descriptor); descriptor = next_fd
        file_fd = os.open(relative.parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=descriptor)
        info = os.fstat(file_fd)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid():
            os.close(file_fd); raise ValueError('Invalid owner or file type')
        return os.fdopen(file_fd, 'rb')
    finally:
        os.close(descriptor)


def read_usage(raw, root=None, now=None):
    now = time.time() if now is None else now
    root = root or Path(os.environ.get('CODEX_HOME', str(Path.home()/'.codex'))) / 'sessions'
    path, session = raw.get('transcript_path'), raw.get('session_id')
    if not isinstance(path, str) or not isinstance(session, str) or not session:
        return {}
    try:
        with open_session(path, root) as stream:
            header = stream.readline(LINE_BYTES + 1)
            if len(header)>LINE_BYTES or not header.endswith(b'\n'):
                return {}
            metadata = json.loads(header)
            if metadata.get('type') != 'session_meta' or metadata.get('payload', {}).get('id') != session:
                return {}
            size = os.fstat(stream.fileno()).st_size
            start = max(len(header), size - TAIL_BYTES)
            stream.seek(start)
            tail = stream.read(TAIL_BYTES)
        lines = tail.splitlines(keepends=True)
        if start>len(header): lines=lines[1:]
        for line in reversed(lines):
            if len(line)>LINE_BYTES or not line.endswith(b'\n') or b'"token_count"' not in line:
                continue
            try:
                event = json.loads(line)
                payload=event.get('payload', {})
                if event.get('type')!='event_msg' or payload.get('type')!='token_count':
                    continue
                info=payload.get('info')
                if not isinstance(info, dict): continue
                usage=info.get('last_token_usage')
                if not isinstance(usage,dict): return {}
                stamp=datetime.datetime.fromisoformat(event['timestamp'].replace('Z','+00:00'))
                if stamp.tzinfo is None: return {}
                captured=stamp.timestamp()
                if captured>now: return {}
                result={key:safe_number(usage.get(source)) for key,source in FIELDS.items()}
                totals=info.get('total_token_usage')
                for key,source in (('total_input','input_tokens'),('total_output','output_tokens'),('total_cache_read','cached_input_tokens'),('total_cache_write','cache_write_input_tokens')):
                    result[key]=safe_number(totals.get(source)) if isinstance(totals,dict) else None
                input_total,cache_total=result['total_input'],result['total_cache_read']
                result['total_uncached_input']=input_total-cache_total if input_total is not None and cache_total is not None and cache_total<=input_total else None
                result['compactions']=None
                if start==len(header):
                    try:
                        complete=[json.loads(item) for item in lines if item.endswith(b'\n') and len(item)<=LINE_BYTES]
                        if len(complete)==len(lines) and all(isinstance(item,dict) for item in complete):
                            markers=sum(item.get('type')=='event_msg' and isinstance(item.get('payload'),dict) and item['payload'].get('type')=='context_compacted' for item in complete)
                            summaries=sum(item.get('type')=='compacted' for item in complete)
                            # Some versions persist both a summary and its notification.
                            result['compactions']=max(markers,summaries)
                    except (ValueError,TypeError,RecursionError): pass
                result['window']=safe_number(info.get('model_context_window'))
                # Codex rust-v0.155.1 TUI subtracts its 12,000-token baseline
                # before rounding remaining percentage; display usage is its complement.
                context,window=result['context'],result['window']
                result['context_percent']=None
                if context is not None and window is not None:
                    if window<=12000: result['context_percent']=100
                    else:
                        remaining=max((window-12000)-max(context-12000,0),0)
                        remaining_percent=min(100,max(0,remaining/(window-12000)*100))
                        result['context_percent']=100-int(remaining_percent+0.5)
                if result['window']==0 or (result['context'] is not None and result['window'] is not None and result['context']>result['window']):
                    result['context']=result['window']=result['context_percent']=None
                if not any(value is not None for value in result.values()): return {}
                return {**result,'usage_seq':int(captured*1e6),'usage_source':'codex-rollout'}
            except (ValueError,TypeError,KeyError,AttributeError,OverflowError,RecursionError):
                continue
    except (OSError,ValueError,TypeError,AttributeError,RecursionError):
        pass
    return {}


def opaque_association(raw):
    """Validate source-side turn/child keys without exporting native identifiers."""
    if not isinstance(raw, dict):
        return {}
    session = raw.get('session_id')
    if not isinstance(session, str) or not re.fullmatch(r'[A-Za-z0-9_-]{1,128}', session):
        return {}
    result = {}
    for source, target in (('turn_id', 'turn_key'), ('agent_id', 'child_key')):
        value = raw.get(source)
        if isinstance(value, str) and re.fullmatch(r'[A-Za-z0-9_-]{1,128}', value):
            result[target] = hashlib.sha256(('observatory-association-v1:' + session + ':' + source + ':' + value).encode()).hexdigest()[:24]
    return result


def enrich(raw):
    if not isinstance(raw,dict): return None
    # Raw prompts, arguments, transcript paths and messages never cross Docker stdin.
    result={key:raw.get(key) for key in ('session_id','hook_event_name','tool_name','model')}
    result['observatory_usage']=read_usage(raw)
    result['observatory_association']=opaque_association(raw)
    return result



def refresh_allowances(force=False):
    if '__ALLOWANCES__' != 'true':
        return
    from allowances_probe import read_account
    # Private timestamp only; no account data is persisted on the host.
    path = Path(__file__).parent / 'allowances-refresh.lock'
    descriptor = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    info = os.fstat(descriptor)
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_size > 64:
        os.close(descriptor)
        return
    with os.fdopen(descriptor, 'r+') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return
        try:
            previous = float(lock.read(64) or '0')
        except ValueError:
            previous = 0
        now = time.time()
        if not force and 0 <= now - previous < 60:
            return
        lock.seek(0); lock.truncate(); lock.write(str(now)); lock.flush()
        row = read_account(timeout=4)
        if row:
            subprocess.run(['docker', 'exec', '-i', '__CONTAINER__', 'python3', '-m',
                            'observatory.allowances', '--receive'], input=json.dumps(row).encode(),
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=1, check=False)


def main():
    if sys.argv[1:] == ['--refresh-allowances']:
        try: refresh_allowances(force=True)
        except (OSError, ValueError, TypeError, subprocess.SubprocessError): pass
        return
    if os.environ.get('HERDR_ENV')!='1' or not os.environ.get('HERDR_PANE_ID'): return
    try:
        data=sys.stdin.buffer.read(MAX_INPUT+1)
        if len(data)>MAX_INPUT: return
        item=enrich(json.loads(data))
        if item is None:return
        subprocess.run(['docker','exec','-i','__CONTAINER__','python3','-m','observatory.telemetry',
                        'codex',os.environ['HERDR_PANE_ID'],str(time.time_ns()//1000)],
                       input=json.dumps(item).encode(),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=1.5,check=False)
        refresh_allowances()
    except (OSError,ValueError,TypeError,RecursionError,subprocess.SubprocessError):
        pass


if __name__=='__main__': main()

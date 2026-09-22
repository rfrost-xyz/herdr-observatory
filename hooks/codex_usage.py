#!/usr/bin/env python3
# herdr-observatory adapter v1; installed from the running image
"""Bounded, hook-time numeric enrichment. No transcript content leaves this process."""
import datetime
import json
import os
from pathlib import Path
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
                if not 0<=now-captured<=120: return {}
                result={key:safe_number(usage.get(source)) for key,source in FIELDS.items()}
                result['window']=safe_number(info.get('model_context_window'))
                if result['window']==0 or (result['context'] is not None and result['window'] is not None and result['context']>result['window']):
                    result['context']=result['window']=None
                if not any(value is not None for value in result.values()): return {}
                return {**result,'usage_seq':int(captured*1e6),'usage_source':'codex-rollout'}
            except (ValueError,TypeError,KeyError,AttributeError,OverflowError,RecursionError):
                continue
    except (OSError,ValueError,TypeError,AttributeError,RecursionError):
        pass
    return {}


def enrich(raw):
    if not isinstance(raw,dict): return None
    # Raw prompts, arguments, transcript paths and messages never cross Docker stdin.
    result={key:raw.get(key) for key in ('session_id','hook_event_name','tool_name','model')}
    result['observatory_usage']=read_usage(raw)
    return result


def main():
    if os.environ.get('HERDR_ENV')!='1' or not os.environ.get('HERDR_PANE_ID'): return
    try:
        data=sys.stdin.buffer.read(MAX_INPUT+1)
        if len(data)>MAX_INPUT: return
        item=enrich(json.loads(data))
        if item is None:return
        subprocess.run(['docker','exec','-i','__CONTAINER__','python3','-m','observatory.telemetry',
                        'codex',os.environ['HERDR_PANE_ID'],str(time.time_ns()//1000)],
                       input=json.dumps(item).encode(),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=1.5,check=False)
    except (OSError,ValueError,TypeError,RecursionError,subprocess.SubprocessError):
        pass


if __name__=='__main__': main()

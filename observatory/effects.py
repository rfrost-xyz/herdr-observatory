"""Bounded ttfx library adapter; input is exclusively the server's filtered snapshot."""
import hashlib
import io
import os
import re
import subprocess
import threading
import time


EFFECTS = ('decrypt', 'vhstape', 'crumble')

def lines_for(state, now, page=0, category='all', hold=10):
    hosts=[h for h in state['hosts'] if h['online'] and isinstance(h.get('sampled_at'),(int,float)) and now-h['sampled_at'] < state['interval']+20]
    agents=[a for h in hosts for a in h['agents']]
    visible=[a for a in agents if category=='all' or a.get('category')==category]
    rows=['']*36
    def put(index,text):
        rows[index]=''.join(c if 32<=ord(c)<127 else '?' for c in text)[:120]
    put(0,'+'+'-'*118+'+')
    put(1,f"| HERDR / {state.get('profile','work').upper()} / READ ONLY / CAPTURED {time.strftime('%H:%M:%S UTC',time.gmtime(now))} / theme:{(state.get('theme') or {}).get('name','default')}")
    def pct(v):return str(round(v))+'%' if isinstance(v,(int,float)) else '?'
    def ratio(v):return pct(v['used']/v['total']*100) if v and v.get('total') else '?'
    def rate(v):return str(round(v/1024))+'K/s' if isinstance(v,(int,float)) else '?'
    for i,h in enumerate(state['hosts'][:3]):
        live=h in hosts;m=(h.get('metrics') or {}) if live else {}
        put(2+i,f"| {h['id'][:15]:<15} {'ONLINE' if live else 'OFFLINE'} P{h.get('protocol','?')} cpu {pct(m.get('cpu_percent'))} ram {ratio(m.get('memory'))} disk {ratio(m.get('disk'))} gpu {pct((m.get('gpu') or {}).get('percent'))} rx {rate(m.get('rx_rate'))} tx {rate(m.get('tx_rate'))} / {m.get('scope','unavailable')}")
    put(5,f"+ THREADS / {sum(a['status']=='working' for a in agents)} working / {sum(a['status']=='blocked' for a in agents)} need input / {len(agents)} total "+'-'*80)
    put(6,'| STATE     HOST           ENGINE       PROJECT / THREAD')
    count=max(1,(len(visible)+11)//12);page%=count
    for i,a in enumerate(visible[page*12:(page+1)*12]):
        status='INPUT' if a['status']=='blocked' else a['status'].upper()
        put(7+i,f"| {status:<9} {a['host'][:14]:<14} {a['harness'][:12]:<12} {a['project']} / {a['title']}")
    put(19,f"| {len(agents)} threads / {category} / page {page+1}/{count}")
    put(26,'+ CLI FEED / SAMPLED OBSERVATIONS '+'-'*86+'+')
    for i,a in enumerate(visible[page*12:page*12+6]):
        t=a.get('technical') or {}
        put(27+i,f"| {a['host']}/{a['project']} {a['status'].upper()} rev={t.get('revision','?')} seq={t.get('state_change_seq','?')} / {a['title']}")
    put(33,'| observer@fleet:~$ follow')
    put(34,'+'+'-'*118+'+')
    put(35,f'LIVE / FX HOLD {hold}s [LEFT -1s / RIGHT +1s] / sample {state["interval"]}s')
    key=hashlib.sha256(repr([(h['id'],h['sampled_at']) for h in hosts]).encode()+repr((page,category,hold,state.get('theme'))).encode()).hexdigest()[:20]
    return key if hosts else '', '\n'.join(rows)


SGR = re.compile(r'\x1b\[(?:0|38;2;\d{1,3};\d{1,3};\d{1,3})m')

def parse_frames(raw):
    if len(raw)>128*1024*1024:raise ValueError('oversized frames')
    stream=io.BytesIO(raw);frames=[]
    while length:=stream.readline(12):
        if length==b'END\n':
            if stream.read(1) or not frames:raise ValueError('invalid completion')
            return frames
        size=int(length)
        if not 0<=size<=262144 or len(frames)>=5000:raise ValueError('invalid frame size')
        frame=stream.read(size)
        if len(frame)!=size or stream.read(1)!=b'\n':raise ValueError('truncated frame')
        decoded=frame.decode('utf-8');plain=SGR.sub('',decoded)
        if any(ord(c)<32 and c!='\n' for c in plain):raise ValueError('unexpected terminal control')
        if len(plain.splitlines())>36 or any(len(row)>120 for row in plain.splitlines()):raise ValueError('invalid frame dimensions')
        frames.append(decoded)
    raise ValueError('incomplete animation')


class TextEffects:
    def __init__(self):
        self.lock = threading.Lock()
        self.cached = None
        self.last_attempt = 0

    def snapshot(self, state, previous=None, page=0, category='all', hold=10):
        now = time.time()
        key, text = lines_for(state, now, page, category, hold)
        sources = [{'id': h['id'], 'at': h['sampled_at']} for h in state['hosts'] if h['online'] and isinstance(h.get('sampled_at'), (int,float)) and now-h['sampled_at'] < state['interval']+20]
        effect = EFFECTS[(EFFECTS.index(previous)+1)%len(EFFECTS)] if previous in EFFECTS else EFFECTS[0]
        key = key + ':' + effect if key else ''
        empty = {'key': key, 'text': text, 'frames': [], 'effect': 'plain', 'fps': 30, 'sources': sources}
        if not key:
            return empty
        with self.lock:
            if self.cached and self.cached['key'] == key:
                return self.cached
            if now-self.last_attempt < 1:
                return empty
            self.last_attempt = now
            colours=(state.get('theme') or {}).get('colours') or {}
            palette=[colours.get(k, fallback).lstrip('#') for k,fallback in [('accent','7aa2f7'),('foreground','c0caf5'),('green','9ece6a'),('yellow','e0af68')]]
            try:
                result = subprocess.run([os.environ.get('OBSERVATORY_TEXT_RENDERER', '/usr/local/bin/observatory-text'), effect, *palette],
                    input=text.encode(), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=15, check=True)
                frames = parse_frames(result.stdout)
                value = dict(empty, frames=frames, effect=effect)
            except (OSError, subprocess.SubprocessError, ValueError, UnicodeError):
                value = empty
            self.cached = value
            return value

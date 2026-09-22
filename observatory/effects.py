"""Filtered, bounded terminal layout shared by live and animated views."""
import hashlib
import time

def lines_for(state, now, page=0, category='all', hold=120):
    hosts=[h for h in state['hosts'] if h['online'] and isinstance(h.get('sampled_at'),(int,float)) and now-h['sampled_at'] < state['interval']+20]
    agents=[a for h in hosts for a in h['agents']]
    visible=[a for a in agents if category=='all' or a.get('category')==category]
    rows=['']*44
    def put(index,text):
        rows[index]=''.join(c if 32<=ord(c)<127 else '?' for c in text)[:140]
    put(0,'+'+'-'*138+'+')
    put(1,f"| HERDR / {state.get('profile','work').upper()} / READ ONLY / CAPTURED {time.strftime('%H:%M:%S UTC',time.gmtime(now))} / theme:{(state.get('theme') or {}).get('name','default')}")
    def pct(v):return str(round(v))+'%' if isinstance(v,(int,float)) else '?'
    def ratio(v):return pct(v['used']/v['total']*100) if v and v.get('total') else '?'
    def rate(v):return str(round(v/1024))+'K/s' if isinstance(v,(int,float)) else '?'
    for i,h in enumerate(state['hosts'][:3]):
        live=h in hosts;m=(h.get('metrics') or {}) if live else {}
        put(2+i,f"| {h['id'][:15]:<15} {'ONLINE' if live else 'OFFLINE'} P{h.get('protocol','?')} cpu {pct(m.get('cpu_percent'))} ram {ratio(m.get('memory'))} disk {ratio(m.get('disk'))} gpu {pct((m.get('gpu') or {}).get('percent'))} rx {rate(m.get('rx_rate'))} tx {rate(m.get('tx_rate'))} / {m.get('scope','unavailable')}")
    put(5,f"+ THREADS / {sum(a['status']=='working' for a in agents)} working / {sum(a['status']=='blocked' for a in agents)} need input / {len(agents)} total "+'-'*80)
    put(6,'| STATE     HOST           ENGINE       PROJECT / THREAD')
    count=max(1,(len(visible)+7)//8);page%=count
    for i,a in enumerate(visible[page*8:(page+1)*8]):
        status=a['status'].capitalize()
        put(7+i,f"| {status:<9} {a['host'][:14]:<14} {a['harness'][:12]:<12} {a['project']} / {a['title']}")
    put(15,f"| {len(agents)} threads / {category} / page {page+1}/{count}")
    put(16,'+ NOTABLE EVENTS '+'-'*122+'+')
    put(18,'| State-change history is maintained by the browser; this snapshot is the current baseline.')
    put(41,'| observer@fleet:~$ follow')
    put(42,'+'+'-'*138+'+')
    put(43,f'LIVE / FX HOLD {hold}s [LEFT -1s / RIGHT +1s] / sample {state["interval"]}s')
    key=hashlib.sha256(repr([(h['id'],h['sampled_at']) for h in hosts]).encode()+repr((page,category,hold,state.get('theme'))).encode()).hexdigest()[:20]
    return key if hosts else '', '\n'.join(rows)

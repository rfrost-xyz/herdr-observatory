'use strict';
const canvas = document.getElementById('scene');
const ctx = canvas.getContext('2d');
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
let snapshot = null, received = 0, disconnected = false, initialised = false, paused = false;
let previous = new Map(), agents = [], records = [], serial = 0;
let feedQueue = [], sampleSeen = new Map(), lastEmission = 0;
let width = 0, height = 0, lastFrame = 0, manualPage = null, category = 'all';
let palette = {background:'#101318',foreground:'#c0caf5',blue:'#7aa2f7',green:'#9ece6a',yellow:'#e0af68',red:'#f7768e',cyan:'#7dcfff'};
const clean = (value, limit=160) => String(value ?? '—').replace(/[\x00-\x1f\x7f-\x9f]/g,' ').slice(0,limit);
const number = value => Number.isSafeInteger(value) && value >= 0 ? String(value) : '—';
const statusKind = status => ({working:'EXEC',done:'DONE',blocked:'INPUT',idle:'IDLE',unknown:'UNKNOWN'}[status] || 'UNKNOWN');
const colour = kind => palette[{EXEC:'cyan',DONE:'green',INPUT:'yellow',LOST:'red',DETACH:'yellow',LINK:'blue',ATTACH:'blue'}[kind] || 'foreground'];
function record(kind, text, now) {
  const entry = {id:++serial,kind,text:clean(text),at:now,born:performance.now()};
  records.push(entry); records = records.slice(-60);
}
function observe(data, now=Date.now()) {
  snapshot=data; received=now;
  const recovery=disconnected; disconnected=false;
  if (recovery) record('LINK','browser transport restored / baseline reacquired',now);
  for (const [key,value] of Object.entries(data.theme?.colours || {})) if ((key in palette || key==='accent') && /^#[0-9a-f]{6}$/i.test(value)) {palette[key]=value;if(key==='accent'){palette.blue=value;palette.cyan=value;}}
  reconcile(now,recovery);
  enqueueSamples(data,now);
}
// New collector captures provide a rolling feed even when agent status is unchanged.
// These are explicitly SAMPLE/PANE records, never invented tool calls or milestones.
function enqueueSamples(data, now) {
  for(const host of data.hosts) {
    if(!previous.get(host.id)?.live || sampleSeen.get(host.id)===host.sampled_at) continue;
    sampleSeen.set(host.id,host.sampled_at);
    feedQueue=feedQueue.filter(item=>item.host!==host.id);
    const m=host.metrics, percent=v=>Number.isFinite(v)?Math.round(v)+'%':'—';
    const rows=[{kind:'SAMPLE',text:`${host.id} / capture=${new Date(host.sampled_at*1000).toISOString().slice(11,19)} / panes=${host.agents.length} cpu=${percent(m?.cpu_percent)} ram=${m?.memory?.total?percent(m.memory.used/m.memory.total*100):'—'} / ${clean(m?.scope || 'metrics unavailable')}`}];
    for(const a of host.agents.slice(0,24)) rows.push({kind:'PANE',text:`${host.id}/${a.project} / ${statusKind(a.status)} / ${a.harness} rev=${number(a.technical?.revision)} seq=${number(a.technical?.state_change_seq)} / ${clean(a.title,60)}`});
    feedQueue.push(...rows.map(r=>({...r,host:host.id,captured:host.sampled_at,at:now})));
  }
  feedQueue=feedQueue.slice(-64);
}
function drainFeed(now=performance.now(), wall=Date.now()) {
  if(now-lastEmission<Math.max(160,Math.min(700,4000/Math.max(1,feedQueue.length))))return;
  while(feedQueue.length) {
    const item=feedQueue.shift();
    if(disconnected || !previous.get(item.host)?.live || wall/1000-item.captured>=snapshot.interval+20)continue;
    record(item.kind,item.text,item.at);lastEmission=now;accessible();break;
  }
}
function reconcile(now=Date.now(), reset=false) {
  if (!snapshot) return;
  const next = new Map(); agents=[];
  for (const host of snapshot.hosts) {
    const live=!disconnected && host.online && Number.isFinite(host.sampled_at) && now/1000-host.sampled_at < snapshot.interval+20;
    const old=previous.get(host.id);
    const current=new Map(live ? host.agents.map(a=>[a.id,{...a}]) : []);
    next.set(host.id,{live,agents:current});
    if (initialised && !reset && old && old.live!==live) record(live?'LINK':'LOST',`${host.label} / ${live?'source restored; baseline reacquired':'source unavailable; activity unknown'}`,now);
    if (initialised && !reset && live && old?.live) {
      for (const [id,a] of current) {
        const before=old.agents.get(id);
        if (!before) record('ATTACH',`${host.id}/${a.project} / ${a.harness} / ${statusKind(a.status)}`,now);
        else if (before.status!==a.status) record(statusKind(a.status),`${host.id}/${a.project} / ${clean(a.title,80)} / ${statusKind(before.status)} -> ${statusKind(a.status)}`,now);
      }
      for (const [id,a] of old.agents) if (!current.has(id)) record('DETACH',`${host.id}/${a.project} / pane no longer observed`,now);
    }
    if (live) agents.push(...current.values());
  }
  for (const [id,old] of previous) if (!next.has(id) && old.live && !reset) record('LOST',`${id} / source removed`,now);
  previous=next;
  if (!initialised) { record('BOOT',`observer attached / ${snapshot.profile} / ${agents.length} panes / baseline only`,now,false); initialised=true; }
  accessible();
}
function disconnect(now=Date.now()) {
  if (!disconnected) record('LOST','browser transport unavailable / activity unknown',now);
  disconnected=true; agents=[];feedQueue=[];
  // Preserve source baseline until transport recovery, without inventing per-pane exits.
  accessible();
}
function accessible() {
  document.getElementById('transcript').textContent=[`HERDR OBSERVATORY / ${snapshot?.profile || 'connecting'} / ${disconnected?'disconnected':'sampled observations'}`,...agents.map(a=>`${clean(a.host)}/${clean(a.project)} ${statusKind(a.status)} ${clean(a.title)}`),...(typeof textFrames!=='undefined' && framesCurrent()?[textFrames.text]:[]),...records.map(r=>`${new Date(r.at).toISOString()} ${r.kind} ${r.text}`)].join('\n');
}
function resize() {
  width=innerWidth; height=innerHeight;
  const ratio=Math.min(devicePixelRatio || 1,2);
  canvas.width=Math.round(width*ratio);canvas.height=Math.round(height*ratio);
  ctx?.setTransform(ratio,0,0,ratio,0,0);
  draw(performance.now());
}
function geometry(w,h) {
  const font=Math.max(10,Math.min(18,w/92,h/44));
  const line=font*1.5, margin=Math.max(16,w*.025);
  return {font,line,margin,columns:Math.floor((w-margin*2)/(font*.61)),processRows:Math.max(2,Math.min(12,Math.floor((h-margin*2)/line)-23)),logRows:6};
}
function filteredAgents(){return agents.filter(a=>category==='all'||snapshot?.profile==='work'||a.category===category);}
function currentPage(now=Date.now()) {return (manualPage ?? Math.floor(now/15000))%Math.max(1,Math.ceil(filteredAgents().length/geometry(width,height).processRows));}
let textFrames={key:'',frames:[],text:'Awaiting fresh source observations.',effect:'plain',fps:30}, framesBorn=0;
async function refreshFrames() {
  try {
    const response=await fetch('/api/text-frames',{cache:'no-store',signal:AbortSignal.timeout(5000)});
    if(!response.ok)throw new Error();
    const next=await response.json();
    if(next.key!==textFrames.key || next.effect!==textFrames.effect){textFrames=next;framesBorn=performance.now();accessible();}
  } catch {textFrames={key:'',frames:[],text:'Text renderer unavailable; current state remains above.',effect:'plain',fps:30};accessible();}
  setTimeout(refreshFrames,2000);
}
function framesCurrent(now=Date.now()) {return !disconnected && textFrames.sources?.length>0 && textFrames.sources.every(source=>previous.get(source.id)?.live && now/1000-source.at<snapshot.interval+20);}
function frameText(now) {
  if(!framesCurrent())return 'Source observations unavailable.';
  const index=Math.floor((now-framesBorn)/1000*(textFrames.fps || 30));
  return !paused && !reduced.matches && index>=0 && index<textFrames.frames.length?textFrames.frames[index]:textFrames.text;
}
function draw(now) {
  if(!ctx)return;
  const {font,line,margin,columns,processRows}=geometry(width,height);
  ctx.globalAlpha=1;ctx.fillStyle=palette.background;ctx.fillRect(0,0,width,height);
  ctx.font=`${font}px "DejaVu Sans Mono", "Cascadia Code", monospace`;ctx.textBaseline='top';
  const cell=ctx.measureText('M').width, cols=Math.min(columns,Math.floor((width-margin*2)/cell));
  function text(value,row,tint=palette.foreground){ctx.fillStyle=tint;ctx.fillText(clean(value,cols),margin,margin+row*line);}
  function rule(row,label){const title='─ '+label+' ';text('├'+title+'─'.repeat(Math.max(0,cols-title.length-2))+'┤',row,palette.blue);}
  text('┌'+'─'.repeat(Math.max(0,cols-2))+'┐',0,palette.blue);
  text(`│ HERDR  /  ${clean(snapshot?.profile || 'CONNECTING').toUpperCase()}   ${disconnected?'DISCONNECTED':'READ ONLY'}   ${new Date().toLocaleTimeString('en-GB')}   theme:${clean(snapshot?.theme?.name)}`,1,palette.cyan);
  const hosts=snapshot?.hosts || [];
  for(let i=0;i<3;i++){
    const host=hosts[i];if(!host)continue;
    const live=!disconnected && previous.get(host.id)?.live,m=live?host.metrics:null;
    const pct=v=>Number.isFinite(v)?Math.round(v)+'%':'—';
    const rate=v=>Number.isFinite(v)?Math.round(v/1024)+'K/s':'—';
    text(`│ ${clean(host.id,15).padEnd(15)} ${live?'ONLINE ':'OFFLINE'} P${number(host.protocol)} cpu ${pct(m?.cpu_percent).padEnd(4)} ram ${pct(m?.memory?.total?m.memory.used/m.memory.total*100:null).padEnd(4)} disk ${pct(m?.disk?.total?m.disk.used/m.disk.total*100:null)} gpu ${pct(m?.gpu?.percent).padEnd(4)} rx ${rate(m?.rx_rate)} tx ${rate(m?.tx_rate)}  ${Number.isFinite(host.sampled_at)?Math.max(0,Math.floor(Date.now()/1000-host.sampled_at))+'s':'—'} ${clean(m?.scope || 'unavailable',22)}`,2+i,live?palette.foreground:palette.red);
  }
  const working=agents.filter(a=>a.status==='working').length,blocked=agents.filter(a=>a.status==='blocked').length;
  rule(5,`THREADS / ${working} working / ${blocked} need input / ${agents.length} total`);
  text('│ STATE     HOST           ENGINE       PROJECT / THREAD',6,palette.blue);
  const page=currentPage(),shown=filteredAgents().slice(page*processRows,(page+1)*processRows);
  for(let i=0;i<shown.length;i++){
    const a=shown[i],status=a.status==='working'?'WORKING':statusKind(a.status);
    text(`│ ${status.padEnd(9)} ${clean(a.host,14).padEnd(14)} ${clean(a.harness,12).padEnd(12)} ${clean(a.project,24)} / ${clean(a.title)}`,7+i,colour(statusKind(a.status)));
  }
  if(!shown.length)text(disconnected?'│ Connection lost — current thread state unavailable.':'│ No permitted threads in this view.',7,palette.yellow);
  text(`│ ${category} / page ${filteredAgents().length?page+1:0}/${Math.ceil(filteredAgents().length/processRows)}  [← →] select + hold  [R] rotate  [C] category`,7+processRows,palette.blue);
  const feedRow=Math.floor((height-margin*2)/line)-10;
  const event=records.filter(r=>!['SAMPLE','PANE','BOOT'].includes(r.kind)).at(-1);
  if(event)text(`│ ${new Date(event.at).toLocaleTimeString('en-GB')} ${event.kind} ${event.text}`,feedRow-1,colour(event.kind));
  rule(feedRow,`CLI FEED / ttfx ${textFrames.effect} / sampled observations`);
  const valid=framesCurrent();
  const animate=!paused && !reduced.matches && valid;
  const output=frameText(now);
  const rows=String(output).split('\n');
  for(let i=0;i<6;i++)text('│ '+(rows[i] || ''),feedRow+1+i,palette.foreground);
  text(`│ observer@fleet:~$ follow${animate && Math.floor(now/550)%2?' ▌':''}`,feedRow+7,palette.green);
  text('└'+'─'.repeat(Math.max(0,cols-2))+'┘',feedRow+8,palette.blue);
  text(`${paused?'FX PAUSED':reduced.matches?'REDUCED MOTION':'TTFX TEXT ENGINE'} / sample interval ${snapshot?.interval || '—'}s / no shell execution`,feedRow+9,palette.foreground);
}
function frame(now) {if(!document.hidden && now-lastFrame>=33){drainFeed(now);draw(now);lastFrame=now;}requestAnimationFrame(frame);}
async function refresh() {
  try {const response=await fetch('/api/state',{cache:'no-store',signal:AbortSignal.timeout(8000)});if(!response.ok)throw new Error();observe(await response.json());}
  catch {disconnect();}
  setTimeout(refresh,2000);
}
function pause() {paused=!paused;document.getElementById('pause').setAttribute('aria-pressed',String(paused));document.getElementById('pause').textContent=paused?'[SPACE] resume FX':'[SPACE] pause FX';}
async function fullscreen(){try{if(document.fullscreenElement)await document.exitFullscreen();else await document.documentElement.requestFullscreen();}catch{record('INFO','Use browser fullscreen (F11)',Date.now(),false);}}
 document.getElementById('pause').addEventListener('click',pause);
document.getElementById('fullscreen').addEventListener('click',fullscreen);
addEventListener('keydown',event=>{if(event.target?.tagName==='BUTTON')return;if(event.code==='Space'){event.preventDefault();pause();}if(event.key==='f')fullscreen();if(event.key==='ArrowRight')manualPage=currentPage()+1;if(event.key==='ArrowLeft')manualPage=Math.max(0,currentPage()-1);if(event.key==='r')manualPage=null;if(event.key==='c' && snapshot?.profile==='personal'){category=['all','work','personal'][(['all','work','personal'].indexOf(category)+1)%3];manualPage=0;}});
addEventListener('resize',resize);
setInterval(()=>{if(received && Date.now()-received>12000)disconnect();if(!disconnected)reconcile();},1000);
resize();requestAnimationFrame(frame);refresh();refreshFrames();

'use strict';
const canvas = document.getElementById('scene');
const ctx = canvas.getContext('2d');
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
let snapshot = null, received = 0, disconnected = false, initialised = false, paused = false;
let previous = new Map(), agents = [], records = [], serial = 0;
let feedQueue = [], sampleSeen = new Map(), lastEmission = 0;
let scrollStarted=0, scrollDistance=0, stampCandidate=null, activeStamp=null, lastStamp=-Infinity;
const GRID={columns:140,rows:44,feedStart:31,feedRows:10};
function scrollOffset(now=performance.now()) {return paused || reduced.matches ? 0 : scrollDistance*Math.max(0,Math.min(1,1-(now-scrollStarted)/260));}
function stampCurrent(entry) {return entry && !disconnected && previous.get(entry.host)?.live && previous.get(entry.host).agents.get(entry.agent)?.status===entry.status;}
function updateStamp(now=performance.now()) {
  if(activeStamp && (!stampCurrent(activeStamp) || now-activeStamp.shown>=6000))activeStamp=null;
  if(stampCandidate && (!stampCurrent(stampCandidate) || now-stampCandidate.born>=30000))stampCandidate=null;
  if(!activeStamp && stampCandidate && !paused && !reduced.matches && now-lastStamp>=30000){activeStamp={...stampCandidate,shown:now};stampCandidate=null;lastStamp=now;}
  return activeStamp;
}
let width = 0, height = 0, lastFrame = 0, manualPage = null, category = 'all';
let palette = {background:'#101318',foreground:'#c0caf5',blue:'#7aa2f7',green:'#9ece6a',yellow:'#e0af68',red:'#f7768e',cyan:'#7dcfff'};
const clean = (value, limit=160) => String(value ?? '—').replace(/[\x00-\x1f\x7f-\x9f]/g,' ').slice(0,limit);
const number = value => Number.isSafeInteger(value) && value >= 0 ? String(value) : '—';
const statusKind = status => ({working:'EXEC',done:'DONE',blocked:'INPUT',idle:'IDLE',unknown:'UNKNOWN'}[status] || 'UNKNOWN');
const colour = kind => palette[{EXEC:'cyan',DONE:'green',INPUT:'yellow',LOST:'red',DETACH:'yellow',LINK:'blue',ATTACH:'blue'}[kind] || 'foreground'];
function record(kind, text, now) {
  const entry = {id:++serial,kind,text:clean(text),at:now,born:performance.now()};
  scrollDistance=Math.min(10,scrollOffset(entry.born)+1);scrollStarted=entry.born;
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
        else if (before.status!==a.status) {
          record(statusKind(a.status),`${host.id}/${a.project} / ${statusKind(before.status)} -> ${statusKind(a.status)} / ${clean(a.title,72)}`,now);
          if(['blocked','done'].includes(a.status))stampCandidate={...records.at(-1),host:host.id,agent:id,status:a.status};
        } else {
          const oldSeq=before.technical?.state_change_seq,newSeq=a.technical?.state_change_seq;
          if(Number.isSafeInteger(oldSeq) && Number.isSafeInteger(newSeq) && newSeq!==oldSeq)
            record('STATESEQ',`${host.id}/${a.project} / seq ${oldSeq} -> ${newSeq} / intermediate states not sampled`,now);
          else if(Number.isSafeInteger(before.technical?.revision) && Number.isSafeInteger(a.technical?.revision) && before.technical.revision!==a.technical.revision)
            record('UPDATE',`${host.id}/${a.project} / rev ${before.technical.revision} -> ${a.technical.revision}`,now);
          for(const key of ['interactive_ready','launch_pending'])if(typeof before.technical?.[key]==='boolean' && typeof a.technical?.[key]==='boolean' && before.technical[key]!==a.technical[key])
            record(key==='interactive_ready'?'READY':'LAUNCH',`${host.id}/${a.project} / ${key} ${before.technical[key]} -> ${a.technical[key]}`,now);
        }
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
  document.getElementById('transcript').textContent=[`HERDR OBSERVATORY / ${snapshot?.profile || 'connecting'} / ${disconnected?'disconnected':'sampled observations'}`,...agents.map(a=>`${clean(a.host)}/${clean(a.project)} ${statusKind(a.status)} ${clean(a.title)}`),...(typeof textFrames!=='undefined' && animationIndex>=0 && framesCurrent()?['Animated capture: '+textFrames.text]:[]),...records.map(r=>`${new Date(r.at).toISOString()} ${r.kind} ${r.text}`)].join('\n');
}
function resize() {
  width=innerWidth; height=innerHeight;
  const ratio=Math.min(devicePixelRatio || 1,2);
  canvas.width=Math.round(width*ratio);canvas.height=Math.round(height*ratio);
  ctx?.setTransform(ratio,0,0,ratio,0,0);
  draw(performance.now());
}
function geometry(w,h) {
  const margin=Math.max(20,Math.min(40,w*.025)),top=16,cell=Math.max(1,(w-margin*2)/GRID.columns),line=Math.max(1,(h-top*2)/GRID.rows);
  const font=Math.min(cell/.61,line*.85);
  return {font,cell,line,margin,top,columns:GRID.columns,processRows:12,logRows:GRID.feedRows};
}
function filteredAgents(){return agents.filter(a=>category==='all'||snapshot?.profile==='work'||a.category===category);}
function currentPage(now=Date.now()) {return (manualPage ?? Math.floor(now/15000))%Math.max(1,Math.ceil(filteredAgents().length/geometry(width,height).processRows));}
let textFrames={text:'',sources:[]}, effectSession=null, effectFrame=null, effectLibrary=null, effectBag=[];
let holdSeconds=10, holdElapsed=0, lastEffect='', animationIndex=-1, frameElapsed=0, loadingFrames=false;
function framesCurrent() {return !disconnected && textFrames.sources?.length>0 && textFrames.sources.every(source=>previous.get(source.id)?.live);}
function stopEffect() {effectSession?.free();effectSession=null;effectFrame=null;animationIndex=-1;holdElapsed=0;}
async function refreshFrames() {
  if(loadingFrames || animationIndex>=0 || paused || reduced.matches || disconnected || !snapshot?.terminal_text || !snapshot.hosts.some(h=>previous.get(h.id)?.live))return;
  loadingFrames=true;
  try {
    effectLibrary ??= await import('./effects.mjs').then(module=>module.loadEffects().then(lib=>({...lib,next:module.nextEffect})));
    if(paused || reduced.matches || disconnected)return;
    const effect=effectLibrary.next(effectLibrary.catalogue,effectBag,lastEffect);
    textFrames={text:sceneRows(performance.now(),false).join('\n'),sources:snapshot.hosts.filter(h=>previous.get(h.id)?.live).map(h=>({id:h.id}))};
    if(!framesCurrent())return;
    effectSession=effectLibrary.create(textFrames.text,effect,{...palette});
    effectFrame=effectSession.next();
    if(!effectFrame){stopEffect();return;}
    animationIndex=0;frameElapsed=0;lastEffect=effect;accessible();
  } catch {stopEffect(); /* Keep the live terminal readable on renderer failure. */}
  finally {loadingFrames=false;holdElapsed=0;}
}
function advanceEffects(delta) {
  if(disconnected || (animationIndex>=0 && !framesCurrent())){stopEffect();return;}
  if(paused || reduced.matches)return;
  if(animationIndex>=0){
    frameElapsed+=delta;
    if(frameElapsed>=1000/30){
      frameElapsed-=1000/30;
      try {const next=effectSession.next();if(next){effectFrame=next;animationIndex++;}else stopEffect();}
      catch {stopEffect();}
    }
  }else if(!loadingFrames){holdElapsed+=delta;if(holdElapsed>=holdSeconds*1000)refreshFrames();}
}
function adjustHold(change){holdSeconds=Math.max(0,Math.min(300,holdSeconds+change));}
function themedColour(value, fallback) {
  if(!(value>>>24))return fallback;
  const rgb=[(value>>>16)&255,(value>>>8)&255,value&255];
  // Keep native palette colours; map fixed upstream accents into the active theme.
  const colours=[palette.blue,palette.cyan,palette.green,palette.yellow,palette.red,palette.foreground];
  const candidates=colours.map(hex=>[1,3,5].map(i=>parseInt(hex.slice(i,i+2),16)));
  const intensity=Math.max(...rgb)/255;
  const norm=v=>{const max=Math.max(...v)||1;return v.map(c=>c/max);};
  const target=norm(rgb);
  let best=0,score=Infinity;
  candidates.forEach((v,i)=>{const distance=norm(v).reduce((sum,c,j)=>sum+(c-target[j])**2,0);if(distance<score){score=distance;best=i;}});
  if(score<0.0001)return `rgb(${rgb.join(',')})`;
  return `rgb(${candidates[best].map(c=>Math.round(c*intensity)).join(',')})`;
}
function paintEffect(frame,x,y,cell,line) {
  if(!frame)return;
  for(let i=0;i<frame.symbols.length;i++){
    const flag=frame.flags[i],symbol=frame.symbols[i];
    if(flag&32)continue;
    const px=x+(i%frame.width)*cell,py=y+Math.floor(i/frame.width)*line;
    let fg=themedColour(frame.fg[i],palette.foreground),bg=themedColour(frame.bg[i],palette.background);
    if(flag&16)[fg,bg]=[bg,fg];
    if(bg!==palette.background){ctx.fillStyle=bg;ctx.fillRect(px,py,cell,line);}
    if(symbol && symbol!==32){ctx.globalAlpha=flag&2?.55:1;ctx.fillStyle=fg;ctx.fillText(String.fromCodePoint(symbol),px,py,cell);}
    if(flag&8){ctx.fillStyle=fg;ctx.fillRect(px,py+line*.85,cell,1);}
  }
  ctx.globalAlpha=1;
}
function eventLine(entry) {return `${new Date(entry.at).toISOString().slice(11,23)}Z [${entry.kind}] ${entry.text}`;}
function sceneRows(now=performance.now(),showArt=true) {
  const rows=Array(GRID.rows).fill('');
  const put=(row,value)=>{rows[row]=clean(value,GRID.columns);};
  put(0,'+'+'-'.repeat(GRID.columns-2)+'+');
  put(1,`| HERDR / ${snapshot?.profile || 'connecting'} / ${disconnected?'DISCONNECTED':'READ ONLY'} / OBSERVED ${new Date().toISOString().slice(11,19)}Z / theme:${snapshot?.theme?.name || 'default'}`);
  for(const [i,h] of (snapshot?.hosts || []).slice(0,3).entries()){
    const live=previous.get(h.id)?.live && !disconnected,m=live?h.metrics:null,pct=v=>Number.isFinite(v)?Math.round(v)+'%':'?';
    put(2+i,`| ${clean(h.id,15).padEnd(15)} ${live?'ONLINE':'OFFLINE'} P${number(h.protocol)} cpu ${pct(m?.cpu_percent)} ram ${pct(m?.memory?.total?m.memory.used/m.memory.total*100:null)} gpu ${pct(m?.gpu?.percent)} disk ${pct(m?.disk?.total?m.disk.used/m.disk.total*100:null)} rx ${Number.isFinite(m?.rx_rate)?Math.round(m.rx_rate/1024)+'K/s':'?'} tx ${Number.isFinite(m?.tx_rate)?Math.round(m.tx_rate/1024)+'K/s':'?'} / ${live?'capture '+Math.max(0,Math.floor(Date.now()/1000-h.sampled_at))+'s ago':'activity unknown'} / ${m?.scope || 'unavailable'}`);
  }
  put(5,`+ THREADS / ${agents.filter(a=>a.status==='working').length} working / ${agents.filter(a=>a.status==='blocked').length} need input / ${agents.length} total `+'-'.repeat(GRID.columns));
  put(6,'| STATE     HOST           ENGINE       READY LAUNCH FOCUS  REV / SEQ       PROJECT / THREAD');
  const shown=filteredAgents().slice(currentPage()*12,currentPage()*12+12),flag=v=>v===true?'yes':v===false?'no ':' ? ';
  shown.forEach((a,i)=>{const t=a.technical || {};put(7+i,`| ${statusKind(a.status).padEnd(9)} ${clean(a.host,14).padEnd(14)} ${clean(a.harness,12).padEnd(12)} ${flag(t.interactive_ready)}   ${flag(t.launch_pending)}   ${flag(t.focused)}   ${(number(t.revision)+'/'+number(t.state_change_seq)).padEnd(15)} ${a.project} / ${a.title}`);});
  if(!shown.length)put(7,'| '+(disconnected?'Connection lost; current thread state unavailable.':'No permitted threads in this view.'));
  put(19,`| ${category} / page ${filteredAgents().length?currentPage()+1:0}/${Math.ceil(filteredAgents().length/12)} [PgUp PgDn] pages [R] rotate [C] category`);
  const stamp=showArt?updateStamp(now):null,art=stamp && typeof STAMP_ART!=='undefined'?STAMP_ART[stamp.kind]:null;
  if(art && !paused && !reduced.matches)art.forEach((row,i)=>put(20+i,'| '+row));
  else {
    put(21,'| EVENT REGISTER / observed transitions; no raw shell output');
    const milestones=records.filter(r=>!['SAMPLE','PANE','BOOT'].includes(r.kind)).slice(-5);
    milestones.forEach((r,i)=>put(23+i,'| '+eventLine(r)));
  }
  if(stamp)put(29,'| '+eventLine(stamp));
  put(30,'+ CLI FEED / TIMESTAMPED SAMPLED OBSERVATIONS '+'-'.repeat(GRID.columns));
  records.slice(-GRID.feedRows).forEach((r,i)=>put(GRID.feedStart+GRID.feedRows-Math.min(records.length,GRID.feedRows)+i,'| '+eventLine(r)));
  put(41,'| observer@fleet:~$ follow');
  put(42,'+'+'-'.repeat(GRID.columns-2)+'+');
  put(43,`${paused?'FX PAUSED':reduced.matches?'REDUCED MOTION':'LIVE'} / FX HOLD ${holdSeconds}s [LEFT -1s / RIGHT +1s] / sample ${snapshot?.interval || '?'}s / observations, not a complete event stream`);
  return rows;
}
function draw(now=performance.now()) {
  if(!ctx)return;
  const {font,cell,line,margin,top}=geometry(width,height);
  ctx.globalAlpha=1;ctx.fillStyle=palette.background;ctx.fillRect(0,0,width,height);
  ctx.font=`${font}px "DejaVu Sans Mono", "Cascadia Code", monospace`;ctx.textBaseline='top';
  const playing=animationIndex>=0 && framesCurrent() && !reduced.matches;
  document.getElementById('controls').hidden=playing;
  if(playing){paintEffect(effectFrame,margin,top,cell,line);return;}
  const rows=sceneRows(now);
  function text(value,row,tint=palette.foreground){ctx.fillStyle=tint;let col=0;for(const glyph of clean(value,GRID.columns)){ctx.fillText(glyph,margin+col*cell,top+row*line,cell);col++;}}
  rows.forEach((row,i)=>{if(i<GRID.feedStart || i>=GRID.feedStart+GRID.feedRows)text(row,i,/INPUT|OFFLINE/.test(row)?palette.yellow:/EXEC/.test(row)?palette.cyan:activeStamp && i>=20 && i<=28?colour(activeStamp.kind):palette.foreground);});
  ctx.save();ctx.beginPath();ctx.rect(margin,top+GRID.feedStart*line,width-margin*2,GRID.feedRows*line);ctx.clip();
  const feed=records.slice(-(GRID.feedRows+1)),start=GRID.feedStart+GRID.feedRows-feed.length,offset=scrollOffset(now);
  feed.forEach((r,i)=>text('| '+eventLine(r),start+i+offset,colour(r.kind)));
  ctx.restore();
}
function frame(now) {if(!document.hidden && now-lastFrame>=33){drainFeed(now);advanceEffects(Math.min(100,now-lastFrame));draw();lastFrame=now;}if(document.hidden)lastFrame=now;requestAnimationFrame(frame);}
async function refresh() {
  try {const response=await fetch(`/api/state?page=${currentPage()}&category=${category}&hold=${holdSeconds}`,{cache:'no-store',signal:AbortSignal.timeout(8000)});if(!response.ok)throw new Error();observe(await response.json());}
  catch {disconnect();}
  setTimeout(refresh,2000);
}
function pause() {paused=!paused;document.getElementById('pause').setAttribute('aria-pressed',String(paused));document.getElementById('pause').textContent=paused?'[SPACE] resume FX':'[SPACE] pause FX';}
async function fullscreen(){try{if(document.fullscreenElement)await document.exitFullscreen();else await document.documentElement.requestFullscreen();}catch{record('INFO','Use browser fullscreen (F11)',Date.now(),false);}}
 document.getElementById('pause').addEventListener('click',pause);
document.getElementById('fullscreen').addEventListener('click',fullscreen);
addEventListener('keydown',event=>{if(event.target?.tagName==='BUTTON' && event.code==='Space')return;if(event.code==='Space'){event.preventDefault();pause();}if(event.key==='f')fullscreen();if(event.key==='ArrowRight'){event.preventDefault();adjustHold(1);}if(event.key==='ArrowLeft'){event.preventDefault();adjustHold(-1);}if(event.key==='PageDown')manualPage=currentPage()+1;if(event.key==='PageUp')manualPage=Math.max(0,currentPage()-1);if(event.key==='r')manualPage=null;if(event.key==='c' && snapshot?.profile==='personal'){category=['all','work','personal'][(['all','work','personal'].indexOf(category)+1)%3];manualPage=0;}});
addEventListener('resize',resize);
setInterval(()=>{if(received && Date.now()-received>12000)disconnect();if(!disconnected)reconcile();},1000);
resize();requestAnimationFrame(frame);refresh();

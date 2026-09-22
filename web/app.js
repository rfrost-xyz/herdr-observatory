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
const clean = (value, limit=160) => Array.from(String(value ?? '—').replace(/[\x00-\x1f\x7f-\x9f]/g,' ')).slice(0,limit).join('');
const number = value => Number.isSafeInteger(value) && value >= 0 ? String(value) : '—';
const statusKind = status => ({working:'EXEC',done:'DONE',blocked:'INPUT',idle:'IDLE',unknown:'UNKNOWN'}[status] || 'UNKNOWN');
const colour = kind => palette[{EXEC:'cyan',DONE:'green',INPUT:'yellow',LOST:'red',DETACH:'yellow',LINK:'blue',ATTACH:'blue'}[kind] || 'foreground'];
const FONT_FACE='"Observatory Nerd", "JetBrainsMono Nerd Font", monospace';
let nerdFontReady=false;
const GLYPHS={terminal:'\uea85',host:'\uf233',threads:'\uf126',feed:'\uf0ca',clock:'\uf017',branch:'\ue0a0',working:'\uf04b',done:'\uf00c',blocked:'\uf071',idle:'\uf04c',unknown:'\uf128'};
function icon(name) {return nerdFontReady?GLYPHS[name] || '·':({terminal:'›',host:'◇',threads:'≡',feed:'≡',clock:'◷',branch:'⑂',working:'▶',done:'✓',blocked:'!',idle:'Ⅱ',unknown:'?'}[name] || '·');}
async function loadFont() {
  try {const loaded=await document.fonts?.load('16px "Observatory Nerd"');nerdFontReady=Boolean(loaded?.length);}
  catch {nerdFontReady=false;}
  draw();
}
function padded(value,length) {const text=clean(value,length);return text+' '.repeat(Math.max(0,length-Array.from(text).length));}
function fields(entries) {let row='';for(const [column,value] of entries){row=padded(row,column)+value;}return clean(row,GRID.columns);}
function meter(value) {if(!Number.isFinite(value))return '░░░░░░   ?';const n=Math.round(Math.max(0,Math.min(100,value))/100*6);return '▰'.repeat(n)+'▱'.repeat(6-n)+' '+padded(Math.round(value)+'%',4);}
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
function eventLine(entry) {const state={EXEC:'working',DONE:'done',INPUT:'blocked',IDLE:'idle'}[entry.kind];return `${new Date(entry.at).toISOString().slice(11,23)}  ${icon(state || 'feed')} ${padded(entry.kind,8)} ${entry.text}`;}
function sceneRows(now=performance.now(),showArt=true) {
  const rows=Array(GRID.rows).fill('');
  const put=(row,value)=>{rows[row]=clean(value,GRID.columns);};
  const rule=label=>'╭─ '+label+' '+'─'.repeat(Math.max(0,GRID.columns-Array.from(label).length-5))+'╮';
  put(0,` ${icon('terminal')}  herdr observatory  ${nerdFontReady?'':'›'}  ${snapshot?.profile || 'connecting'}                                 ${disconnected?'○ disconnected':'● observing'}  ·  ${new Date().toISOString().slice(11,19)} UTC`);
  for(const [i,h] of (snapshot?.hosts || []).slice(0,3).entries()){
    const live=previous.get(h.id)?.live && !disconnected,m=live?h.metrics:null;
    const pct=v=>Number.isFinite(v)?Math.round(v)+'%':'?';
    put(2+i,fields([[1,icon('host')+'  '+padded(h.id,15)],[20,live?'● online':'○ offline'],[31,'cpu '+meter(m?.cpu_percent)],[49,'ram '+meter(m?.memory?.total?m.memory.used/m.memory.total*100:null)],[67,'gpu '+pct(m?.gpu?.percent)],[76,'disk '+pct(m?.disk?.total?m.disk.used/m.disk.total*100:null)],[86,'↓'+(Number.isFinite(m?.rx_rate)?Math.round(m.rx_rate/1024)+'K/s':'?')],[96,'↑'+(Number.isFinite(m?.tx_rate)?Math.round(m.tx_rate/1024)+'K/s':'?')],[106,live?Math.max(0,Math.floor(Date.now()/1000-h.sampled_at))+'s':'unknown'],[114,'p'+number(h.protocol)],[120,clean(m?.scope || 'unavailable',20)]]));
  }
  put(5,rule(`${icon('threads')} Threads  ·  ${agents.filter(a=>a.status==='working').length} running  ·  ${agents.filter(a=>a.status==='blocked').length} need input  ·  ${agents.length} total`));
  put(6,fields([[5,'State'],[15,'Project'],[35,'Thread'],[77,'Engine'],[92,'Host'],[108,'R L F'],[118,'Rev / Seq']]));
  const shown=filteredAgents().slice(currentPage()*12,currentPage()*12+12),flag=v=>v===true?'●':v===false?'·':'?';
  shown.forEach((a,i)=>{const t=a.technical || {};put(7+i,fields([[2,icon(a.status)],[5,padded(statusKind(a.status),8)],[15,padded(a.project,18)],[35,padded(a.title,40)],[77,padded(a.harness,12)],[92,padded(a.host,14)],[108,`${flag(t.interactive_ready)} ${flag(t.launch_pending)} ${flag(t.focused)}`],[118,number(t.revision)+' / '+number(t.state_change_seq)]]));});
  if(!shown.length)put(7,'  '+(disconnected?'Connection lost · current thread state unavailable':'No permitted threads in this view'));
  put(19,`╰─ ${category}  ·  ${filteredAgents().length?currentPage()+1:0}/${Math.ceil(filteredAgents().length/12)}  ·  PgUp/PgDn page   r rotate   c category   ${'─'.repeat(8)}   R ready · L launching · F focused`);
  const stamp=showArt?updateStamp(now):null,art=stamp && typeof STAMP_ART!=='undefined'?STAMP_ART[stamp.kind]:null;
  if(art && !paused && !reduced.matches)art.forEach((row,i)=>put(20+i,'   '+row));
  else {
    put(21,`  ${icon('branch')} Recent transitions`);
    const milestones=records.filter(r=>!['SAMPLE','PANE','BOOT'].includes(r.kind)).slice(-5);
    milestones.forEach((r,i)=>put(23+i,'  '+eventLine(r)));
    if(!milestones.length)put(24,'    Watching for state changes · baseline captured');
  }
  if(stamp)put(29,'  '+eventLine(stamp));
  put(30,rule(`${icon('feed')} Activity stream  ·  sampled observations`));
  records.slice(-GRID.feedRows).forEach((r,i)=>put(GRID.feedStart+GRID.feedRows-Math.min(records.length,GRID.feedRows)+i,'  '+eventLine(r)));
  put(41,'╰'+'─'.repeat(GRID.columns-2)+'╯');
  put(42,` ${icon('terminal')}  follow  ${nerdFontReady?'':'›'}  ${paused?'paused':reduced.matches?'reduced motion':'live'}  ·  ${snapshot?.theme?.name || 'default'}                                  ${icon('clock')}  FX hold ${holdSeconds}s   ← / → adjust`);
  put(43,`  Read only · ${snapshot?.interval || '?'}s samples · intermediate transitions may be missed`);
  return rows;
}
function draw(now=performance.now()) {
  if(!ctx)return;
  const {font,cell,line,margin,top}=geometry(width,height);
  ctx.globalAlpha=1;ctx.fillStyle=palette.background;ctx.fillRect(0,0,width,height);
  ctx.font=`${font}px ${FONT_FACE}`;ctx.textBaseline='top';
  const playing=animationIndex>=0 && framesCurrent() && !reduced.matches;
  document.getElementById('controls').hidden=playing;
  if(playing){paintEffect(effectFrame,margin,top,cell,line);return;}
  const rows=sceneRows(now);
  function text(value,row,tint=palette.foreground,from=0,to=GRID.columns){let col=0;for(const glyph of clean(value,GRID.columns)){if(col>=from && col<to){ctx.fillStyle=tint;ctx.fillText(glyph,margin+col*cell,top+row*line,cell);}col++;}}
  function band(row,alpha,tint=palette.foreground){ctx.globalAlpha=alpha;ctx.fillStyle=tint;ctx.fillRect(margin,top+row*line,width-margin*2,line);ctx.globalAlpha=1;}
  for(const row of [0,42])band(row,.13,palette.blue);
  band(6,.055);
  rows.forEach((row,i)=>{
    if(i>=GRID.feedStart && i<GRID.feedStart+GRID.feedRows)return;
    if(i>=7 && i<=18 && row){
      const state=filteredAgents()[currentPage()*12+i-7]?.status;
      if(state){const tint=colour(statusKind(state));band(i,state==='blocked'?.09:i%2?.025:.045,state==='blocked'?palette.yellow:palette.foreground);text(row,i,palette.foreground,35,75);text(row,i,tint,0,14);text(row,i,palette.blue,15,33);ctx.globalAlpha=.65;text(row,i,palette.foreground,77);ctx.globalAlpha=1;return;}
    }
    const muted=[6,19,43].includes(i);
    ctx.globalAlpha=muted?.58:1;
    text(row,i,[0,5,21,30,42].includes(i)?palette.blue:activeStamp && i>=20 && i<=28?colour(activeStamp.kind):palette.foreground);
    ctx.globalAlpha=1;
  });
  ctx.save();ctx.beginPath();ctx.rect(margin,top+GRID.feedStart*line,width-margin*2,GRID.feedRows*line);ctx.clip();
  const feed=records.slice(-(GRID.feedRows+1)),start=GRID.feedStart+GRID.feedRows-feed.length,offset=scrollOffset(now);
  feed.forEach((r,i)=>text('  '+eventLine(r),start+i+offset,colour(r.kind)));
  ctx.restore();
}
function frame(now) {if(!document.hidden && now-lastFrame>=33){drainFeed(now);advanceEffects(Math.min(100,now-lastFrame));draw();lastFrame=now;}if(document.hidden)lastFrame=now;requestAnimationFrame(frame);}
async function refresh() {
  try {const response=await fetch(`/api/state?page=${currentPage()}&category=${category}&hold=${holdSeconds}`,{cache:'no-store',signal:AbortSignal.timeout(8000)});if(!response.ok)throw new Error();observe(await response.json());}
  catch {disconnect();}
  setTimeout(refresh,2000);
}
function pause() {paused=!paused;document.getElementById('pause').setAttribute('aria-pressed',String(paused));document.getElementById('pause').textContent=paused?'Space resume':'Space pause';}
async function fullscreen(){try{if(document.fullscreenElement)await document.exitFullscreen();else await document.documentElement.requestFullscreen();}catch{record('INFO','Use browser fullscreen (F11)',Date.now(),false);}}
 document.getElementById('pause').addEventListener('click',pause);
document.getElementById('fullscreen').addEventListener('click',fullscreen);
addEventListener('keydown',event=>{if(event.target?.tagName==='BUTTON' && event.code==='Space')return;if(event.code==='Space'){event.preventDefault();pause();}if(event.key==='f')fullscreen();if(event.key==='ArrowRight'){event.preventDefault();adjustHold(1);}if(event.key==='ArrowLeft'){event.preventDefault();adjustHold(-1);}if(event.key==='PageDown')manualPage=currentPage()+1;if(event.key==='PageUp')manualPage=Math.max(0,currentPage()-1);if(event.key==='r')manualPage=null;if(event.key==='c' && snapshot?.profile==='personal'){category=['all','work','personal'][(['all','work','personal'].indexOf(category)+1)%3];manualPage=0;}});
addEventListener('resize',resize);
setInterval(()=>{if(received && Date.now()-received>12000)disconnect();if(!disconnected)reconcile();},1000);
resize();loadFont();requestAnimationFrame(frame);refresh();

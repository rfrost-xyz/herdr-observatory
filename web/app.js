'use strict';
const canvas = document.getElementById('scene');
const ctx = canvas.getContext('2d');
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
let snapshot = null, received = 0, disconnected = false, initialised = false, paused = false;
let previous = new Map(), agents = [], records = [], impacts = [], serial = 0;
let feedQueue = [], sampleSeen = new Map(), lastEmission = 0, lastFeed = -Infinity;
let width = 0, height = 0, lastFrame = 0, manualPage = null, category = 'all';
let palette = {background:'#101318',foreground:'#c0caf5',blue:'#7aa2f7',green:'#9ece6a',yellow:'#e0af68',red:'#f7768e',cyan:'#7dcfff'};
const clean = (value, limit=160) => String(value ?? '—').replace(/[\x00-\x1f\x7f-\x9f]/g,' ').slice(0,limit);
const number = value => Number.isSafeInteger(value) && value >= 0 ? String(value) : '—';
const statusKind = status => ({working:'EXEC',done:'DONE',blocked:'INPUT',idle:'IDLE',unknown:'UNKNOWN'}[status] || 'UNKNOWN');
const colour = kind => palette[{EXEC:'cyan',DONE:'green',INPUT:'yellow',LOST:'red',DETACH:'yellow',LINK:'blue',ATTACH:'blue'}[kind] || 'foreground'];
function record(kind, text, now, react=true, strength=1) {
  const entry = {id:++serial,kind,text:clean(text),at:now,born:performance.now(),strength};
  records.push(entry); records = records.slice(-60);
  if (react && !paused && !reduced.matches) { impacts.push({...entry,x:.25+(serial*0.173)%0.5,y:.25+(serial*0.117)%0.45}); impacts=impacts.slice(-8); }
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
    record(item.kind,item.text,item.at,true,.45);lastEmission=now;lastFeed=now;accessible();break;
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
  disconnected=true; agents=[];feedQueue=[];lastFeed=-Infinity;impacts=impacts.filter(i=>i.kind==='LOST');
  // Preserve source baseline until transport recovery, without inventing per-pane exits.
  accessible();
}
function accessible() {
  document.getElementById('transcript').textContent=[`HERDR OBSERVATORY / ${snapshot?.profile || 'connecting'} / ${disconnected?'disconnected':'sampled observations'}`,...agents.map(a=>`${clean(a.host)}/${clean(a.project)} ${statusKind(a.status)} ${clean(a.title)}`),...records.map(r=>`${new Date(r.at).toISOString()} ${r.kind} ${r.text}`)].join('\n');
}
function resize() {
  width=innerWidth; height=innerHeight;
  const ratio=Math.min(devicePixelRatio || 1,2);
  canvas.width=Math.round(width*ratio);canvas.height=Math.round(height*ratio);
  ctx?.setTransform(ratio,0,0,ratio,0,0);
  draw(performance.now());
}
function geometry(w,h) {
  const font=Math.max(10,Math.min(17,w/98,h/45));
  const line=font*1.65, margin=Math.max(20,w*.035);
  return {font,line,margin,columns:Math.floor((w-margin*2)/(font*.61)),processRows:Math.max(2,Math.min(9,Math.floor(h/line*.18))),logRows:Math.max(2,Math.floor(h/line*.22))};
}
function filteredAgents(){return agents.filter(a=>category==='all'||snapshot?.profile==='work'||a.category===category);}
function currentPage(now=Date.now()) {return (manualPage ?? Math.floor(now/15000))%Math.max(1,Math.ceil(filteredAgents().length/geometry(width,height).processRows));}
function draw(now) {
  if (!ctx) return;
  const {font,line,margin,columns,processRows,logRows}=geometry(width,height);
  const motion=!paused && !reduced.matches;
  impacts=impacts.filter(i=>now-i.born<2300);
  const active=motion?impacts:[];
  const feedEnergy=motion && !disconnected && agents.length ? Math.max(0,1-(now-lastFeed)/6500) : 0;
  ctx.fillStyle=palette.background;ctx.fillRect(0,0,width,height);
  // A spatial field, disturbed by observations. No synthetic activity counters.
  ctx.lineWidth=.6;
  for (let row=0;row<16;row++) {
    ctx.beginPath();
    for(let col=0;col<=40;col++) {
      let x=col*width/40,y=height*.22+row*height*.055+Math.sin(col*.35+now*.0012+row*.7)*feedEnergy*5;
      for(const i of active) {const age=(now-i.born)/2300,dist=Math.hypot(x-i.x*width,y-i.y*height);y+=Math.sin((dist-age*width*1.1)/35)*Math.exp(-Math.abs(dist-age*width*1.1)/110)*(1-age)*40*i.strength;}
      col?ctx.lineTo(x,y):ctx.moveTo(x,y);
    }
    ctx.strokeStyle=palette.blue;ctx.globalAlpha=.10+feedEnergy*.04;ctx.stroke();
  }
  for(const i of active) {
    const age=(now-i.born)/2300;
    ctx.globalAlpha=(1-age)*.42*i.strength;ctx.strokeStyle=colour(i.kind);ctx.lineWidth=1.5;
    ctx.beginPath();ctx.ellipse(i.x*width,i.y*height,Math.max(1,age*width),Math.max(1,age*width*.55),0,0,Math.PI*2);ctx.stroke();
    ctx.globalAlpha=(1-age)*.025*i.strength;ctx.fillStyle=colour(i.kind);ctx.fillRect(0,0,width,height);
  }
  if(active.some(i=>i.strength===1)){const latest=active.filter(i=>i.strength===1).at(-1),age=(now-latest.born)/2300;ctx.globalAlpha=(1-age)*.15;ctx.strokeStyle=colour(latest.kind);ctx.lineWidth=1;ctx.font=`${Math.min(width*.14,160)}px monospace`;ctx.strokeText(latest.kind,width*.54,height*.51);}
  ctx.globalAlpha=1;ctx.font=`${font}px ui-monospace, "Cascadia Code", "DejaVu Sans Mono", monospace`;ctx.textBaseline='top';
  function text(value,y,tint=palette.foreground,alpha=1,stable=false) {
    const str=clean(value,columns);ctx.fillStyle=tint;ctx.globalAlpha=alpha;
    // Every terminal line participates in the expanding disturbance.
    let shift=0;
    for(const i of stable?[]:active) {const age=(now-i.born)/2300,d=Math.abs(y-i.y*height);shift+=Math.sin(d/18-age*22)*Math.exp(-Math.abs(d-age*height)/95)*(1-age)*16*i.strength;}
    ctx.fillText(str,margin+shift,y);
    // Horizontal signal tearing is confined to the stream, never thread states.
    if(!stable && y>height*.65 && active.length){
      const impact=active.at(-1),age=now-impact.born;
      if(age<800){ctx.save();ctx.beginPath();ctx.rect(margin,y+font*.45,width-margin*2,3);ctx.clip();ctx.globalAlpha=.45*(1-age/800);ctx.fillStyle=palette.cyan;ctx.fillText(str,margin+shift+18*Math.sin(age*.03),y);ctx.restore();}
    }
    ctx.globalAlpha=1;
  }
  text('HERDR / OBSERVATORY                                      '+new Date().toLocaleTimeString('en-GB'),margin,palette.cyan);
  text(`observer@fleet:~$ follow --profile ${clean(snapshot?.profile || 'connecting')} --read-only`,margin+line*1.8);
  text(`sampled state / ${clean(snapshot?.theme?.name || 'awaiting palette')} / ${disconnected?'TRANSPORT LOST':'no shell execution'} / ${agents.length} panes / ${agents.filter(a=>a.status==='working').length} working / ${agents.filter(a=>a.status==='blocked').length} need input`,margin+line*2.8,palette.foreground,.5);
  let y=margin+line*4.5;
  for(const host of (snapshot?.hosts || []).slice(0,3)) {
    const live=!disconnected && previous.get(host.id)?.live,m=live?host.metrics:null;
    const cpu=Number.isFinite(m?.cpu_percent)?m.cpu_percent.toFixed(0)+'%':'—';
    text(`${live?'::':'!!'} ${clean(host.label,18).padEnd(18)} link=${live?'up  ':'lost'} proto=${number(host.protocol).padEnd(3)} cpu=${cpu.padEnd(4)} scope=${clean(m?.scope || 'unavailable',32)}`,y,live?palette.blue:palette.red);y+=line;
    const percent=v=>Number.isFinite(v)?Math.round(v)+'%':'—';
    const ratio=v=>v?.total?percent(v.used/v.total*100):'—';
    const rate=v=>Number.isFinite(v)?Math.round(v/1024)+'K':'—';
    text(`   ram=${ratio(m?.memory)} disk=${ratio(m?.disk)} gpu=${percent(m?.gpu?.percent)} rx=${rate(m?.rx_rate)} tx=${rate(m?.tx_rate)} / sample ${Number.isFinite(host.sampled_at)?Math.max(0,Math.floor(Date.now()/1000-host.sampled_at))+'s ago':'unavailable'}`,y,palette.foreground,.5);y+=line;
  }
  y=margin+line*11.5;
  text('  PANE / SOURCE          STATE    ENGINE       PROJECT / OBSERVED TASK',y,palette.foreground,.45);y+=line*1.3;
  const page=currentPage();
  const shown=filteredAgents().slice(page*processRows,(page+1)*processRows);
  for(const a of shown) {
    const address=`${clean(String(a.id).split(':').at(-1),6)}@${clean(a.host,12)}`.padEnd(22);
    text(`  ${address}${(a.status==='working'?'WORKING':statusKind(a.status)).padEnd(9)}${clean(a.harness,12).padEnd(13)}${clean(a.project,20)} r${number(a.technical?.revision)} s${number(a.technical?.state_change_seq)} / ${clean(a.title)}`,y,colour(statusKind(a.status)),1,true);y+=line;

  }
  if(!shown.length) text(disconnected?'  [activity unavailable]':'  [no permitted panes observed]',y,palette.foreground,.55);
  y=height-line*(logRows+4.2);
  text(`── OBSERVATION STREAM / ${records.length} retained / ${category} panes ${filteredAgents().length?page+1:0}/${Math.ceil(filteredAgents().length/processRows)} ──`,y,palette.blue);y+=line*1.5;
  const newest=records.at(-1),arrival=motion && newest?Math.max(0,1-(now-newest.born)/420):0;
  y+=arrival*line;
  ctx.save();ctx.beginPath();ctx.rect(margin-30,height-line*(logRows+2.7),width-margin,logRows*line);ctx.clip();
  for(const r of records.slice(-logRows)) {
    const full=`${new Date(r.at).toLocaleTimeString('en-GB')}  ${r.kind.padEnd(7)} ${r.text}`;
    const count=motion?Math.max(0,Math.floor((now-r.born)*.22)):full.length;
    text(full.slice(0,count),y,colour(r.kind),.9);y+=line;
  }
  ctx.restore();
  // Outgoing transcript glyphs crumble into fragments on each arriving record.
  if(motion && newest && records.length>logRows){
    const age=(now-newest.born)/1200;
    if(age>=0 && age<1){ctx.save();ctx.beginPath();ctx.rect(0,margin+line*(12.8+processRows),width,height);ctx.clip();const old=records[records.length-logRows-1].text;
      ctx.fillStyle=colour(newest.kind);ctx.font=`${font}px monospace`;
      for(let n=0;n<Math.min(70,old.length);n++){
        const seed=(n*37+newest.id*13)%101, x=margin+n*font*.61+Math.sin(seed)*age*65;
        const y0=height-line*(logRows+2.7);
        ctx.globalAlpha=(1-age)*.7;ctx.fillText(old[n],x,y0-age*(20+seed*.9));
        ctx.fillRect(x+seed%7,y0+age*seed,1+seed%3,2);
      }
      ctx.restore();
    }
  }
  ctx.globalAlpha=1;
  text(`observer@fleet:~$ _${motion && Math.floor(now/550)%2?'':'▌'}`,height-line*2.2,palette.green);
  text(`PASSIVE / ${paused?'FX PAUSED':reduced.matches?'REDUCED MOTION':'REACTIVE RENDER'} / ← → pages · R rotate · C category / sampled feed + state events`,height-line*.8,palette.foreground,.4);
}
function frame(now) {if(!document.hidden && now-lastFrame>=33){drainFeed(now);draw(now);lastFrame=now;}requestAnimationFrame(frame);}
async function refresh() {
  try {const response=await fetch('/api/state',{cache:'no-store',signal:AbortSignal.timeout(8000)});if(!response.ok)throw new Error();observe(await response.json());}
  catch {disconnect();}
  setTimeout(refresh,2000);
}
function pause() {paused=!paused;impacts=[];document.getElementById('pause').setAttribute('aria-pressed',String(paused));document.getElementById('pause').textContent=paused?'[SPACE] resume FX':'[SPACE] pause FX';}
async function fullscreen(){try{if(document.fullscreenElement)await document.exitFullscreen();else await document.documentElement.requestFullscreen();}catch{record('INFO','Use browser fullscreen (F11)',Date.now(),false);}}
 document.getElementById('pause').addEventListener('click',pause);
document.getElementById('fullscreen').addEventListener('click',fullscreen);
addEventListener('keydown',event=>{if(event.target?.tagName==='BUTTON')return;if(event.code==='Space'){event.preventDefault();pause();}if(event.key==='f')fullscreen();if(event.key==='ArrowRight')manualPage=currentPage()+1;if(event.key==='ArrowLeft')manualPage=Math.max(0,currentPage()-1);if(event.key==='r')manualPage=null;if(event.key==='c' && snapshot?.profile==='personal'){category=['all','work','personal'][(['all','work','personal'].indexOf(category)+1)%3];manualPage=0;}});
addEventListener('resize',resize);
setInterval(()=>{if(received && Date.now()-received>12000)disconnect();if(!disconnected)reconcile();},1000);
resize();requestAnimationFrame(frame);refresh();

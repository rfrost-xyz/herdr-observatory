'use strict';
const canvas = document.getElementById('event-effect');
const ctx = canvas.getContext('2d');
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
let snapshot = null, received = 0, disconnected = false, initialised = false;
let rowActivity = new Map();
let previous = new Map(), agents = [], records = [], serial = 0;
let pixelField=null,titleMark=null,musicTitle=null,fieldHeight=-1,music=null,musicReceived=0;
const GRID={columns:140,feedRows:4,threadRows:8};
let lastFrame=0,manualPage=null,category='all';
let palette = {background:'#101318',foreground:'#c0caf5',blue:'#7aa2f7',green:'#9ece6a',yellow:'#e0af68',red:'#f7768e',cyan:'#7dcfff'};
const clean = (value, limit=160) => Array.from(String(value ?? '—').replace(/[\x00-\x1f\x7f-\x9f]/g,' ')).slice(0,limit).join('');
const number = value => Number.isSafeInteger(value) && value >= 0 ? String(value) : '—';
const statusKind = status => ({working:'Working',done:'Done',blocked:'Blocked',idle:'Idle',unknown:'Unknown'}[status] || 'Unknown');
const colour = kind => palette[{Working:'cyan',Done:'green',Blocked:'yellow',Idle:'blue',Unknown:'yellow',LOST:'red',DETACH:'yellow',LINK:'blue',ATTACH:'blue',TOOL:'cyan',COMPACT:'yellow',USAGE:'green',BOOT:'blue',INFO:'blue'}[kind] || 'foreground'];
const FONT_FACE='"Observatory Nerd", "JetBrainsMono Nerd Font", monospace';
let nerdFontReady=false;
const GLYPHS={terminal:'\uea85',host:'\uf233',threads:'\uf126',feed:'\uf0ca',clock:'\uf017',branch:'\ue0a0',working:'\uf04b',done:'\uf00c',blocked:'\uf071',idle:'\uf04c',unknown:'\uf128',tool:'\uf0ad',compact:'\uf066',usage:'\uf080',attach:'\uf067',detach:'\uf068',link:'\uf0c1',lost:'\uf127',info:'\uf05a'};
function icon(name) {return nerdFontReady?GLYPHS[name] || '·':({terminal:'›',host:'◇',threads:'≡',feed:'≡',clock:'◷',branch:'⑂',working:'▶',done:'✓',blocked:'!',idle:'Ⅱ',unknown:'?',tool:'⚒',compact:'⇥',usage:'▥',attach:'+',detach:'−',link:'↔',lost:'×',info:'i'}[name] || '·');}
async function loadFont() {
  try {const loaded=await document.fonts?.load('16px "Observatory Nerd"');nerdFontReady=Boolean(loaded?.length);}
  catch {nerdFontReady=false;}
  draw();
}
function record(kind, text, now, context={}) {
  const entry = {id:++serial,kind,text:clean(text),at:now,born:performance.now(),project:clean(context.project || ''),detail:clean(context.detail || ''),host:clean(context.host || ''),state:clean(context.state || '—'),thread:clean(context.thread || '—'),eligible:Boolean(context.eligible)};
  records.push(entry); records = records.slice(-60);
  if(entry.eligible && !document.hidden && !reduced.matches && !disconnected && animationIndex<0 && !loadingFrames && !pendingEffect && holdElapsed>=holdSeconds*1000)pendingEffect=entry;
}
function observe(data, now=Date.now()) {
  snapshot=data; received=now;
  const recovery=disconnected; disconnected=false;
  if (recovery) record('LINK','Connection restored · watching for changes',now);
  for (const [key,value] of Object.entries(data.theme?.colours || {})) if ((key in palette || key==='accent') && /^#[0-9a-f]{6}$/i.test(value)) {palette[key]=value;if(key==='accent'){palette.blue=value;palette.cyan=value;}}
  reconcile(now,recovery);
}
function paneNumber(a) {const prefix=a.host+':';return clean(a.id?.startsWith(prefix)?a.id.slice(prefix.length):a.id,24);}
function agentEvent(kind,a,now,action) {record(kind,action,now,{project:a.project,detail:a.title,host:a.host,state:statusKind(a.status),thread:paneNumber(a),eligible:true});}
function telemetryFor(a,now=Date.now()) {
  const t=a.technical?.telemetry;
  return t && Number.isSafeInteger(t.seq) && now-t.seq/1000>=0 && now-t.seq/1000<=120000?t:null;
}
function telemetryBrief(a,now=Date.now()) {
  const t=telemetryFor(a,now);if(!t)return a.title;
  const usage=t.input!=null?`in ${number(t.input)} out ${number(t.output_tokens)} cache ${number(t.cache_read)}/${number(t.cache_write)}`:t.model;
  return [t.phase,t.tool,usage,t.context!=null && t.window?`ctx~${Math.round(100*t.context/t.window)}%`:null].filter(Boolean).join(' · ');
}
function telemetryEvent(a,before,now) {
  const t=telemetryFor(a,now);if(!t || t.seq===before.technical?.telemetry?.seq)return;
  const usageChanged=t.input!=null && ['input','output_tokens','cache_read','cache_write'].some(k=>t[k]!==before.technical?.telemetry?.[k]);
  const action=(usageChanged && ['output','idle'].includes(t.event)?'response received':null) || {'tool-start':'tool started','tool-end':'tool finished','compact-start':'compacting context','compact-end':'context compacted','compact-failed':'compaction failed','model':'model changed','subagent-start':'subagent started','subagent-stop':'subagent stopped'}[t.event];
  if(!action)return;
  const details=[t.model,t.result,t.context!=null?`context ~${number(t.context)} / ${number(t.window)}`:null,
    t.input!=null?`last response in ${number(t.input)} out ${number(t.output_tokens)} cache R ${number(t.cache_read)} W ${number(t.cache_write)}`:null].filter(Boolean).join(' · ');
  record(action==='response received'?'USAGE':t.event.startsWith('subagent')?'SUBAGENT':t.event.startsWith('compact')?'COMPACT':'TOOL',`${action}${t.tool?' · '+t.tool:''}`,now,{project:a.project,detail:details,host:a.host,state:statusKind(a.status),thread:paneNumber(a),eligible:true});
}

function reconcile(now=Date.now(), reset=false) {
  if (!snapshot) return;
  const next = new Map(); agents=[];
  for (const host of snapshot.hosts) {
    const live=!disconnected && host.online && Number.isFinite(host.sampled_at) && now/1000-host.sampled_at < snapshot.interval+20;
    const old=previous.get(host.id);
    const current=new Map(live ? host.agents.map(a=>[a.id,{...a}]) : []);
    next.set(host.id,{live,agents:current});
    if (initialised && !reset && old && old.live!==live) record(live?'LINK':'LOST',`${host.label || host.id} · ${live?'reconnected':'connection lost'}`,now);
    if (initialised && !reset && live && old?.live) {
      for (const [id,a] of current) {
        const before=old.agents.get(id);
        if (before && (before.status!==a.status || (telemetryFor(a,now)?.seq && telemetryFor(a,now).seq!==before.technical?.telemetry?.seq))) {rowActivity.set(id,performance.now());}
        if (before) telemetryEvent(a,before,now);
        if (!before) agentEvent('ATTACH',a,now,'thread joined');
        else if (before.status!==a.status) {
          agentEvent(statusKind(a.status),a,now,({working:'started working',blocked:'needs your input',done:'completed',idle:'became idle'}[a.status] || 'state unknown'));
        }
      }
      for (const [id,a] of old.agents) if (!current.has(id)) agentEvent('DETACH',a,now,'thread left');
    }
    if (live) agents.push(...current.values());
  }
  for (const [id,old] of previous) if (!next.has(id) && old.live && !reset) record('LOST',`${id} · source removed`,now);
  previous=next;
  if(pendingEffect && (!previous.get(pendingEffect.host)?.live || !records.slice(-GRID.feedRows).some(r=>r.id===pendingEffect.id)))pendingEffect=null;
  if((loadingFrames || animationIndex>=0) && !framesCurrent())stopEffect();
  const present=new Set(agents.map(a=>a.id));
  for(const id of rowActivity.keys())if(!present.has(id))rowActivity.delete(id);
  if (!initialised) { record('BOOT',`Watching ${agents.length} threads · ${snapshot.profile}`,now,false); initialised=true; }
  accessible();
}
function disconnect(now=Date.now()) {
  if (!disconnected) record('LOST','Connection lost · activity unknown',now);
  disconnected=true; agents=[];rowActivity.clear();pendingEffect=null;stopEffect();
  // Preserve source baseline until transport recovery, without inventing per-pane exits.
  accessible();
}
function accessible() {
  document.getElementById('transcript').textContent=[`HERDR OBSERVATORY / ${snapshot?.profile || 'connecting'} / ${disconnected?'disconnected':'sampled observations'}`,...agents.map(a=>`${clean(a.host)}/${clean(a.project)} ${statusKind(a.status)} ${clean(a.title)} · ${clean(telemetryBrief(a))}`),...(typeof textFrames!=='undefined' && animationIndex>=0 && framesCurrent()?['Animated capture: '+textFrames.text]:[]),...records.map(r=>eventLine(r))].join('\n');
}
function threadMotion(a,now=performance.now(),wall=Date.now()) {
  const host=snapshot?.hosts.find(h=>h.id===a.host);
  if(document.hidden || reduced.matches || disconnected || !host?.online || !Number.isFinite(host.sampled_at) || wall/1000-host.sampled_at>=snapshot.interval+20)return {moving:false,flash:0,glyph:icon(a.status)};
  const hash=Array.from(a.id).reduce((n,c)=>(Math.imul(n,31)+c.codePointAt(0))>>>0,0);
  const moving=a.status==='working';
  const elapsed=now-(rowActivity.get(a.id) ?? -Infinity);
  return {moving,
    flash:Math.max(0,1-elapsed/1800),
    glyph:moving?Array.from('⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏')[Math.floor((now+hash%1000)/130)%10]:icon(a.status)};
}
function filteredAgents(){return agents.filter(a=>category==='all'||snapshot?.profile==='work'||a.category===category);}
function currentPage(now=Date.now()) {return (manualPage ?? Math.floor(now/15000))%Math.max(1,Math.ceil(filteredAgents().length/GRID.threadRows));}
let textFrames={text:'',sources:[]}, effectSession=null, effectFrame=null, effectLibrary=null, effectBag=[],pendingEffect=null;
let effectGeneration=0;
let holdSeconds=120, holdElapsed=Infinity, lastEffect='', animationIndex=-1, frameElapsed=0, loadingFrames=false;
function framesCurrent() {return !disconnected && textFrames.sources?.length>0 && textFrames.sources.every(source=>previous.get(source.id)?.live) && records.slice(-GRID.feedRows).some(r=>r.id===textFrames.recordId);}
function stopEffect() {effectGeneration++;effectSession?.free();effectSession=null;effectFrame=null;animationIndex=-1;holdElapsed=0;}
async function refreshFrames() {
  const entry=pendingEffect;pendingEffect=null;
  if(!entry || loadingFrames || animationIndex>=0 || reduced.matches || disconnected)return;
  loadingFrames=true;const generation=effectGeneration;
  // This arrival is consumed even if loading fails. Only a subsequent new event can retry.
  textFrames={text:eventLine(entry),recordId:entry.id,sources:[{id:entry.host}]};
  try {
    effectLibrary ??= await import('./effects.mjs').then(module=>module.loadEffects().then(lib=>({...lib,next:module.nextEffect})));
    if(generation!==effectGeneration || document.hidden || reduced.matches || !framesCurrent())return;
    const effect=effectLibrary.next(effectLibrary.catalogue,effectBag,lastEffect);
    effectSession=effectLibrary.create(textFrames.text,effect,{...palette});
    effectFrame=effectSession.next();
    if(!effectFrame){stopEffect();return;}
    animationIndex=0;frameElapsed=0;lastEffect=effect;accessible();
  } catch {stopEffect(); /* Keep the live event readable on renderer failure. */}
  finally {loadingFrames=false;holdElapsed=0;}
}
function advanceEffects(delta) {
  if(disconnected || ((animationIndex>=0 || loadingFrames) && !framesCurrent())){pendingEffect=null;stopEffect();return;}
  if(reduced.matches){pendingEffect=null;return;}
  if(animationIndex>=0){
    frameElapsed+=delta;
    if(frameElapsed>=1000/30){
      frameElapsed-=1000/30;
      try {const next=effectSession.next();if(next){effectFrame=next;animationIndex++;}else stopEffect();}
      catch {stopEffect();}
    }
  }else if(!loadingFrames){holdElapsed+=delta;if(pendingEffect)refreshFrames();}
}
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
function eventLine(entry) {
  const project=clean(entry.project || '—',24),state=clean(entry.state,8),thread=clean(entry.thread,24);
  return clean(`${new Date(entry.at).toISOString().slice(11,19)} ${project} ${state} ${thread} ${entry.text}${entry.kind==='USAGE'?' · '+entry.detail:''}`,GRID.columns-2);
}
function phaseLabel(value){return ({ready:'Ready',working:'Working',tool:'Running tool',thinking:'Thinking',output:'Responding',compacting:'Compacting context',idle:'Idle',interrupted:'Interrupted',ended:'Ended'}[value] || 'Unavailable');}
function percent(value){return Number.isFinite(value)?Math.round(value)+'%':'Unavailable';}
function rate(value){return Number.isFinite(value)?(value/1024).toFixed(1)+' KiB/s':'Unavailable';}
function eventIcon(kind){return icon(({Working:'working',Done:'done',Blocked:'blocked',Idle:'idle',Unknown:'unknown',SUBAGENT:'threads',TOOL:'tool',COMPACT:'compact',USAGE:'usage',ATTACH:'attach',DETACH:'detach',LINK:'link',LOST:'lost',BOOT:'terminal',INFO:'info'})[kind] || 'info');}
function fitBackground(){
  const top=Math.max(0,Math.min(innerHeight,Math.floor(document.getElementById('activity').getBoundingClientRect().top)));
  if(top!==fieldHeight){fieldHeight=top;document.getElementById('pixel-field').style.height=top+'px';pixelField?.resize();}
}
function compactNumber(value){return Number.isSafeInteger(value) && value>=0?(value>=1000000?(value/1000000).toFixed(1)+'m':value>=1000?(value/1000).toFixed(1)+'k':String(value)):'—';}
function cardTelemetry(a,t){
  const tiles=[];
  const has=key=>Number.isSafeInteger(t?.[key]) && t[key]>=0;
  if(has('context') && t.window>0)tiles.push({label:'Context estimate',value:Math.round(t.context/t.window*100)+'%',detail:`~${number(t.context)} / ${number(t.window)} tokens`,ratio:t.context/t.window});
  if(has('input') || has('output_tokens'))tiles.push({label:'Last response',value:`${compactNumber(t.input)} ↗ ${compactNumber(t.output_tokens)} ↘`,detail:`${number(t.input)} input / ${number(t.output_tokens)} output tokens`});
  if(has('cache_read') || has('cache_write'))tiles.push({label:'Cache R / W',value:`${compactNumber(t.cache_read)} R / ${compactNumber(t.cache_write)} W`,detail:`${number(t.cache_read)} read / ${number(t.cache_write)} written tokens`});
  const subagent=t?.event==='subagent-start'?'Subagent started':t?.event==='subagent-stop'?'Subagent stopped':null;
  const activity=subagent || (t?phaseLabel(t.phase):statusKind(a.status));
  const note=!t?'No fresh hook sample':!tiles.length?(a.harness==='codex'?'Usage not exposed by Codex hooks':'Waiting for usage from harness'):!['context','window','input','output_tokens','cache_read','cache_write'].every(has)?'Some usage fields not reported':'';
  return {activity,tool:t?.tool || '',model:t?.model || '',tiles,note,subagent:Boolean(subagent)};
}
function observationColour(entry){return colour(['Working','Blocked','Done','Idle','Unknown'].includes(entry.state) && !['LOST','LINK','DETACH'].includes(entry.kind)?entry.state:entry.kind);}
function viewModel(now=performance.now(),wall=Date.now()) {
  const shown=filteredAgents().slice(currentPage(wall)*8,currentPage(wall)*8+8);
  return {
    profile:snapshot?.profile || 'Connecting',display:snapshot?.display?.host?`${clean(snapshot.display.host)} · ${snapshot.display.role==='Client'?'Client':'Host'}`:'Connecting',theme:snapshot?.theme?.name || 'Unavailable',
    connection:disconnected?'Connection lost':snapshot?(snapshot.hosts.some(h=>previous.get(h.id)?.live)?'Observing':'Sources unavailable'):'Connecting',
    total:agents.length,visibleTotal:filteredAgents().length,
    working:agents.filter(a=>a.status==='working').length,blocked:agents.filter(a=>a.status==='blocked').length,done:agents.filter(a=>a.status==='done').length,
    page:filteredAgents().length?currentPage(wall)+1:0,pages:Math.ceil(filteredAgents().length/8),
    hosts:(snapshot?.hosts || []).slice(0,3).map(h=>{const live=!disconnected && previous.get(h.id)?.live,m=live?h.metrics:null;return {id:h.id,label:clean(h.label || h.id),online:Boolean(live),metrics:[
      ['Processor',percent(m?.cpu_percent)],['Memory',percent(m?.memory?.total?m.memory.used/m.memory.total*100:null)],
      ['Graphics',percent(m?.gpu?.percent)],['Storage',percent(m?.disk?.total?m.disk.used/m.disk.total*100:null)],
      ['Network down',rate(m?.rx_rate)],['Network up',rate(m?.tx_rate)],['Sample age',live?Math.max(0,Math.floor(wall/1000-h.sampled_at))+'s':'Unavailable']
    ]};}),
    cards:shown.map(a=>{const t=telemetryFor(a,wall),motion=threadMotion(a,now,wall);return {
      id:a.id,project:clean(a.project),checkout:clean(a.checkout || 'Checkout not reported'),state:statusKind(a.status),harness:clean(a.harness),pane:paneNumber(a),host:clean(a.host),motion,
      ...cardTelemetry(a,t)
    };}),
    events:records.slice(-GRID.feedRows).map(r=>({id:r.id,line:eventLine(r),kind:r.kind,state:r.state})),
    empty:disconnected?'Current threads unavailable while disconnected':snapshot && !snapshot.hosts.some(h=>previous.get(h.id)?.live)?'Current threads unavailable. Waiting for a source.':'No permitted threads in this view'
  };
}
function node(tag,className,text){const element=document.createElement(tag);element.className=className;if(text!==undefined)element.textContent=text;return element;}
function setText(element,text){if(element.textContent!==String(text))element.textContent=text;}
let cardNodes=[],hostNodes=[],eventNodes=[];
function buildView(){
  cardNodes=Array.from({length:8},()=>{
    const card=node('article','thread-card'),head=node('div','card-top'),project=node('h3','project'),status=node('span','state'),glyph=node('span','state-glyph'),state=node('span','state-word');status.append(glyph,state);head.append(project,status);
    const checkout=node('p','checkout'),identity=node('p','identity'),activity=node('div','card-activity'),activityGlyph=node('span','activity-glyph'),activityText=node('strong','activity-text'),tool=node('span','tool-name'),model=node('p','model-name'),metrics=node('div','card-metrics'),coverage=node('p','coverage');
    activity.title='Latest observed hook activity; the state badge is Herdr’s current state';activity.append(activityGlyph,activityText,tool);
    const fields=Array.from({length:3},()=>{const tile=node('div','usage-tile'),label=node('span','usage-label'),value=node('strong','usage-value'),bar=node('span','usage-bar'),fill=node('i','');bar.append(fill);bar.setAttribute('aria-hidden','true');tile.append(label,value,bar);metrics.append(tile);return {tile,label,value,bar,fill};});
    card.append(head,checkout,identity,activity,model,metrics,coverage);document.getElementById('threads').append(card);return {card,project,glyph,state,checkout,identity,activityGlyph,activityText,tool,model,metrics,coverage,fields};
  });
  hostNodes=Array.from({length:3},()=>{const row=node('article','host-row'),name=node('strong','host-name'),status=node('span','host-status'),metrics=node('dl','host-metrics');const fields=Array.from({length:7},(_,i)=>{const pair=node('div','metric'),label=node('dt',''),value=node('dd',''),gauge=node('span','metric-gauge'),fill=node('i','metric-fill'),direction=node('span','network-direction',i===4?'↓':i===5?'↑':'');gauge.setAttribute('aria-hidden','true');direction.setAttribute('aria-hidden','true');gauge.append(fill);gauge.hidden=i>=4;direction.hidden=i<4 || i>5;pair.append(label,value,gauge,direction);metrics.append(pair);return {label,value,gauge,fill,direction};});row.append(name,status,metrics);document.getElementById('fleet').append(row);return {row,name,status,fields};});
  eventNodes=Array.from({length:4},()=>{const row=node('div','event-row'),glyph=node('span','event-icon'),text=node('span','event-text');glyph.setAttribute('aria-hidden','true');row.append(glyph,text);document.getElementById('events').append(row);return {row,glyph,text};});
}
function applyTheme(){
  const root=document.documentElement;
  for(const [key,value] of Object.entries(palette))root.style.setProperty('--'+key,value);
  const rgb=palette.background.slice(1).match(/../g).map(x=>parseInt(x,16));
  root.style.colorScheme=(rgb[0]*.2126+rgb[1]*.7152+rgb[2]*.0722)>150?'light':'dark';
}
function draw(now=performance.now()) {
  const view=viewModel(now);applyTheme();
  pixelField?.update({colours:palette});
  titleMark?.update(palette);
  fitBackground();
  const track=currentMusic();
  const widget=document.getElementById("music-widget");widget.hidden=!track;
  setText(document.getElementById("music-title"),track?.title || (track?"Track title unavailable":""));
  setText(document.getElementById('music-artist'),track?.artist || '');
  setText(document.getElementById('music-detail'),track?(track.state==='playing'?'Playing on iapetus':'Paused on iapetus'):'');
  musicTitle?.update(track,palette);
  setText(document.getElementById('music-accessible'),track?[track.title,track.artist,track.state].filter(Boolean).join(' · '):'');
  widget.title=track?[track.title,track.artist].filter(Boolean).join(' · '):'';
  setText(document.getElementById('profile'),view.display);
  setText(document.getElementById('working-count'),view.working);setText(document.getElementById('blocked-count'),view.blocked);
  setText(document.getElementById('done-count'),view.done);
  setText(document.getElementById('theme'),view.theme==='Unavailable'?'Theme unavailable':view.theme);setText(document.getElementById('connection'),view.connection);
  hostNodes.forEach((host,i)=>{const model=view.hosts[i];host.row.hidden=!model;if(!model)return;setText(host.name,model.label);host.name.title=model.label;setText(host.status,model.online?'Online':'Offline');host.status.dataset.online=String(model.online);host.fields.forEach((f,j)=>{setText(f.label,model.metrics[j][0]);setText(f.value,model.metrics[j][1]==='Unavailable'?'—':model.metrics[j][1]);f.value.title=model.metrics[j][1];f.value.setAttribute('aria-label',model.metrics[j][1]);if(j<4){const value=model.metrics[j][1],known=value.endsWith('%');f.gauge.dataset.known=String(known);f.fill.style.width=(known?Math.max(0,Math.min(100,parseFloat(value))):0)+'%';}if(j===4 || j===5)f.direction.dataset.known=String(model.metrics[j][1]!=='Unavailable');});});
  cardNodes.forEach((card,i)=>{const model=view.cards[i];card.card.hidden=!model;if(!model)return;
    card.card.dataset.state=model.state.toLowerCase();card.card.dataset.thread=model.id;card.card.style.setProperty('--impulse',model.motion.flash.toFixed(3));
    setText(card.project,model.project);card.project.title=model.project;setText(card.glyph,model.motion.glyph);setText(card.state,model.state);
    setText(card.checkout,`${icon('branch')} ${model.checkout}`);card.checkout.title='Worktree / checkout: '+model.checkout;
    setText(card.identity,`${model.harness} · ${model.host} · ${model.pane}`);card.identity.title=`Harness: ${model.harness} · Host: ${model.host} · Pane: ${model.pane}`;
    setText(card.activityGlyph,icon(model.subagent?'threads':model.tool?'tool':model.state.toLowerCase()));setText(card.activityText,model.activity);setText(card.tool,model.tool);card.tool.hidden=!model.tool;card.tool.title=model.tool;
    setText(card.model,model.model);card.model.hidden=!model.model;card.model.title=model.model;
    card.metrics.hidden=!model.tiles.length;card.fields.forEach((f,j)=>{const tile=model.tiles[j];f.tile.hidden=!tile;if(!tile)return;setText(f.label,tile.label);setText(f.value,tile.value);f.tile.title=tile.detail;f.tile.setAttribute('aria-label',tile.label+': '+tile.detail);f.bar.hidden=tile.ratio==null;f.fill.style.width=(Math.min(1,Math.max(0,tile.ratio || 0))*100)+'%';});
    setText(card.coverage,model.note);card.coverage.hidden=!model.note;
  });
  const empty=document.getElementById('empty');empty.hidden=Boolean(view.cards.length);setText(empty,view.empty);
  setText(document.getElementById('thread-total'),view.visibleTotal===view.total?view.total+' total':view.visibleTotal+' of '+view.total);
  setText(document.getElementById('page'),`${view.page} / ${view.pages}`);setText(document.getElementById('category'),`Category: ${category}`);document.getElementById('category').hidden=snapshot?.profile!=='personal';
  setText(document.getElementById('rotate'),manualPage===null?'Auto-page threads: on':'Auto-page threads: off');document.getElementById('rotate').hidden=view.pages<=1;document.getElementById('rotate').setAttribute('aria-pressed',String(manualPage===null));
  setText(document.getElementById('sampling'),`Read only · ${snapshot?.interval || '?'}s samples · intermediate changes may be missed`);
  const playing=Boolean(ctx) && animationIndex>=0 && framesCurrent() && !reduced.matches;
  let target=null;
  eventNodes.forEach((event,i)=>{const model=view.events[i];event.row.hidden=!model;if(!model)return;setText(event.glyph,eventIcon(model.kind));setText(event.text,model.line);event.text.style.visibility=playing && model.id===textFrames.recordId?'hidden':'visible';event.row.style.color=observationColour(model);event.row.style.setProperty('--event-colour',observationColour(model));if(playing && model.id===textFrames.recordId)target=event.text;});
  canvas.hidden=!target;
  if(target && ctx){const rect=target.getBoundingClientRect(),container=document.getElementById('activity').getBoundingClientRect(),ratio=Math.min(devicePixelRatio || 1,2);canvas.style.left=(rect.left-container.left)+'px';canvas.style.top=(rect.top-container.top)+'px';canvas.style.width=rect.width+'px';canvas.style.height=rect.height+'px';canvas.width=Math.round(rect.width*ratio);canvas.height=Math.round(rect.height*ratio);ctx.setTransform(ratio,0,0,ratio,0,0);ctx.clearRect(0,0,rect.width,rect.height);const font=parseFloat(getComputedStyle(target).fontSize);ctx.font=`${font}px ${FONT_FACE}`;const cell=ctx.measureText?.('M').width || font*.6;ctx.textBaseline='top';paintEffect(effectFrame,0,Math.max(0,(rect.height-font)/2),cell,rect.height);}
}
function frame(now) {pixelField?.frame(now);titleMark?.frame(now);musicTitle?.frame(now);if(!document.hidden && now-lastFrame>=33){advanceEffects(Math.min(100,now-lastFrame));draw();lastFrame=now;}if(document.hidden)lastFrame=now;requestAnimationFrame(frame);}
async function refresh() {
  try {const response=await fetch(`/api/state?page=${currentPage()}&category=${category}&hold=${holdSeconds}`,{cache:'no-store',signal:AbortSignal.timeout(8000)});if(!response.ok)throw new Error();observe(await response.json());}
  catch {disconnect();}
  setTimeout(refresh,2000);
}
async function fullscreen(){try{if(document.fullscreenElement)await document.exitFullscreen();else await document.documentElement.requestFullscreen();}catch{record('INFO','Use browser fullscreen (F11)',Date.now(),false);}}
function changePage(delta){manualPage=Math.max(0,currentPage()+delta);draw();}
function cycleCategory(){if(snapshot?.profile==='personal'){category=['all','work','personal'][(['all','work','personal'].indexOf(category)+1)%3];manualPage=0;draw();}}
for(const [id,action] of Object.entries({fullscreen,previous:()=>changePage(-1),next:()=>changePage(1),rotate:()=>{manualPage=manualPage===null?currentPage():null;draw();},category:cycleCategory}))document.getElementById(id).addEventListener('click',action);
addEventListener('keydown',event=>{if(event.key==='f')fullscreen();if(event.key==='PageDown')changePage(1);if(event.key==='PageUp')changePage(-1);if(event.key==='r'){manualPage=null;draw();}if(event.key==='c')cycleCategory();});
reduced.addEventListener?.('change',()=>{if(reduced.matches){pendingEffect=null;effectGeneration++;}draw();});
addEventListener('resize',()=>{fitBackground();pixelField?.resize();titleMark?.resize();draw();});
setInterval(()=>{if(received && Date.now()-received>12000)disconnect();if(!disconnected)reconcile();},1000);
buildView();draw();loadFont();requestAnimationFrame(frame);refresh();

function currentMusic(now=Date.now()) {
  return music?.available && ['playing','paused'].includes(music.state) && Number.isFinite(music.captured_at) && now-music.captured_at*1000>=0 && now-music.captured_at*1000<=3000 && now-musicReceived>=0 && now-musicReceived<=3000 ? music:null;
}
async function refreshMusic(){
  if(!document.hidden){
    try{const response=await fetch('/api/music',{cache:'no-store',signal:AbortSignal.timeout(1500)});if(!response.ok)throw new Error();music=await response.json();musicReceived=Date.now();pixelField?.update({music:currentMusic()});}
    catch{music=null;pixelField?.update({music:null});}
  }
  setTimeout(refreshMusic,100);
}
async function loadBackground(){
  try{const {PixelField}=await import('./background.mjs');pixelField=new PixelField(document.getElementById('pixel-field'));pixelField.update({colours:palette,music:currentMusic()});fitBackground();pixelField.resize();}
  catch{/* The foreground remains usable without a background renderer. */}
}
function backgroundHit(target){return target===document.body || ['observatory','threads','thread-section','fleet','pixel-field'].includes(target?.id) || target?.className==='thread-section';}
addEventListener('pointermove',event=>{if(backgroundHit(event.target))pixelField?.pointer(event.clientX,event.clientY);else pixelField?.pointer(null,null);});
addEventListener('click',event=>{if(backgroundHit(event.target))pixelField?.click(event.clientX,event.clientY);});
loadBackground();refreshMusic();

async function loadTitle(){
  try{const {TitleMark}=await import('./title.mjs');titleMark=new TitleMark(document.getElementById('title-mark'));titleMark.update(palette);document.getElementById('title-fallback').hidden=Boolean(titleMark.ctx);}
  catch{/* Keep the accessible, plain title if the artwork renderer is unavailable. */}
}
document.getElementById('brand-title').addEventListener('click',()=>titleMark?.trigger());
loadTitle();

async function loadMusicTitle(){
  try{const {MusicTitle}=await import('./music-title.mjs');musicTitle=new MusicTitle(document.getElementById('music-effect'),document.getElementById('music-title'));}
  catch{/* Native track text remains readable. */}
}
loadMusicTitle();

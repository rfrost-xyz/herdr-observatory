'use strict';
const canvas = document.getElementById('event-effect');
const ctx = canvas.getContext('2d');
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
let snapshot = null, received = 0, disconnected = false, initialised = false;
let rowActivity = new Map();
let previous = new Map(), agents = [], records = [], serial = 0;
let allowancePanel=null;
let reactRenderer=null;
let pixelField=null,titleMark=null,musicTitle=null,musicArtist=null,fieldHeight=-1,music=null,musicReceived=0;
const GRID={columns:140,feedRows:4,threadRows:8};
let lastFrame=0,manualPage=null,category='all';
const fleetHistory=new Map(),cardClicks=new Map();
const cacheHistory=new Map();
let palette = {background:'#101318',foreground:'#c0caf5',blue:'#7aa2f7',green:'#9ece6a',yellow:'#e0af68',red:'#f7768e',cyan:'#7dcfff'};
const clean = (value, limit=160) => Array.from(String(value ?? '—').replace(/[\x00-\x1f\x7f-\x9f]/g,' ')).slice(0,limit).join('');
const number = value => Number.isSafeInteger(value) && value >= 0 ? String(value) : '—';
const statusKind = status => ({working:'Working',done:'Done',blocked:'Blocked',idle:'Idle',unknown:'Unknown'}[status] || 'Unknown');
const colour = kind => kind==='Idle'?'var(--muted)':palette[{Working:'yellow',Done:'green',Blocked:'red',Unknown:'yellow',LOST:'red',DETACH:'yellow',LINK:'blue',ATTACH:'blue',TOOL:'cyan',COMPACT:'yellow',USAGE:'green',BOOT:'blue',INFO:'blue'}[kind] || 'foreground'];
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
  snapshot=data; received=now;sampleFleet(data);
  const recovery=disconnected; disconnected=false;
  if (recovery) record('LINK','Connection restored · watching for changes',now);
  for (const [key,value] of Object.entries(data.theme?.colours || {})) if ((key in palette || key==='accent') && /^#[0-9a-f]{6}$/i.test(value)) {palette[key]=value;if(key==='accent'){palette.blue=value;palette.cyan=value;}}
  reconcile(now,recovery);
  sampleCache(now);
}
function paneNumber(a) {const prefix=a.host+':';return clean(a.id?.startsWith(prefix)?a.id.slice(prefix.length):a.id,24);}
function agentEvent(kind,a,now,action) {record(kind,action,now,{project:a.project,detail:a.title,host:a.host,state:statusKind(a.status),thread:paneNumber(a),eligible:true});}
function telemetryFor(a,now=Date.now()) {
  const t=a.technical?.telemetry;
  if(!t || !Number.isSafeInteger(t.seq) || now-t.seq/1000<0)return null;
  if(t.usage_seq!=null && (!Number.isSafeInteger(t.usage_seq) || now-t.usage_seq/1000<0))return {...t,...Object.fromEntries(['input','output_tokens','cache_read','cache_write','context','window','usage_seq','usage_source','total_input','total_output','total_cache_read','total_cache_write','total_uncached_input','compactions','context_percent'].map(key=>[key,null]))};
  return t;
}
function telemetryBrief(a,now=Date.now()) {
  const t=telemetryFor(a,now);if(!t)return a.title;
  const usage=t.input!=null?`in ${number(t.input)} out ${number(t.output_tokens)} cache ${number(t.cache_read)}/${number(t.cache_write)}`:t.model;
  return [now-t.seq/1000>120000?'hook last observed':null,t.phase,t.tool,t.usage_seq!=null && now-t.usage_seq/1000>120000?'usage last known':null,usage,t.context!=null && t.window?`ctx~${Number.isSafeInteger(t.context_percent)?t.context_percent:Math.round(100*t.context/t.window)}%`:null].filter(Boolean).join(' · ');
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
    if(!live)fleetHistory.delete(host.id);
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
  for(const id of rowActivity.keys())if(!present.has(id))rowActivity.delete(id);for(const id of cardClicks.keys())if(!present.has(id))cardClicks.delete(id);
  if (!initialised) { record('BOOT',`Watching ${agents.length} threads · ${snapshot.profile}`,now,false); initialised=true; }
  accessible();
}
function disconnect(now=Date.now()) {
  if (!disconnected) record('LOST','Connection lost · activity unknown',now);
  disconnected=true; agents=[];rowActivity.clear();fleetHistory.clear();cacheHistory.clear();cardClicks.clear();pendingEffect=null;stopEffect();
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
const groupedNumber=value=>Number.isSafeInteger(value) && value>=0?value.toLocaleString('en-GB'):'—';
function compactTokens(value){
  if(!Number.isSafeInteger(value) || value<0)return '—';
  if(value<1000)return String(value);
  const unit=value>=999500?'M':'K',scale=unit==='M'?1000000:1000;
  const amount=value/scale;
  return (amount>=100?amount.toFixed(0):amount.toFixed(1).replace(/\.0$/,''))+unit;
}
function tokenPercent(ratio){
  if(ratio===0)return '0%';
  if(ratio===1)return '100%';
  const pct=ratio*100;
  if(pct<.1)return '<0.1%';
  if(pct>99.9)return '>99.9%';
  return Number(pct.toFixed(1))+'%';
}
function ageLabel(milliseconds){
  const seconds=Math.max(0,Math.floor(milliseconds/1000));
  if(seconds<120)return `${seconds}s ago`;
  if(seconds<7200)return `${Math.floor(seconds/60)}m ago`;
  if(seconds<172800)return `${Math.floor(seconds/3600)}h ago`;
  return `${Math.floor(seconds/86400)}d ago`;
}
function cardTelemetry(a,t){
  const tiles=[],has=key=>Number.isSafeInteger(t?.[key]) && t[key]>=0;
  const usageSource=t?.usage_seq!=null?'usage':'hook',cumulativeSource=a?.harness==='pi'?'hook':usageSource;
  if(has('context') && t.window>0){const supplied=has('context_percent'),pct=supplied?t.context_percent:Math.round(t.context/t.window*100);tiles.push({kind:'context',source:a?.harness==='pi'?'hook':usageSource,label:'Context',value:pct+'% used',exact:`~${compactTokens(t.context)} of ${compactTokens(t.window)}`,detail:`Estimated context: ~${groupedNumber(t.context)} of ${groupedNumber(t.window)} tokens. ${supplied?(t.usage_source==='codex-rollout'?'Codex percentage accounts for its reserved baseline.':'Percentage reported by the harness.'):'Percentage calculated from the reported context and window.'}`,ratio:pct/100});}
  if(has('total_input') || has('total_output'))tiles.push({kind:'session',source:cumulativeSource,label:'Session tokens',value:`↓ ${compactTokens(t.total_input)}  ↑ ${compactTokens(t.total_output)}`,detail:`${groupedNumber(t.total_input)} input / ${groupedNumber(t.total_output)} output tokens (session)`});
  if(has('total_cache_read') || has('total_uncached_input') || has('total_cache_write')){
    const bounded=has('total_input') && has('total_cache_read') && has('total_uncached_input') && t.total_cache_read<=t.total_input && t.total_uncached_input<=t.total_input-t.total_cache_read;
    const other=bounded?t.total_input-t.total_cache_read:null;
    const writes=has('total_cache_write')?t.total_cache_write:null;
    const complete=bounded && (writes==null?other===t.total_uncached_input:other-t.total_uncached_input===writes);
    const ratio=complete && t.total_input>0?t.total_cache_read/t.total_input:null;
    tiles.push({kind:'balance',source:cumulativeSource,label:ratio==null?'Input balance':`Cache read ${tokenPercent(ratio)} of input`,value:`Read ${compactTokens(t.total_cache_read)}`,exact:`Uncached ${compactTokens(t.total_uncached_input)}${writes>0?` · Write ${compactTokens(writes)}`:''}`,detail:`${groupedNumber(t.total_cache_read)} cache-read / ${groupedNumber(t.total_uncached_input)} uncached${writes!=null?` / ${groupedNumber(writes)} cache-write`:''} input tokens (session)${ratio==null?'':`; ${tokenPercent(ratio)} of input was cache-read`}`,ratio,uncachedRatio:ratio==null?null:t.total_uncached_input/t.total_input,writeRatio:ratio==null?null:(writes ?? 0)/t.total_input});
  }
  if(has('input') || has('output_tokens'))tiles.push({kind:'response',source:usageSource,label:'Last response',value:`↓ ${compactTokens(t.input)}  ↑ ${compactTokens(t.output_tokens)}`,detail:`${groupedNumber(t.input)} input / ${groupedNumber(t.output_tokens)} output tokens (last response)`});
  if(!tiles.some(tile=>tile.kind==='balance') && (has('cache_read') || has('cache_write')))tiles.push({kind:'response',source:usageSource,label:'Last response cache',value:`Read ${compactTokens(t.cache_read)}`,exact:`Write ${compactTokens(t.cache_write)}`,detail:`${groupedNumber(t.cache_read)} read / ${groupedNumber(t.cache_write)} written cache tokens (last response)`});
  const compactions=tiles.length || has('compactions')?{value:has('compactions')?number(t.compactions):'—',detail:has('compactions')?`${number(t.compactions)} reported session compactions`:'Complete compaction history unavailable'}:null;
  const subagent=t?.event==='subagent-start'?'Subagent started':t?.event==='subagent-stop'?'Subagent stopped':null;
  const phase=t?phaseLabel(t.phase):'';
  const toolEnd=t?.event==='tool-end'?(t.result==='error'?'Tool error':t.result==='cancelled'?'Tool cancelled':'Tool finished'):null;
  const activity=subagent || toolEnd || (!['Working','Idle','Ready','Ended'].includes(phase)?phase:'');
  const note=!t?'No hook sample':!tiles.length?'Waiting for reported usage':!['context','window','input','output_tokens','cache_read','cache_write'].every(has)?'Partial usage coverage':'';
  return {activity,tool:t?.tool || '',model:t?.model || '',tiles,compactions,note,subagent:Boolean(subagent)};
}
function sampleCache(now=Date.now()){
  const present=new Set(agents.map(a=>a.id));
  for(const id of cacheHistory.keys())if(!present.has(id))cacheHistory.delete(id);
  for(const a of agents){
    if(a.harness!=='codex'){cacheHistory.delete(a.id);continue;}
    const generation=a.technical?.session_generation;
    if(!Number.isSafeInteger(generation)){cacheHistory.delete(a.id);continue;}
    const t=telemetryFor(a,now);
    let history=cacheHistory.get(a.id);
    if(!history || history.generation!==generation){
      history={generation,baseline:null,points:[],markers:[],lastEventSeq:null,model:null,compactions:null,gapPending:false};
      cacheHistory.set(a.id,history);
    }
    if(!t){history.gapPending=history.gapPending || Boolean(history.baseline || history.points.length);history.baseline=null;history.points=[];history.markers=[];continue;}
    if(t.seq!==history.lastEventSeq){
      if(history.model && t.model && history.model!==t.model)history.markers.push({at:t.seq,kind:'model'});
      if(history.compactions!=null && t.compactions!=null && t.compactions>history.compactions)history.markers.push({at:t.seq,kind:'compaction'});
      if(t.event==='compact-end' && t.compactions==null)history.markers.push({at:t.seq,kind:'compaction'});
      history.lastEventSeq=t.seq;
      if(t.model)history.model=t.model;
      if(t.compactions!=null)history.compactions=t.compactions;
      history.markers=history.markers.slice(-24);
    }
    const valid=Number.isSafeInteger(t.usage_seq) && Number.isSafeInteger(t.total_input) && Number.isSafeInteger(t.total_cache_read)
      && t.total_input>=0 && t.total_cache_read>=0 && t.total_cache_read<=t.total_input;
    if(!valid){history.gapPending=history.gapPending || Boolean(history.baseline || history.points.length);history.baseline=null;history.points=[];history.markers=[];continue;}
    const sample={at:t.usage_seq,input:t.total_input,read:t.total_cache_read};
    const prior=history.baseline;
    if(prior?.at===sample.at)continue;
    if(prior && (sample.at<prior.at || sample.input<prior.input || sample.read<prior.read || sample.read-prior.read>sample.input-prior.input)){
      history.points=[];history.markers=[];history.gapPending=true;history.baseline=sample;continue;
    }
    if(prior && sample.input>prior.input){
      const input=sample.input-prior.input,read=sample.read-prior.read;
      if(history.gapPending){history.markers.push({at:sample.at,kind:'gap'});history.gapPending=false;}
      history.points.push({at:sample.at,input,read,ratio:read/input});
      history.points=history.points.slice(-24);
    }
    history.baseline=sample;
  }
}
function recentCache(id){
  const history=cacheHistory.get(id),points=history?.points || [];
  if(!points.length)return null;
  const input=points.reduce((total,p)=>total+p.input,0),read=points.reduce((total,p)=>total+p.read,0);
  return {ratio:read/input,input,read,points,markers:history.markers};
}
function cacheTrend(recent){
  const glyphs='▁▂▃▄▅▆▇█',first=recent.points[0].at;
  const entries=[...recent.points.map(point=>({at:point.at,text:glyphs[Math.min(7,Math.floor(point.ratio*7))]})),
    ...recent.markers.filter(marker=>marker.at>=first).map(marker=>({at:marker.at,text:marker.kind==='model'?'M':marker.kind==='compaction'?'C':'?'}))];
  return entries.sort((a,b)=>a.at-b.at).slice(-30).map(entry=>entry.text).join('');
}
function historicalActivity(t,activity,age){
  if(!activity)return '';
  const event={'tool-start':'Tool started',thinking:'Thinking observed',output:'Response observed','compact-start':'Compaction started'}[t.event];
  return `Observed: ${event || activity} · ${age.replace(/ ago$/,'')}`;
}
function observationColour(entry){return colour(['Working','Blocked','Done','Idle','Unknown'].includes(entry.state) && !['LOST','LINK','DETACH'].includes(entry.kind)?entry.state:entry.kind);}
function viewModel(now=performance.now(),wall=Date.now()) {
  const shown=filteredAgents().slice(currentPage(wall)*8,currentPage(wall)*8+8);
  return {
    profile:snapshot?.profile || 'Connecting',display:snapshot?.display?.host?`${clean(snapshot.display.host)} · ${snapshot.display.role==='Client'?'Client':'Host'}`:'Connecting',theme:snapshot?.theme?.name || 'Unavailable',
    connection:disconnected?'Connection lost':snapshot?(snapshot.hosts.some(h=>previous.get(h.id)?.live)?'Observing':'Sources unavailable'):'Connecting',
    total:agents.length,visibleTotal:filteredAgents().length,
    working:agents.filter(a=>a.status==='working').length,blocked:agents.filter(a=>a.status==='blocked').length,done:agents.filter(a=>a.status==='done').length,idle:agents.filter(a=>a.status==='idle').length,
    page:filteredAgents().length?currentPage(wall)+1:0,pages:Math.ceil(filteredAgents().length/8),
    hosts:(snapshot?.hosts || []).slice(0,3).map(h=>{const live=!disconnected && previous.get(h.id)?.live,m=live?h.metrics:null;return {id:h.id,label:clean(h.label || h.id),online:Boolean(live),graphicsScope:m?.gpu?.source==='intel-xe-pmu'?'Intel Xe device, busiest engine':m?.gpu?.source==='nvidia-visible'?'Visible NVIDIA GPUs, mean utilisation':null,metrics:[
      ['Processor',percent(m?.cpu_percent)],['Memory',percent(m?.memory?.total?m.memory.used/m.memory.total*100:null)],
      ['Graphics',percent(m?.gpu?.percent)],['Storage',percent(m?.disk?.total?m.disk.used/m.disk.total*100:null)],
      ['Network down',rate(m?.rx_rate)],['Network up',rate(m?.tx_rate)],['Sample age',live?Math.max(0,Math.floor(wall/1000-h.sampled_at))+'s':'Unavailable']
    ]};}),
    cards:shown.map(a=>{const t=telemetryFor(a,wall),motion=threadMotion(a,now,wall),hookAge=t?ageLabel(wall-t.seq/1000):null,usageAge=t?.usage_seq!=null?ageLabel(wall-t.usage_seq/1000):null;
      const hookLastKnown=Boolean(t && wall-t.seq/1000>120000),usageLastKnown=Boolean(t?.usage_seq!=null && wall-t.usage_seq/1000>120000);
    const detail=cardTelemetry(a,t);return {
      id:a.id,project:clean(a.project),checkout:clean(a.checkout || 'Checkout not reported'),state:statusKind(a.status),harness:clean(a.harness),pane:paneNumber(a),host:clean(a.host),motion,
      freshness:t?`Hook ${hookLastKnown?'last observed ':''}${hookAge}`:'No hook sample',
      usageFreshness:usageAge?`Usage ${usageLastKnown?'last known ':''}${usageAge}`:'Usage time unknown',
      usageSource:t?.usage_source==='codex-rollout'?'Codex record':t?.usage_source==='pi-extension'?'Pi extension':'reported sample',
      hookLastKnown,usageLastKnown,hookAge,usageAge,
      ...detail,historicalActivity:hookLastKnown && Boolean(detail.activity),activity:hookLastKnown?historicalActivity(t,detail.activity,hookAge):detail.activity,
      tiles:detail.tiles.map(tile=>({...tile,lastKnown:tile.source==='usage'?usageLastKnown:hookLastKnown})),recentCache:a.harness==='codex'?recentCache(a.id):null
    };}),
    events:records.slice(-GRID.feedRows).map(r=>({id:r.id,line:eventLine(r),kind:r.kind,state:r.state})),
    empty:disconnected?'Current threads unavailable while disconnected':snapshot && !snapshot.hosts.some(h=>previous.get(h.id)?.live)?'Current threads unavailable. Waiting for a source.':'No permitted threads in this view'
  };
}
function node(tag,className,text){const element=document.createElement(tag);element.className=className;if(text!==undefined)element.textContent=text;return element;}
function setText(element,text){if(element.textContent!==String(text))element.textContent=text;}
let cardNodes=[],hostNodes=[],eventNodes=[];
function pulseCard(id){const a=agents.find(a=>a.id===id),host=snapshot?.hosts.find(h=>h.id===a?.host);if(!reduced.matches && !document.hidden && !disconnected && host?.online && Number.isFinite(host.sampled_at) && Date.now()/1000-host.sampled_at<snapshot.interval+20)cardClicks.set(id,performance.now());}
function sparkline(values,maximum=100){
  const glyphs='▁▂▃▄▅▆▇█',tail=values.slice(-24);
  return tail.map(v=>Number.isFinite(v)?glyphs[Math.max(0,Math.min(7,Math.floor(v/Math.max(1,maximum)*7)))]: '·').join('');
}
function sampleFleet(data){
  const ids=new Set(data.hosts.map(h=>h.id));for(const id of fleetHistory.keys())if(!ids.has(id))fleetHistory.delete(id);
  for(const h of data.hosts){const old=fleetHistory.get(h.id);if(!h.online){fleetHistory.delete(h.id);continue;}if(!Number.isFinite(h.sampled_at) || old?.at===h.sampled_at)continue;
    const m=h.metrics,values=[m?.cpu_percent,m?.memory?.total?m.memory.used/m.memory.total*100:null,m?.gpu?.percent,m?.disk?.total?m.disk.used/m.disk.total*100:null,m?.rx_rate,m?.tx_rate];
    fleetHistory.set(h.id,{at:h.sampled_at,rows:[...(old?.rows || []),values.map(v=>Number.isFinite(v)?v:null)].slice(-24)});
  }
}
function buildView(){
  cardNodes=Array.from({length:8},(_,index)=>{
    const card=node('article','thread-card'),head=node('div','card-top'),project=node('h3','project'),status=node('span','state'),glyph=node('span','state-glyph'),state=node('span','state-word');status.append(glyph,state);head.append(project,status);
    card.setAttribute('role','button');card.setAttribute('tabindex','0');
    card.addEventListener('click',()=>pulseCard(card.dataset.thread));card.addEventListener('keydown',event=>{if(['Enter',' '].includes(event.key)){event.preventDefault?.();pulseCard(card.dataset.thread);}});
    const checkout=node('p','checkout'),identity=node('p','identity'),activity=node('div','card-activity'),activityGlyph=node('span','activity-glyph'),activityText=node('strong','activity-text'),tool=node('span','tool-name'),model=node('p','model-name'),metrics=node('div','card-metrics'),recent=node('p','cache-recent'),coverage=node('p','coverage');
    activity.title='Latest observed hook activity; the state badge is Herdr’s current state';activity.append(activityGlyph,activityText,tool);
    const fields=Array.from({length:4},()=>{const tile=node('div','usage-tile'),label=node('span','usage-label'),value=node('strong','usage-value'),exact=node('small','usage-exact'),bar=node('span','usage-bar'),fill=node('i',''),remainder=node('i','usage-remainder'),write=node('i','usage-write');tile.setAttribute('role','group');bar.append(fill,remainder,write);bar.setAttribute('aria-hidden','true');tile.append(label,value,exact,bar);metrics.append(tile);return {tile,label,value,exact,bar,fill,remainder,write};});
    const detail=node('div','thread-meta'),freshness=node('p','thread-freshness'),compactionAge=node('span','freshness-part'),hookAge=node('span','freshness-part'),usageAge=node('span','freshness-part'),accessibleMetrics=node('span','sr-only');accessibleMetrics.id=`thread-metrics-${index}`;detail.append(identity,model);freshness.append(compactionAge,hookAge,usageAge);card.append(head,checkout,detail,activity,metrics,recent,freshness,coverage,accessibleMetrics);document.getElementById('threads').append(card);return {card,project,glyph,state,checkout,identity,detail,freshness,compactionAge,hookAge,usageAge,activity,activityGlyph,activityText,tool,model,metrics,recent,coverage,accessibleMetrics,fields};
  });
  hostNodes=Array.from({length:3},()=>{const row=node('article','host-row'),name=node('strong','host-name'),status=node('span','host-status'),metrics=node('dl','host-metrics');const fields=Array.from({length:7},(_,i)=>{const pair=node('div','metric'),graph=node('span','metric-graph'),label=node('dt',''),value=node('dd',''),gauge=node('span','metric-gauge'),fill=node('i','metric-fill'),direction=node('span','network-direction',i===4?'↓':i===5?'↑':'');gauge.setAttribute('aria-hidden','true');direction.setAttribute('aria-hidden','true');gauge.append(fill);gauge.hidden=i>=4;direction.hidden=i<4 || i>5;pair.append(label,value,gauge,direction,graph);metrics.append(pair);return {label,value,gauge,fill,direction,graph};});row.append(name,status,metrics);document.getElementById('fleet').append(row);return {row,name,status,fields};});
  eventNodes=Array.from({length:4},()=>{const row=node('div','event-row'),glyph=node('span','event-icon'),text=node('span','event-text');glyph.setAttribute('aria-hidden','true');row.append(glyph,text);document.getElementById('events').append(row);return {row,glyph,text};});
}
function applyTheme(){
  const root=document.documentElement;
  for(const [key,value] of Object.entries(palette))root.style.setProperty('--'+key,value);
  const rgb=palette.background.slice(1).match(/../g).map(x=>parseInt(x,16));
  root.style.colorScheme=(rgb[0]*.2126+rgb[1]*.7152+rgb[2]*.0722)>150?'light':'dark';
}
function draw(now=performance.now()) {
  const view=viewModel(now);applyTheme();if(!reactRenderer)allowancePanel?.update(snapshot?.allowances || [],{disconnected,now:Date.now()});
  pixelField?.update({colours:palette});
  titleMark?.update(palette);
  fitBackground();
  const track=currentMusic();
  const widget=document.getElementById("music-widget");widget.hidden=!track;
  setText(document.getElementById("music-title"),track?.title || (track?"Track title unavailable":""));
  setText(document.getElementById('music-artist'),track?.artist || '');

  musicTitle?.update(track,palette);musicArtist?.update(track,palette);
  setText(document.getElementById('music-accessible'),track?[track.title,track.artist,track.state].filter(Boolean).join(' · '):'');
  widget.title=track?[track.title,track.artist].filter(Boolean).join(' · '):'';
  setText(document.getElementById('idle-count'),view.idle);
  setText(document.getElementById('working-count'),view.working);setText(document.getElementById('blocked-count'),view.blocked);
  setText(document.getElementById('done-count'),view.done);
  setText(document.getElementById('connection'),view.connection);
  document.getElementById('connection-loader').hidden=view.connection!=='Connecting';
  if(!reactRenderer){hostNodes.forEach((host,i)=>{const model=view.hosts[i];host.row.hidden=!model;if(!model)return;setText(host.name,model.label);host.name.title=model.label;setText(host.status,model.online?'Online':'Offline');host.status.dataset.online=String(model.online);host.row.title='Sample age: '+model.metrics[6][1];host.fields.forEach((f,j)=>{setText(f.label,model.metrics[j][0]);setText(f.value,model.metrics[j][1]==='Unavailable'?'—':model.metrics[j][1]);const detail=j===2&&model.graphicsScope?`${model.metrics[j][1]} · ${model.graphicsScope}`:model.metrics[j][1];f.value.title=detail;f.value.setAttribute('aria-label',detail);if(j<4){const value=model.metrics[j][1],known=value.endsWith('%');f.gauge.dataset.known=String(known);f.fill.style.width=(known?Math.max(0,Math.min(100,parseFloat(value))):0)+'%';}if(j===4 || j===5)f.direction.dataset.known=String(model.metrics[j][1]!=='Unavailable');const values=model.online?(fleetHistory.get(model.id)?.rows || []).map(row=>row[j]):[];const scale=j>=4?Math.max(1,...values.filter(Number.isFinite)):100;f.graph.hidden=j>5;setText(f.graph,sparkline(values,scale));const measured=values.filter(Number.isFinite).length;f.graph.title=`${measured} measured samples · ${values.length-measured} missing${j>=4?(measured?' · peak '+rate(Math.max(...values.filter(Number.isFinite))):' · peak unavailable'):' · 0–100% scale'}`;f.graph.setAttribute('aria-label',f.graph.title);});});
  cardNodes.forEach((card,i)=>{const model=view.cards[i];card.card.hidden=!model;if(!model){for(const field of ['project','state','checkout','identity','activityText','tool','model','recent','coverage','accessibleMetrics','compactionAge','hookAge','usageAge']){setText(card[field],'');card[field].title='';}card.freshness.title='';card.freshness.setAttribute('aria-label','');for(const f of card.fields){for(const part of ['label','value','exact'])setText(f[part],'');f.tile.title='';f.tile.setAttribute('aria-label','');}card.card.setAttribute('aria-label','');return;}
    card.card.dataset.state=model.state.toLowerCase();card.card.dataset.thread=model.id;card.card.setAttribute('aria-label',`${model.project}, ${model.state}. Activate for a brief visual effect`);card.card.setAttribute('aria-describedby',card.accessibleMetrics.id);setText(card.accessibleMetrics,[model.freshness,model.usageAge?`${model.usageFreshness} (${model.usageSource})`:null,...model.tiles.map(tile=>`${tile.lastKnown?'Last known '+(tile.source==='usage'?model.usageAge:model.hookAge)+'. ':''}${tile.label}: ${tile.detail}`),model.compactions?`Compactions: ${model.compactions.detail}`:null].filter(Boolean).join(' '));const clickAge=now-(cardClicks.get(model.id) ?? -Infinity);const clickLevel=reduced.matches || document.hidden || disconnected?0:Math.max(0,1-clickAge/650);card.card.style.setProperty('--click',clickLevel.toFixed(3));card.card.style.setProperty('--glitch-x',(clickLevel>0?Math.sin(clickAge*.13)*2*clickLevel:0).toFixed(2)+'px');card.card.style.setProperty('--impulse',model.motion.flash.toFixed(3));
    setText(card.project,model.project);card.project.title=model.project;setText(card.glyph,model.motion.glyph);setText(card.state,model.state);
    const checkout=model.checkout && !['.bare','Checkout not reported',model.project].includes(model.checkout)?model.checkout:'';setText(card.checkout,checkout?`${icon('branch')} ${checkout}`:'');card.checkout.hidden=!checkout;card.checkout.title='Worktree / checkout: '+model.checkout;
    setText(card.identity,`${model.harness} · ${model.host} · ${model.pane}`);card.identity.title=`Harness: ${model.harness} · Host: ${model.host} · Pane: ${model.pane}`;
    card.activity.hidden=!model.activity && !model.tool;card.activity.dataset.historical=String(model.historicalActivity);card.activity.title=model.historicalActivity?`${model.activity}${model.tool?` · ${model.tool}`:''}`:'Latest observed hook activity; the state badge is Herdr’s current state';setText(card.activityGlyph,icon(model.subagent?'threads':model.tool?'tool':model.state.toLowerCase()));setText(card.activityText,model.activity);setText(card.tool,model.tool);card.tool.hidden=!model.tool;card.tool.title=model.tool;
    setText(card.model,model.model);card.model.hidden=!model.model;card.model.title=model.model;
    card.metrics.hidden=!model.tiles.length;card.fields.forEach((f,j)=>{const tile=model.tiles[j];f.tile.hidden=!tile;if(!tile)return;f.tile.dataset.kind=tile.kind;f.tile.dataset.lastKnown=String(tile.lastKnown);setText(f.label,tile.label);setText(f.value,tile.value);setText(f.exact,tile.exact || '');f.exact.hidden=!tile.exact;f.tile.title=tile.detail;f.tile.setAttribute('aria-label',`${tile.lastKnown?'Last known. ':''}${tile.label}: ${tile.detail}`);f.bar.hidden=tile.ratio==null;f.fill.style.width=(Math.min(1,Math.max(0,tile.ratio || 0))*100)+'%';f.remainder.hidden=tile.kind!=='balance' || tile.ratio==null;f.remainder.style.width=((tile.uncachedRatio ?? 1-Math.min(1,Math.max(0,tile.ratio || 0)))*100)+'%';f.write.hidden=tile.kind!=='balance' || !tile.writeRatio;f.write.style.width=((tile.writeRatio || 0)*100)+'%';});
    const recent=model.recentCache;setText(card.recent,recent?`${model.usageLastKnown?'Last known ':''}recent cache ${tokenPercent(recent.ratio)}  ${cacheTrend(recent)}`:'');card.recent.hidden=!recent;card.recent.title=recent?`${groupedNumber(recent.read)} cached of ${groupedNumber(recent.input)} input tokens across ${recent.points.length} distinct intervals. M marks a model change; C marks compaction; ? marks an unavailable interval. Markers are observations, not cache-miss reasons.`:'';card.recent.setAttribute('aria-label',card.recent.title);
    setText(card.coverage,model.note);card.coverage.hidden=!model.note;setText(card.compactionAge,model.compactions?`Compactions ${model.compactions.value}`:'');card.compactionAge.hidden=!model.compactions;setText(card.hookAge,model.freshness);setText(card.usageAge,model.usageFreshness);card.freshness.setAttribute('aria-label',[model.compactions?`Compactions ${model.compactions.value}`:null,model.freshness,model.usageFreshness].filter(Boolean).join('. '));card.freshness.title=[model.compactions?.detail,model.usageAge?`Usage source: ${model.usageSource}`:null].filter(Boolean).join(' · ');
  });}
  const empty=document.getElementById('empty');empty.hidden=Boolean(view.cards.length);setText(empty,view.empty);
  setText(document.getElementById('thread-total'),view.visibleTotal===view.total?view.total+' total':view.visibleTotal+' of '+view.total);
  setText(document.getElementById('page'),`${view.page} / ${view.pages}`);setText(document.getElementById('category'),`Category: ${category}`);document.getElementById('category').hidden=snapshot?.profile!=='personal';
  setText(document.getElementById('rotate'),manualPage===null?'Auto-page threads: on':'Auto-page threads: off');document.getElementById('rotate').hidden=view.pages<=1;document.getElementById('rotate').setAttribute('aria-pressed',String(manualPage===null));

  const playing=Boolean(ctx) && animationIndex>=0 && framesCurrent() && !reduced.matches;
  let target=null;
  if(reactRenderer){const rendered=reactRenderer.update({view,now,history:fleetHistory,cardClicks,disconnected,reduced:reduced.matches,playing,effectRecordId:textFrames.recordId,accounts:snapshot?.allowances||[],helpers:{sparkline,rate,icon,cacheTrend,tokenPercent,groupedNumber,observationColour,eventIcon},onPulse:pulseCard});if(playing)target=rendered;}
  else eventNodes.forEach((event,i)=>{const model=view.events[i];event.row.hidden=!model;if(!model)return;setText(event.glyph,eventIcon(model.kind));setText(event.text,model.line);event.text.style.visibility=playing && model.id===textFrames.recordId?'hidden':'visible';event.row.style.color=observationColour(model);event.row.style.setProperty('--event-colour',observationColour(model));if(playing && model.id===textFrames.recordId)target=event.text;});
  canvas.hidden=!target;
  if(target && ctx){const rect=target.getBoundingClientRect(),row=target.parentElement?.getBoundingClientRect() || rect,section=document.querySelector?.('.observations-panel')?.getBoundingClientRect() || row,width=Math.max(0,Math.min(rect.width,row.right===undefined?rect.width:row.right-rect.left,section.right===undefined?rect.width:section.right-rect.left)),container=document.getElementById('activity').getBoundingClientRect(),ratio=Math.min(devicePixelRatio || 1,2);canvas.style.left=(rect.left-container.left)+'px';canvas.style.top=(rect.top-container.top)+'px';canvas.style.width=width+'px';canvas.style.height=rect.height+'px';canvas.width=Math.round(width*ratio);canvas.height=Math.round(rect.height*ratio);ctx.setTransform(ratio,0,0,ratio,0,0);ctx.clearRect(0,0,width,rect.height);const font=parseFloat(getComputedStyle(target).fontSize);ctx.font=`${font}px ${FONT_FACE}`;const cell=ctx.measureText?.('M').width || font*.6;ctx.textBaseline='top';paintEffect(effectFrame,0,Math.max(0,(rect.height-font)/2),cell,rect.height);}
}
function frame(now) {pixelField?.frame(now);titleMark?.frame(now);musicTitle?.frame(now);musicArtist?.frame(now);if(!document.hidden && now-lastFrame>=33){advanceEffects(Math.min(100,now-lastFrame));draw();lastFrame=now;}if(document.hidden)lastFrame=now;requestAnimationFrame(frame);}
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
  try{const {MusicTitle}=await import('./music-title.mjs');musicTitle=new MusicTitle(document.getElementById('music-effect'),document.getElementById('music-title'));musicArtist=new MusicTitle(document.getElementById('artist-effect'),document.getElementById('music-artist'),{field:'artist'});}
  catch{/* Native track text remains readable. */}
}
loadMusicTitle();

function playMusicEffect(){const track=currentMusic();musicTitle?.update(track,palette);musicArtist?.update(track,palette);if(track){musicTitle?.activate();musicArtist?.activate();}}
for(const id of ['music-title','music-artist']){const target=document.getElementById(id);target.setAttribute('role','button');target.setAttribute('tabindex','0');target.setAttribute('title','Play music text effect');target.addEventListener('click',playMusicEffect);target.addEventListener('keydown',event=>{if(['Enter',' '].includes(event.key)){event.preventDefault?.();playMusicEffect();}});}

import('./allowances.mjs').then(({createAllowancePanel})=>{allowancePanel=createAllowancePanel({root:document.getElementById('allowance-panels')});}).catch(()=>{});
import('./react-view.mjs').then(({createReactView})=>{
  for(const id of ['fleet','threads','events','allowance-panels','connection-loader'])document.getElementById(id).replaceChildren();
  document.getElementById('connection-loader').className='';
  allowancePanel=null;
  reactRenderer=createReactView();draw();
}).catch(()=>{/* The browser-native presentation remains usable if React cannot load. */});

import React from 'react';
import {createRoot} from 'react-dom/client';
import {flushSync} from 'react-dom';
import {Blocks} from 'loading-dev';
import {allowanceView} from './allowances.mjs';

const number = value => Number.isSafeInteger(value) && value >= 0 ? value.toLocaleString('en-GB') : '—';

function Machine({host, history, sparkline, rate}) {
  return <article className="host-row" title={`Sample age: ${host.metrics[6][1]}`}>
    <strong className="host-name" title={host.label}>{host.label}</strong>
    <span className="host-status" data-online={String(host.online)}>{host.online ? 'Online' : 'Offline'}</span>
    <dl className="host-metrics">{host.metrics.map(([label,value], index) => {
      const values=host.online ? (history.get(host.id)?.rows || []).map(row=>row[index]) : [];
      const measured=values.filter(Number.isFinite), scale=index>=4?Math.max(1,...measured):100;
      const graphTitle=`${measured.length} measured samples · ${values.length-measured.length} missing${index>=4?(measured.length?' · peak '+rate(Math.max(...measured)):' · peak unavailable'):' · 0–100% scale'}`;
      const known=index<4 && value.endsWith('%');
      return <div className="metric" key={label}><dt>{label}</dt><dd title={value} aria-label={value}>{value==='Unavailable'?'—':value}</dd>
        {index<4 && <span className="metric-gauge" data-known={String(known)} aria-hidden="true"><i className="metric-fill" style={{width:`${known?Math.max(0,Math.min(100,parseFloat(value))):0}%`}}/></span>}
        {(index===4 || index===5) && <span className="network-direction" data-known={String(value!=='Unavailable')} aria-hidden="true">{index===4?'↓':'↑'}</span>}
        {index<=5 && <span className="metric-graph" title={graphTitle} aria-label={graphTitle}>{sparkline(values,scale)}</span>}
      </div>;
    })}</dl>
  </article>;
}

function UsageTile({tile}) {
  return <div className="usage-tile" data-kind={tile.kind} data-last-known={String(tile.lastKnown)} role="group"
    title={tile.detail} aria-label={`${tile.lastKnown?'Last known. ':''}${tile.label}: ${tile.detail}`}>
    <span className="usage-label">{tile.label}</span><strong className="usage-value">{tile.value}</strong>
    {tile.exact && <small className="usage-exact">{tile.exact}</small>}
    {tile.ratio!=null && <span className="usage-bar" aria-hidden="true"><i style={{width:`${Math.min(1,Math.max(0,tile.ratio))*100}%`}}/>
      {tile.kind==='balance' && <i className="usage-remainder" style={{width:`${(tile.uncachedRatio??1-tile.ratio)*100}%`}}/>}
      {tile.kind==='balance' && Boolean(tile.writeRatio) && <i className="usage-write" style={{width:`${tile.writeRatio*100}%`}}/>}</span>}
  </div>;
}

function ThreadCard({model, index, now, cardClicks, disconnected, reduced, icon, cacheTrend, tokenPercent, groupedNumber, onPulse}) {
  const clickAge=now-(cardClicks.get(model.id)??-Infinity);
  const clickLevel=reduced||document.hidden||disconnected?0:Math.max(0,1-clickAge/650);
  const checkout=model.checkout && !['.bare','Checkout not reported',model.project].includes(model.checkout)?model.checkout:'';
  const recent=model.recentCache;
  const recentTitle=recent?`${groupedNumber(recent.read)} cached of ${groupedNumber(recent.input)} input tokens across ${recent.points.length} distinct intervals. M marks a model change; C marks compaction; ? marks an unavailable interval. Markers are observations, not cache-miss reasons.`:'';
  const metrics=[model.freshness,model.usageAge?`${model.usageFreshness} (${model.usageSource})`:null,
    ...model.tiles.map(tile=>`${tile.lastKnown?'Last known '+(tile.source==='usage'?model.usageAge:model.hookAge)+'. ':''}${tile.label}: ${tile.detail}`),
    model.compactions?`Compactions: ${model.compactions.detail}`:null].filter(Boolean).join(' ');
  const style={'--click':clickLevel.toFixed(3),'--glitch-x':`${clickLevel>0?Math.sin(clickAge*.13)*2*clickLevel:0}px`,'--impulse':model.motion.flash.toFixed(3)};
  return <article className="thread-card" data-state={model.state.toLowerCase()} data-thread={model.id} role="button" tabIndex="0"
    aria-label={`${model.project}, ${model.state}. Activate for a brief visual effect`} aria-describedby={`thread-metrics-${index}`} style={style}
    onClick={()=>onPulse(model.id)} onKeyDown={event=>{if(['Enter',' '].includes(event.key)){event.preventDefault();onPulse(model.id);}}}>
    <div className="card-top"><h3 className="project" title={model.project}>{model.project}</h3><span className="state"><span className="state-glyph" aria-hidden="true">{model.motion.moving?<Blocks size={13} color="var(--state)" playState="running"/>:model.motion.glyph}</span><span className="state-word">{model.state}</span></span></div>
    {checkout && <p className="checkout" title={`Worktree / checkout: ${model.checkout}`}>{icon('branch')} {checkout}</p>}
    <div className="thread-meta"><p className="identity" title={`Harness: ${model.harness} · Host: ${model.host} · Pane: ${model.pane}`}>{model.harness} · {model.host} · {model.pane}</p>
      {model.model && <p className="model-name" title={model.model}>{model.model}</p>}</div>
    {(model.activity||model.tool) && <div className="card-activity" data-historical={String(model.historicalActivity)} title={model.historicalActivity?`${model.activity}${model.tool?` · ${model.tool}`:''}`:'Latest observed hook activity; the state badge is Herdr’s current state'}><span className="activity-glyph">{icon(model.subagent?'threads':model.tool?'tool':model.state.toLowerCase())}</span><strong className="activity-text">{model.activity}</strong>{model.tool && <span className="tool-name" title={model.tool}>{model.tool}</span>}</div>}
    {model.tiles.length>0 && <div className="card-metrics">{model.tiles.map((tile,i)=><UsageTile key={`${tile.kind}-${i}`} tile={tile}/>)}</div>}
    {recent && <p className="cache-recent" title={recentTitle} aria-label={recentTitle}>{model.usageLastKnown?'Last known ':''}recent cache {tokenPercent(recent.ratio)}  {cacheTrend(recent)}</p>}
    <p className="thread-freshness" aria-label={[model.compactions?`Compactions ${model.compactions.value}`:null,model.freshness,model.usageFreshness].filter(Boolean).join('. ')} title={[model.compactions?.detail,model.usageAge?`Usage source: ${model.usageSource}`:null].filter(Boolean).join(' · ')}>{model.compactions && <span className="freshness-part">Compactions {model.compactions.value}</span>}<span className="freshness-part">{model.freshness}</span><span className="freshness-part">{model.usageFreshness}</span></p>
    {model.note && <p className="coverage">{model.note}</p>}<span className="sr-only" id={`thread-metrics-${index}`}>{metrics}</span>
  </article>;
}

function Account({sample, now, disconnected}) {
  const row=allowanceView(sample,{now,disconnected});
  const daily=row.daily;
  const room=row.roomPerDay===null?'—':`${new Intl.NumberFormat('en-GB',{maximumFractionDigits:1}).format(row.roomPerDay)}%/day`;
  const burn=row.burnPerDay===null?'—':`${new Intl.NumberFormat('en-GB',{maximumFractionDigits:1}).format(row.burnPerDay)}%/day`;
  return <article className="allowance-card"><h3 className="allowance-name">{row.label}<span className="allowance-plan"> · {row.plan}</span></h3>
    <div className="allowance-weekly"><strong className="allowance-percent">{row.weekly}</strong><span>weekly left</span><span className="allowance-gauge" aria-hidden="true"><i style={{width:`${row.remaining??0}%`}}/></span></div>
    <dl className="allowance-details"><dt>Resets</dt><dd>{row.reset}</dd><dt>Reset passes</dt><dd>{row.passes}</dd><dt>Next expiry</dt><dd>{row.expiry}</dd>
      <dt title="Weekly percentage remaining divided by time until reset. Even-use guide, not a token quota.">Room/day</dt><dd title="Even-use guide in percentage points of the weekly allowance per day.">{room}</dd>
      <dt title="Weekly percentage used divided by elapsed time in the seven-day window. Average so far, not a token count or forecast.">Burn/day</dt><dd title="Average weekly allowance use so far, in percentage points per day.">{burn}</dd></dl>
    <div className="allowance-activity" aria-label={daily?`ChatGPT account token activity, latest ${daily.length} days`: 'ChatGPT account token activity unavailable'}>
      <span>Daily tokens</span><strong>{daily?.length?number(daily.at(-1).tokens):'—'}</strong>
      <span className="allowance-spark" aria-hidden="true">{daily?.map(item=>item.glyph).join('')||'·'}</span></div>
    <small className="allowance-age">{row.status}</small></article>;
}

export function createReactView() {
  const roots=Object.fromEntries(['fleet','threads','events','allowance-panels','connection-loader'].map(id=>[id,createRoot(document.getElementById(id))]));
  return {
    update({view, now, history, cardClicks, disconnected, reduced, playing, effectRecordId, accounts, helpers, onPulse}) {
      flushSync(()=>{
        roots.fleet.render(view.hosts.map(host=><Machine key={host.id} host={host} history={history} sparkline={helpers.sparkline} rate={helpers.rate}/>));
        roots.threads.render(view.cards.map((model,index)=><ThreadCard key={model.id} {...{model,index,now,cardClicks,disconnected,reduced,onPulse}} {...helpers}/>));
        roots.events.render(view.events.map(model=><div className="event-row" key={model.id} data-record-id={model.id} style={{color:helpers.observationColour(model),'--event-colour':helpers.observationColour(model)}}><span className="event-icon" aria-hidden="true">{helpers.eventIcon(model.kind)}</span><span className="event-text" style={{visibility:playing&&model.id===effectRecordId?'hidden':'visible'}}>{model.line}</span></div>));
        roots['allowance-panels'].render(['Personal','Work'].map(label=><Account key={label} sample={accounts.find(item=>item?.label===label)||{label}} now={Date.now()} disconnected={disconnected}/>));
        roots['connection-loader'].render(view.connection==='Connecting'?<Blocks size={13} color="var(--blue)" playState={reduced?'paused':'running'}/>:null);
      });
      return document.querySelector(`#events [data-record-id="${effectRecordId}"] .event-text`);
    }
  };
}

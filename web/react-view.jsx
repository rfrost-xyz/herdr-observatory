import React from 'react';
import {createRoot} from 'react-dom/client';
import {flushSync} from 'react-dom';
import {Blocks} from 'loading-dev';
import {allowanceView, rateLabel} from './allowances.mjs';

function Machine({host, history, sparkline, rate}) {
  return <article className="host-row" title={`Sample age: ${host.metrics[6][1]}`}>
    <strong className="host-name" title={host.label}>{host.label}</strong>
    <span className="host-status" data-online={String(host.online)}>{host.online ? 'Online' : 'Offline'}</span>
    <dl className="host-metrics">{host.metrics.map(([label,value], index) => {
      const values=host.online ? (history.get(host.id)?.rows || []).map(row=>row[index]) : [];
      const measured=values.filter(Number.isFinite), scale=index>=4?Math.max(1,...measured):100;
      const graphTitle=`${measured.length} measured samples · ${values.length-measured.length} missing${index>=4?(measured.length?' · peak '+rate(Math.max(...measured)):' · peak unavailable'):' · 0–100% scale'}`;
      const known=index<4 && value.endsWith('%');
      const detail=index===2 && host.graphicsScope?`${value} · ${host.graphicsScope}`:value;
      return <div className="metric" key={label}><dt>{label}</dt><dd title={detail} aria-label={detail}>{value==='Unavailable'?'—':value}</dd>
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
  const paceLabel=row.pace==='over'?'Over pace':row.pace==='within'?'Within pace':'Pace unknown';
  return <article className="allowance-card" data-pace={row.pace} data-low={String(row.remaining!==null&&row.remaining<=15)}
    aria-label={`${row.label}: ${row.weekly} of weekly allowance left; at the average burn, ${row.runway}; burn ${rateLabel(row.burnPerDay)}; room ${rateLabel(row.roomPerDay)}; ${paceLabel}; reset ${row.reset}; ${row.passes} reset passes; ${row.status}.`}>
    <h3 className="allowance-name">{row.label}<span className="allowance-plan"> · {row.plan}</span></h3>
    <div className="allowance-head"><div className="allowance-reading"><strong className="allowance-percent">{row.weekly}</strong><span>weekly left</span></div><div className="allowance-outlook" title="Estimated from the average weekly allowance used per day so far. Future usage may differ."><span>At average burn</span><strong>{row.runway}</strong></div></div>
    <div className="allowance-gauge" role="meter" aria-label="Weekly allowance remaining" aria-valuemin="0" aria-valuemax="100" aria-valuenow={row.remaining??undefined} aria-valuetext={row.weekly}><i style={{width:`${row.remaining??0}%`}}/></div>
    <div className="allowance-pace" aria-label={`Burn ${rateLabel(row.burnPerDay)}; room ${rateLabel(row.roomPerDay)}. ${paceLabel}.`}>
      <div className="pace-row pace-burn" title="Average weekly allowance used per day so far. This is not a token count or forecast."><span>Burn</span><span className="pace-track" aria-hidden="true"><i style={{width:`${row.burnPerDay===null?0:row.burnPerDay/row.paceScale*100}%`}}/></span><strong>{rateLabel(row.burnPerDay)}</strong></div>
      <div className="pace-row pace-room" title="Weekly allowance left divided by time until reset. An even-use guide in the same units as burn."><span>Room</span><span className="pace-track" aria-hidden="true"><i style={{width:`${row.roomPerDay===null?0:row.roomPerDay/row.paceScale*100}%`}}/></span><strong>{rateLabel(row.roomPerDay)}</strong></div>
    </div>
    <div className="allowance-foot"><span className="allowance-reset-time">Reset {row.reset}</span>{row.passes!=='—'&&row.passes!=='0'&&<span className="allowance-passes" title={`Reset passes: ${row.passes}. Next expiry: ${row.expiry}`}>{row.passes} passes</span>}<small className="allowance-age">{row.status}</small></div>
  </article>;
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

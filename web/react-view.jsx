import React from 'react';
import {createRoot} from 'react-dom/client';
import {flushSync} from 'react-dom';
import {Blocks} from 'loading-dev';
import {allowanceView} from './allowances.mjs';

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
  return <div className="usage-tile" data-kind={tile.kind} data-last-known={String(tile.lastKnown)} data-known={String(tile.ratio!=null)} role="group"
    title={tile.detail} aria-label={`${tile.lastKnown?'Last known. ':''}${tile.label}: ${tile.detail}`}
    style={{'--ratio':`${Math.min(1,Math.max(0,tile.ratio??0))*100}%`}}>
    <span className="usage-label">{tile.label}</span><strong className="usage-value" data-compact={tile.kind==='context'?tile.value.split(' ')[0]:undefined}>{tile.value}</strong>
    {tile.exact && <small className="usage-exact">{tile.exact}</small>}
  </div>;
}

function ThreadCard({model, index, now, cardClicks, disconnected, reduced, icon, cacheTrend, tokenPercent, groupedNumber, onPulse}) {
  const clickAge=now-(cardClicks.get(model.id)??-Infinity);
  const clickLevel=reduced||document.hidden||disconnected?0:Math.max(0,1-clickAge/650);
  const checkout=model.checkout && !['.bare','Checkout not reported',model.project].includes(model.checkout)?model.checkout:'';
  const recent=model.recentCache;
  const recentTitle=recent?`${groupedNumber(recent.read)} cached of ${groupedNumber(recent.input)} input tokens across ${recent.points.length} distinct intervals. M marks a model change; C marks compaction; ? marks an unavailable interval. Markers are observations, not cache-miss reasons.`:'';
  const metrics=[`Host ${model.host}, Herdr pane ${model.pane}, harness ${model.harness}, model ${model.model || 'unavailable'}.`,model.freshness,model.usageAge?`${model.usageFreshness} (${model.usageSource})`:null,
    ...model.tiles.map(tile=>`${tile.lastKnown?'Last known '+(tile.source==='usage'?model.usageAge:model.hookAge)+'. ':''}${tile.label}: ${tile.detail}`),
    model.compactions?`Compactions: ${model.compactions.detail}`:null,model.note,
    recentTitle].filter(Boolean).join(' ');
  const style={'--click':clickLevel.toFixed(3),'--glitch-x':`${clickLevel>0?Math.sin(clickAge*.13)*2*clickLevel:0}px`,'--impulse':model.motion.flash.toFixed(3)};
  return <article className="thread-card" data-state={model.state.toLowerCase()} data-thread={model.id} role="button" tabIndex="0"
    aria-label={`${model.project}, ${model.state}. Activate for a brief visual effect`} aria-describedby={`thread-metrics-${index}`} style={style}
    onClick={()=>onPulse(model.id)} onKeyDown={event=>{if(['Enter',' '].includes(event.key)){event.preventDefault();onPulse(model.id);}}}>
    <div className="card-top"><div className="project-panel"><h3 className="project" title={model.project}>{model.project}</h3>{checkout && <p className="checkout" title={`Worktree / checkout: ${model.checkout}`}>{icon('branch')} {checkout}</p>}</div>
      <div className="state-panel"><span className="state" role="img" aria-label={model.state} title={model.state}><span className="state-glyph" aria-hidden="true">{model.motion.moving?<Blocks size={30} color="var(--state)" playState="running"/>:model.motion.glyph}</span></span></div>
      {(model.activity||model.tool) && <div className="card-activity" data-historical={String(model.historicalActivity)} title={model.historicalActivity?`${model.activity}${model.tool?` · ${model.tool}`:''}`:'Latest observed hook activity; the state icon is Herdr’s current state'}><strong className="activity-text">{model.activity||'Tool observed'}</strong>{model.tool && <span className="tool-name" title={model.tool}>{model.tool}</span>}</div>}</div>
    {model.instruments.length>0 ? <div className="card-metrics">{model.instruments.map((tile,i)=><UsageTile key={`${tile.kind}-${i}`} tile={tile}/>)}</div>
      : <div className="visual-pending" title={model.note}><span aria-hidden="true">?</span><strong>{model.note==='No hook sample'?'No hook sample':'Usage pending'}</strong></div>}
    {recent && <p className="cache-recent" title={recentTitle} aria-label={recentTitle}>{cacheTrend(recent)}</p>}
    <div className="card-footer"><div className="thread-meta"><p className="identity" title={`Host: ${model.host} · Herdr pane: ${model.pane} · Harness: ${model.harness} · Model: ${model.model || 'unavailable'}${model.compactions && model.compactions.value !== '—' ? ` · ${model.compactions.detail}` : ''}`}>{model.host} · {model.pane} · {model.harness} · {model.model || '—'}{model.compactions && model.compactions.value !== '—' && ` · C ${model.compactions.value}`}</p></div></div>
    <span className="sr-only" id={`thread-metrics-${index}`}>{metrics}</span>
  </article>;
}

function Account({sample, now, disconnected}) {
  const row=allowanceView(sample,{now,disconnected});
  return <article className="allowance-card" data-low={String(row.remaining!==null&&row.remaining<=15)} data-pace={row.pace}
    aria-label={`${row.label}: ${row.weekly} of weekly allowance left; ${row.paceLabel}; reset ${row.reset}; ${row.passes} reset passes; pass expiry ${row.expiry}; ${row.status}.${row.activity ? ` ${row.activity.exact} tokens across ${row.activity.count} reported dates.` : ' Account activity unavailable.'}`}>
    <h3 className="allowance-name">{row.label}<span className="allowance-plan"> · {row.plan}</span></h3>
    <div className="allowance-head"><div className="allowance-reading"><strong className="allowance-percent">{row.weekly}</strong>{row.displayDifference!==null&&<span className="allowance-variance" data-direction={row.displayDifference>0?'ahead':row.displayDifference<0?'behind':'even'} title={row.paceLabel} aria-label={row.paceLabel}>{row.displayDifference>0?'+':''}{row.displayDifference}%</span>}</div><span className="allowance-reset-head">Reset {row.reset}</span></div>
    <div className="allowance-gauge" role="meter" aria-label="Weekly allowance remaining" aria-valuemin="0" aria-valuemax="100" aria-valuenow={row.remaining??undefined} aria-valuetext={row.weekly}><i style={{width:`${row.remaining??0}%`}}/>{row.timeRemaining!==null&&<><span className="allowance-pace-gap" aria-hidden="true" title={row.paceLabel} style={{left:`${Math.min(row.remaining,row.timeRemaining)}%`,width:`${Math.abs(row.paceDifference)}%`}}/><span className="allowance-time-marker" style={{left:`${row.timeRemaining}%`}} title={`${Math.round(row.timeRemaining)}% of week remaining · ${row.paceLabel}`}/></>}</div>
    <div className="allowance-activity">{row.activity?<><span className="activity-summary" title={`${row.activity.exact} tokens across ${row.activity.count} reported dates`}>{row.activity.total} tokens · {row.activity.count} reported days</span><span className="activity-bars" aria-hidden="true">{row.activity.daily.map(item=><i key={item.date} style={{height:`${item.height}%`}} title={`${item.date}: ${item.tokens.toLocaleString('en-GB')} tokens`}/>)}</span></>:<span className="activity-summary">Account activity unavailable</span>}</div>
    <div className="allowance-foot">{row.passes!=='—'&&row.passes!=='0'&&<span className="allowance-passes" title={`Reset passes: ${row.passes}. Next expiry: ${row.expiry}`}>{row.passes} passes</span>}<small className="allowance-age">{row.status}</small></div>
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

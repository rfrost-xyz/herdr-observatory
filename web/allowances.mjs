// Read-only account instruments. Relative times never imply an automatic refill.
const clean = value => String(value ?? '').replace(/[\x00-\x1f\x7f-\x9f]/g, ' ').slice(0, 40);
const planLabel = value => ({prolite:'Pro Lite',self_serve_business_prolite:'Business Pro Lite',self_serve_business_usage_based:'Business usage',enterprise_cbp_automation:'Enterprise automation',enterprise_cbp_usage_based:'Enterprise usage',ent26:'Enterprise',edu_plus:'Education Plus',edu_pro:'Education Pro',unknown:'Plan unknown'}[value] || clean(value).replaceAll('_', ' ') || 'Plan unknown');
const validTime = value => Number.isFinite(value) && value > 0;
const day = 86400000;
const rateFormat=new Intl.NumberFormat('en-GB',{maximumFractionDigits:1});
export const rateLabel = value => value === null ? '—' : `${rateFormat.format(value)}%/day`;
export function relativeTime(seconds, now = Date.now()) {
  if (!validTime(seconds)) return 'Unknown';
  const delta = seconds * 1000 - now, minutes = Math.floor(Math.abs(delta) / 60000);
  if (Math.abs(delta) < 60000) return delta > 0 ? 'in <1m' : 'just now';
  const time = minutes >= 1440 ? `${Math.floor(minutes / 1440)}d ${Math.floor(minutes % 1440 / 60)}h`
    : minutes >= 60 ? `${Math.floor(minutes / 60)}h ${minutes % 60}m` : `${Math.max(1, minutes)}m`;
  return delta > 0 ? `in ${time}` : `${time} ago`;
}
export function allowanceView(sample, {now = Date.now(), disconnected = false} = {}) {
  const fresh = !disconnected && sample?.available === true && validTime(sample.sampled_at)
    && now / 1000 >= sample.sampled_at && now / 1000 - sample.sampled_at <= 600;
  const remaining = fresh && Number.isFinite(sample.weekly_remaining) && sample.weekly_remaining >= 0 && sample.weekly_remaining <= 100
    && (!validTime(sample.weekly_resets_at) || sample.weekly_resets_at * 1000 > now) ? sample.weekly_remaining : null;
  const count = fresh && Number.isSafeInteger(sample.reset_count) && sample.reset_count >= 0
    && (!validTime(sample.reset_expires_at) || sample.reset_expires_at * 1000 > now) ? sample.reset_count : null;
  const buckets=fresh && Array.isArray(sample.daily_usage) && sample.daily_usage.length<=30 && sample.daily_usage.every(item=>/^\d{4}-\d{2}-\d{2}$/.test(item?.date||'') && Number.isSafeInteger(item.tokens) && item.tokens>=0)?sample.daily_usage:null;
  const peak=buckets?.length?Math.max(1,...buckets.map(item=>item.tokens)):1;
  const untilReset=remaining!==null && validTime(sample.weekly_resets_at)?sample.weekly_resets_at*1000-now:null;
  const windowValid=untilReset!==null && untilReset>0 && untilReset<=7*day;
  const roomPerDay=windowValid?remaining*day/untilReset:null;
  const elapsed=windowValid?7*day-untilReset:null;
  const burnPerDay=elapsed!==null && elapsed>=3600000?(100-remaining)*day/elapsed:null;
  const paceScale=Math.max(25,roomPerDay??0,burnPerDay??0);
  const pace=burnPerDay===null || roomPerDay===null?'unknown':burnPerDay>roomPerDay?'over':'within';
  return {
    label: clean(sample?.label) || 'Account', plan: planLabel(sample?.plan),
    remaining, weekly: remaining === null ? '—' : `${Math.round(remaining)}%`,
    reset: fresh ? relativeTime(sample.weekly_resets_at, now) : 'Unknown',
    passes: count === null ? '—' : String(count),
    expiry: fresh && count !== 0 ? relativeTime(sample.reset_expires_at, now) : count === 0 ? 'None' : 'Unknown',
    status: fresh ? `Checked ${relativeTime(sample.sampled_at, now)}` : disconnected ? 'Disconnected' : 'Awaiting account sample',
    roomPerDay,burnPerDay,paceScale,pace,
    daily:buckets?.map(item=>({...item,glyph:'▁▂▃▄▅▆▇█'[Math.min(7,Math.floor(item.tokens/peak*7))]}))||null,
  };
}
export function createAllowancePanel({root}) {
  let signature = '';
  const document = root.ownerDocument;
  const element = (tag, name, text) => {const e = document.createElement(tag);e.className = name;if (text !== undefined)e.textContent = text;return e;};
  function update(samples = [], options = {}) {
    const rows = ['Personal', 'Work'].map(label => allowanceView(samples.find(s => s?.label?.toLowerCase() === label.toLowerCase()) || {label}, options));
    const next = JSON.stringify(rows);if (next === signature)return;signature = next;
    const panels = rows.map(row => {
      const panel = element('article', 'allowance-card');
      panel.setAttribute('data-pace',row.pace);
      panel.setAttribute('data-low',String(row.remaining!==null&&row.remaining<=15));
      const paceLabel=row.pace==='over'?'Over pace':row.pace==='within'?'Within pace':'Pace unknown';
      panel.setAttribute('aria-label',`${row.label}: ${row.weekly} of weekly allowance left; burn ${rateLabel(row.burnPerDay)}; room ${rateLabel(row.roomPerDay)}; ${paceLabel}; reset ${row.reset}; ${row.passes} reset passes; ${row.status}.`);
      const heading = element('h3', 'allowance-name', row.label);
      heading.append(element('span', 'allowance-plan', ` · ${row.plan}`));
      const head=element('div','allowance-head'),reading=element('div','allowance-reading'),reset=element('div','allowance-reset');
      reading.append(element('strong','allowance-percent',row.weekly),element('span','','weekly left'));
      reset.append(element('span','','Resets'),element('strong','',row.reset));head.append(reading,reset);
      const gauge = element('span', 'allowance-gauge');
      const fill = element('i', '');fill.style.width = `${row.remaining ?? 0}%`;gauge.append(fill);
      gauge.setAttribute('role','meter');gauge.setAttribute('aria-label','Weekly allowance remaining');gauge.setAttribute('aria-valuemin','0');gauge.setAttribute('aria-valuemax','100');gauge.setAttribute('aria-valuetext',row.weekly);
      if(row.remaining!==null)gauge.setAttribute('aria-valuenow',String(row.remaining));
      const pace=element('div','allowance-pace');
      pace.setAttribute('aria-label',`Burn ${rateLabel(row.burnPerDay)}; room ${rateLabel(row.roomPerDay)}. ${paceLabel}.`);
      for(const [name,value,detail] of [['Burn',row.burnPerDay,'Average weekly allowance used per day so far. This is not a token count or forecast.'],['Room',row.roomPerDay,'Weekly allowance left divided by time until reset. An even-use guide in the same units as burn.']]){
        const line=element('div',`pace-row pace-${name.toLowerCase()}`),track=element('span','pace-track'),bar=element('i','');
        line.title=detail;track.setAttribute('aria-hidden','true');bar.style.width=`${value===null?0:value/row.paceScale*100}%`;track.append(bar);
        line.append(element('span','',name),track,element('strong','',rateLabel(value)));pace.append(line);
      }
      const foot=element('div','allowance-foot');foot.append(element('span','pace-verdict',paceLabel));
      if(row.passes!=='—'&&row.passes!=='0'){
        const passes=element('span','allowance-passes',`${row.passes} passes`);passes.title=`Reset passes: ${row.passes}. Next expiry: ${row.expiry}`;foot.append(passes);
      }
      foot.append(element('small','allowance-age',row.status));
      panel.append(heading,head,gauge,pace,foot);
      return panel;
    });
    root.replaceChildren(...panels);
  }
  return {update};
}

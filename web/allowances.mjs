// Read-only account instruments. Relative times never imply an automatic refill.
const clean = value => String(value ?? '').replace(/[\x00-\x1f\x7f-\x9f]/g, ' ').slice(0, 40);
const planLabel = value => ({prolite:'Pro Lite',self_serve_business_prolite:'Business Pro Lite',self_serve_business_usage_based:'Business usage',enterprise_cbp_automation:'Enterprise automation',enterprise_cbp_usage_based:'Enterprise usage',ent26:'Enterprise',edu_plus:'Education Plus',edu_pro:'Education Pro',unknown:'Plan unknown'}[value] || clean(value).replaceAll('_', ' ') || 'Plan unknown');
const validTime = value => Number.isFinite(value) && value > 0;
const week = 7 * 86400000;
const compactTokens = value => {
  const amount = Number(value);
  if (amount < 1000) return String(value);
  const scale = amount >= 1e12 ? 1e12 : amount >= 1e9 ? 1e9 : amount >= 1e6 ? 1e6 : 1e3;
  const unit = scale === 1e12 ? 'T' : scale === 1e9 ? 'B' : scale === 1e6 ? 'M' : 'K';
  return `${(amount / scale).toFixed(1).replace(/\.0$/, '')}${unit}`;
};
function reportedActivity(sample, fresh, now) {
  const rows = sample?.daily_usage;
  if (!fresh || !Array.isArray(rows) || rows.length > 30) return null;
  const dates = new Set(), today = new Date(now).toISOString().slice(0, 10);
  const daily = [];
  for (const row of rows) {
    const date = row?.date, tokens = row?.tokens;
    if (typeof date !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(date) ||
        Number.isNaN(Date.parse(`${date}T00:00:00Z`)) || new Date(`${date}T00:00:00Z`).toISOString().slice(0, 10) !== date ||
        date > today || dates.has(date) || !Number.isSafeInteger(tokens) || tokens < 0) return null;
    dates.add(date); daily.push({date, tokens});
  }
  if (!daily.length) return null;
  daily.sort((a,b) => a.date.localeCompare(b.date));
  const peak = Math.max(...daily.map(row => row.tokens));
  const total = daily.reduce((sum,row) => sum + BigInt(row.tokens), 0n);
  return {daily: daily.map(row => ({...row, height: peak ? row.tokens / peak * 100 : 0})),
    count: daily.length, total: compactTokens(total), exact: total.toLocaleString('en-GB')};
}
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
  const untilReset = validTime(sample.weekly_resets_at) ? sample.weekly_resets_at * 1000 - now : null;
  const paceKnown = remaining !== null && untilReset !== null && untilReset > 0 && untilReset <= week;
  const timeRemaining = paceKnown ? untilReset / week * 100 : null;
  const paceDifference = paceKnown ? remaining - timeRemaining : null;
  const pace = paceDifference === null ? 'unknown' : paceDifference > 3 ? 'reserve' : paceDifference < -3 ? 'deficit' : 'on';
  const paceLabel = pace === 'unknown' ? 'Pace unknown' : pace === 'on' ? 'On pace' : `${Math.round(Math.abs(paceDifference))} percentage points in ${pace}`;
  return {
    label: clean(sample?.label) || 'Account', plan: planLabel(sample?.plan),
    remaining, weekly: remaining === null ? '—' : `${Math.round(remaining)}%`,
    reset: fresh ? relativeTime(sample.weekly_resets_at, now) : 'Unknown',
    passes: count === null ? '—' : String(count),
    expiry: fresh && count !== 0 ? relativeTime(sample.reset_expires_at, now) : count === 0 ? 'None' : 'Unknown',
    status: fresh ? `Checked ${relativeTime(sample.sampled_at, now)}` : disconnected ? 'Disconnected' : 'Awaiting account sample',
    pace, paceLabel, timeRemaining, paceDifference, activity: reportedActivity(sample, fresh, now),
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
      panel.setAttribute('data-low',String(row.remaining!==null&&row.remaining<=15));
      panel.setAttribute('data-pace',row.pace);
      panel.setAttribute('aria-label',`${row.label}: ${row.weekly} of weekly allowance left; ${row.paceLabel}; reset ${row.reset}; ${row.passes} reset passes; pass expiry ${row.expiry}; ${row.status}.${row.activity ? ` ${row.activity.exact} tokens across ${row.activity.count} reported dates.` : ' Account activity unavailable.'}`);
      const heading = element('h3', 'allowance-name', row.label);
      heading.append(element('span', 'allowance-plan', ` · ${row.plan}`));
      const head=element('div','allowance-head'),reading=element('div','allowance-reading'),reset=element('span','allowance-reset-head',`Reset ${row.reset}`);
      reading.append(element('strong','allowance-percent',row.weekly),element('span','','weekly left'));
      head.append(reading,reset);
      const gauge = element('span', 'allowance-gauge');
      const fill = element('i', '');fill.style.width = `${row.remaining ?? 0}%`;gauge.append(fill);
      if(row.timeRemaining !== null){const marker=element('span','allowance-time-marker');marker.style.left=`${row.timeRemaining}%`;marker.title=`${Math.round(row.timeRemaining)}% of week remaining`;gauge.append(marker);}
      gauge.setAttribute('role','meter');gauge.setAttribute('aria-label','Weekly allowance remaining');gauge.setAttribute('aria-valuemin','0');gauge.setAttribute('aria-valuemax','100');gauge.setAttribute('aria-valuetext',row.weekly);
      if(row.remaining!==null)gauge.setAttribute('aria-valuenow',String(row.remaining));
      const pace=element('div','allowance-pace-label');pace.append(element('span','',row.paceLabel));if(row.timeRemaining!==null)pace.append(element('small','','│ time left'));
      const foot=element('div','allowance-foot');
      if(row.passes!=='—'&&row.passes!=='0'){
        const passes=element('span','allowance-passes',`${row.passes} passes`);passes.title=`Reset passes: ${row.passes}. Next expiry: ${row.expiry}`;foot.append(passes);
      }
      foot.append(element('small','allowance-age',row.status));
      const activity=element('div','allowance-activity');
      if(row.activity){
        const title=element('span','activity-summary',`${row.activity.total} tokens · ${row.activity.count} reported days`);
        title.title=`${row.activity.exact} tokens across ${row.activity.count} reported dates`;
        const bars=element('span','activity-bars');bars.setAttribute('aria-hidden','true');
        for(const item of row.activity.daily){const bar=element('i','');bar.style.height=`${item.height}%`;bar.title=`${item.date}: ${item.tokens.toLocaleString('en-GB')} tokens`;bars.append(bar);}
        activity.append(title,bars);
      } else activity.append(element('span','activity-summary','Account activity unavailable'));
      panel.append(heading,head,gauge,pace,activity,foot);
      return panel;
    });
    root.replaceChildren(...panels);
  }
  return {update};
}

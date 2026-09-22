// Read-only account instruments. Relative times never imply an automatic refill.
const clean = value => String(value ?? '').replace(/[\x00-\x1f\x7f-\x9f]/g, ' ').slice(0, 40);
const planLabel = value => ({prolite:'Pro Lite',self_serve_business_prolite:'Business Pro Lite',self_serve_business_usage_based:'Business usage',enterprise_cbp_automation:'Enterprise automation',enterprise_cbp_usage_based:'Enterprise usage',ent26:'Enterprise',edu_plus:'Education Plus',edu_pro:'Education Pro',unknown:'Plan unknown'}[value] || clean(value).replaceAll('_', ' ') || 'Plan unknown');
const validTime = value => Number.isFinite(value) && value > 0;
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
  return {
    label: clean(sample?.label) || 'Account', plan: planLabel(sample?.plan),
    remaining, weekly: remaining === null ? '—' : `${Math.round(remaining)}%`,
    reset: fresh ? relativeTime(sample.weekly_resets_at, now) : 'Unknown',
    passes: count === null ? '—' : String(count),
    expiry: fresh && count !== 0 ? relativeTime(sample.reset_expires_at, now) : count === 0 ? 'None' : 'Unknown',
    status: fresh ? `Checked ${relativeTime(sample.sampled_at, now)}` : disconnected ? 'Disconnected' : 'Awaiting account sample',
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
      const heading = element('h3', 'allowance-name', row.label);
      heading.append(element('span', 'allowance-plan', ` · ${row.plan}`));
      const weekly = element('div', 'allowance-weekly');
      weekly.append(element('strong', 'allowance-percent', row.weekly), element('span', '', 'weekly left'));
      const gauge = element('span', 'allowance-gauge');gauge.setAttribute('aria-hidden', 'true');
      const fill = element('i', '');fill.style.width = `${row.remaining ?? 0}%`;gauge.append(fill);weekly.append(gauge);
      const details = element('dl', 'allowance-details');
      for (const [label, value] of [['Resets', row.reset], ['Reset passes', row.passes], ['Next expiry', row.expiry]]) {
        details.append(element('dt', '', label), element('dd', '', value));
      }
      panel.append(heading, weekly, details, element('small', 'allowance-age', row.status));
      return panel;
    });
    root.replaceChildren(...panels);
  }
  return {update};
}

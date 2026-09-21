'use strict';
const $ = id => document.getElementById(id);
let state = null;
let received = 0;
let failed = false;
let page = 0;
let rotationPaused = false;
const AGENT_PAGE = 6;
const HOST_PAGE = 3;
const escapeHtml = value => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const label = status => ({working:'Working',blocked:'Needs input',done:'Done',idle:'Idle',unknown:'Unknown'}[status] || 'Unknown');
const ago = at => at ? `${Math.max(0, Math.floor(Date.now() / 1000 - at))}s ago` : 'No sample';
const bytes = n => n == null ? '—' : n >= 1073741824 ? `${(n / 1073741824).toFixed(1)} GB` : `${(n / 1048576).toFixed(1)} MB`;
const rate = n => n == null ? '—' : n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB/s` : `${(n / 1024).toFixed(1)} KB/s`;
const pct = n => n == null || !Number.isFinite(n) ? '—' : `${n.toFixed(0)}%`;
const stale = () => failed || !received || Date.now() - received > 12000;
const usable = host => !stale() && host.online && Date.now() / 1000 - host.sampled_at < (state.interval + 20);
const visibleAgents = () => state.hosts.flatMap(h => usable(h) ? h.agents : []).filter(a => ($('machine').value === 'all' || a.host === $('machine').value) && ($('category').value === 'all' || a.category === $('category').value));
function meter(name, value, detail) {
  const width = Number.isFinite(value) ? Math.max(0, Math.min(100, value)) : 0;
  return `<div class="meter"><span>${name}</span><span class="bar"><i style="width:${width}%"></i></span><span class="meter-value">${escapeHtml(detail ?? pct(value))}</span></div>`;
}
function sparkSegments(trend) {
  const segments = [];
  let points = [];
  trend.forEach((sample, index) => {
    if (!Number.isFinite(sample.working)) {
      if (points.length) segments.push(points.join(' '));
      points = [];
    } else {
      points.push(`${index * 100 / Math.max(1, trend.length - 1)},${28 - Math.min(25, sample.working * 5)}`);
    }
  });
  if (points.length) segments.push(points.join(' '));
  return segments.map(points => `<polyline points="${points}"/>`).join('');
}
// Identity controls rhythm; monotonic time keeps replaced cards on that timeline.
function identityHash(identity) {
  let hash = 2166136261;
  for (const character of String(identity)) hash = Math.imul(hash ^ character.charCodeAt(0), 16777619) >>> 0;
  // Avalanche the suffix too, so neighbouring pane IDs do not look correlated.
  hash = Math.imul(hash ^ (hash >>> 16), 0x85ebca6b) >>> 0;
  hash = Math.imul(hash ^ (hash >>> 13), 0xc2b2ae35) >>> 0;
  return (hash ^ (hash >>> 16)) >>> 0;
}
function heartbeatPhase(identity) {
  const hash = identityHash(identity);
  const sweep = 2800 + hash % 2701;
  const pulse = 1700 + (hash >>> 8) % 1801;
  const elapsed = performance.now() + hash % 60000;
  return `--sweep-duration:${sweep}ms;--pulse-duration:${pulse}ms;--sweep-delay:-${elapsed % sweep}ms;--pulse-delay:-${elapsed % pulse}ms`;
}
const technicalNumber = value => Number.isSafeInteger(value) && value >= 0 ? value : '—';
function technicalStrip(agent) {
  const t = agent.technical || {};
  const flag = (value, yes, no) => value === true ? yes : value === false ? no : '—';
  return `<div class="technical-strip"><span title="Herdr pane revision; not a token count">REV <b>${technicalNumber(t.revision)}</b></span><span title="Herdr state-change sequence; not completed tasks">SEQ <b>${technicalNumber(t.state_change_seq)}</b></span><span class="${t.focused === true ? 'focus-marker' : ''}" title="Focused in Herdr">${flag(t.focused,'FOCUS','BACKGROUND')}</span><span title="Interactive readiness / launch pending, when supplied by Herdr">${t.launch_pending === true ? 'LAUNCHING' : flag(t.interactive_ready,'READY','NOT READY')}</span></div>`;
}
function threadHeartbeat() {
  return '<span class="thread-heartbeat" aria-hidden="true"><svg viewBox="0 0 240 16" preserveAspectRatio="none"><path d="M0 9H32l4 -3 4 3h20l5 -7 5 13 5 -6h48l4 -3 4 3h31l5 -7 5 13 5 -6H240"/></svg></span>';
}
function render() {
  $('clock').textContent = new Date().toLocaleTimeString('en-GB');
  if (!state) { $('connection').textContent = failed ? 'DISCONNECTED' : 'CONNECTING'; return; }
  $('connection').textContent = stale() ? 'STALE / DISCONNECTED' : '● LIVE';
  $('connection').style.color = stale() ? 'var(--yellow)' : 'var(--green)';
  $('profile').textContent = state.profile;
  $('theme').textContent = state.theme.name;
  for (const [key, colour] of Object.entries(state.theme.colours)) {
    if (/^[a-z_]+$/.test(key) && /^#[0-9a-f]{6}$/i.test(colour)) document.documentElement.style.setProperty(`--${key}`, colour);
  }
  const all = state.hosts.flatMap(h => usable(h) ? h.agents : []);
  for (const status of ['working','blocked','done']) $(status).textContent = all.filter(a => a.status === status).length;
  $('connected').textContent = `${state.hosts.filter(usable).length}/${state.hosts.length}`;
  $('fleet-note').textContent = stale() ? 'LAST SNAPSHOT · CONNECTION LOST' : `REFRESH ${state.interval}S · READ ONLY`;
  $('category').hidden = state.profile === 'work';
  if (state.profile === 'work') $('category').value = 'all';
  const previous = $('machine').value;
  const options = '<option value="all">All machines</option>' + state.hosts.map(h => `<option value="${escapeHtml(h.id)}">${escapeHtml(h.label)}</option>`).join('');
  if ($('machine').innerHTML !== options) { $('machine').innerHTML = options; $('machine').value = state.hosts.some(h => h.id === previous) ? previous : 'all'; }
  const hostPages = Math.max(1, Math.ceil(state.hosts.length / HOST_PAGE));
  const shownHosts = state.hosts.slice((page % hostPages) * HOST_PAGE, (page % hostPages + 1) * HOST_PAGE);
  $('network').innerHTML = shownHosts.map(h => {
    const live = usable(h), agents = live ? h.agents : [], working = agents.filter(a => a.status === 'working').length;
    return `<article style="${heartbeatPhase('machine:' + h.id)}" class="machine-node ${live ? (working ? 'active' : '') : 'offline'}"><div class="node-header"><strong>${escapeHtml(h.label)}</strong><small>${live ? `HERDR ${escapeHtml(h.version)} / P${technicalNumber(h.protocol)}` : 'UNAVAILABLE'}</small></div><div class="node-signal"><span class="node-core">◎</span><span class="signal-line"></span><div class="agent-dots">${agents.slice(0,24).map(a => `<span class="agent-dot ${a.status}" title="${escapeHtml(a.project)}: ${label(a.status)}"></span>`).join('') || '<span class="agent-dot"></span>'}</div></div><div class="node-count">${live ? `<b>${working}</b> working / <b>${agents.length}</b> agents · ${escapeHtml(ago(h.sampled_at))}` : escapeHtml(stale() ? 'Display disconnected' : h.error || 'Sample expired')}</div></article>`;
  }).join('');
  const agents = visibleAgents().sort((a,b) => ['blocked','working','done','idle','unknown'].indexOf(a.status) - ['blocked','working','done','idle','unknown'].indexOf(b.status));
  const agentPages = Math.max(1, Math.ceil(agents.length / AGENT_PAGE));
  $('page-counter').textContent = `Agents ${page % agentPages + 1}/${agentPages} · Machines ${page % hostPages + 1}/${hostPages}`;
  const shownAgents = agents.slice((page % agentPages) * AGENT_PAGE, (page % agentPages + 1) * AGENT_PAGE);
  $('agents').innerHTML = shownAgents.map(a => `<article class="agent-card" style="${heartbeatPhase('thread:' + a.host + ':' + a.id)}">${a.status === 'working' ? threadHeartbeat() : ''}<div class="agent-top"><strong>${escapeHtml(a.project)}</strong><span class="badge ${a.status}">● ${label(a.status)}</span></div><p class="task" title="${escapeHtml(a.title)}">${escapeHtml(a.title)}</p>${technicalStrip(a)}<div class="agent-meta"><span>${escapeHtml(a.harness)} / ${escapeHtml(a.host)} / ${escapeHtml(a.category)}</span><span title="Time since this state was first observed">${escapeHtml(ago(a.since))}</span></div></article>`).join('') || `<p class="empty">${stale() ? 'Connection lost. Agent activity is no longer live.' : state.profile === 'work' ? 'No visible work agents. Only configured work projects appear in this profile.' : 'No agents match this view.'}</p>`;
  const history = state.history.filter(a => ($('machine').value === 'all' || a.host === $('machine').value) && ($('category').value === 'all' || a.category === $('category').value));
  $('timeline').innerHTML = history.slice(0,6).map(a => `<div class="event ${a.status}"><span class="event-time">${new Date(a.at * 1000).toLocaleTimeString('en-GB')} / ${escapeHtml(a.host)}</span><span class="event-project">${escapeHtml(a.project)}</span><br><span class="event-status">${a.observation === 'discovered' ? 'DISCOVER' : 'STATE'} → ${label(a.status).toLowerCase()}</span></div>`).join('') || '<p class="empty">Waiting for observed state changes.</p>';
  $('resources').innerHTML = shownHosts.map(h => {
    const m = !stale() && h.metrics && Date.now()/1000 - h.sampled_at < state.interval + 20 ? h.metrics : null;
    if (!m) return `<article class="resource offline"><h3>${escapeHtml(h.label)}</h3><p class="resource-error">Telemetry unavailable</p><span class="resource-scope">${escapeHtml(ago(h.sampled_at))}</span></article>`;
    const memory = m.memory ? m.memory.used / m.memory.total * 100 : null;
    const disk = m.disk ? m.disk.used / m.disk.total * 100 : null;
    const trace = sparkSegments(h.trend);
    return `<article class="resource"><h3>${escapeHtml(h.label)}</h3><div class="resource-scope">${escapeHtml(m.scope)} · ${escapeHtml(ago(h.sampled_at))}</div>${meter('CPU',m.cpu_percent)}${meter('RAM',memory,m.memory ? bytes(m.memory.used) : '—')}${meter('DISK',disk)}${meter('GPU',m.gpu?.percent,m.gpu ? pct(m.gpu.percent) : 'Unavailable')}<div class="resource-bottom"><span>↓ ${rate(m.rx_rate)}</span><span>↑ ${rate(m.tx_rate)}</span><span>${m.gpu ? `${bytes(m.gpu.used)} VRAM` : 'No GPU sample'}</span></div><svg class="spark" viewBox="0 0 100 30" preserveAspectRatio="none" role="img" aria-label="Recent working agent count, clipped at five">${trace}</svg><span class="resource-scope">Working agents / last ${h.trend.length} samples · gaps = unavailable</span></article>`;
  }).join('');
  $('footer-status').textContent = state.profile === 'work' ? 'WORK DISPLAY · Permitted projects only' : `PERSONAL VIEW · Full fleet insight${state.publication ? (state.publication.ok ? ' · Office display synced' : ' · Office sync unavailable') : ''}`;
}
async function refresh() {
  try {
    const response = await fetch('/api/state', {cache:'no-store', signal:AbortSignal.timeout(8000)});
    if (!response.ok) throw new Error('Unavailable');
    state = await response.json(); received = Date.now(); failed = false;
  } catch { failed = true; }
  render(); setTimeout(refresh, 2000);
}
$('machine').addEventListener('change', () => { page = 0; render(); });
$('category').addEventListener('change', () => { page = 0; render(); });
$('previous').addEventListener('click', () => { page = Math.max(0, page - 1); render(); });
$('next').addEventListener('click', () => { page += 1; render(); });
$('pause').addEventListener('click', () => {
  rotationPaused = !rotationPaused;
  $('pause').textContent = rotationPaused ? 'Resume' : 'Pause';
  $('pause').setAttribute('aria-pressed', String(rotationPaused));
  $('pause').setAttribute('aria-label', rotationPaused ? 'Resume page rotation' : 'Pause page rotation');
});
setInterval(() => { if (!rotationPaused && !stale()) { page += 1; render(); } }, 15000);
$('fullscreen').addEventListener('click', async () => {
  try { if (document.fullscreenElement) await document.exitFullscreen(); else await document.documentElement.requestFullscreen(); }
  catch { $('connection').textContent = 'Use browser fullscreen (F11)'; }
});
setInterval(render, 1000);
refresh();

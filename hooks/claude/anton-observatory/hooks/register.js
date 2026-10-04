// herdr-observatory Claude Code mod v1; installed by the Omarchy plugin
// Reports the context window to the installed runtime, as argv, and from a
// fresh session.measure at most two five_hour/seven_day rate-limit windows
// (kind, used percent, reset epoch seconds) after it. A measurement is fresh
// when an earlier one for the same session was seen since load or the last
// session.end, no spend_limit window has been listed since then, and a window
// moved or appeared ('rateLimits' changed) or the session's cost total grew
// strictly ('cost' changed). The cost total stays in memory and is never sent.
// Reset times are converted with integer arithmetic only. It never blocks,
// alters or answers an event: every hook returns next(e).
const nativeRuntime = '';

const windowLimit = 100000000;
const staleMs = 3000;
// Window durations in seconds; a reset is kept within duration plus an hour.
const durations = { five_hour: 18000, seven_day: 604800 };
const kinds = ['five_hour', 'seven_day'];
const isoPattern = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$/;
// Shared by every hook of this module.
let confirmed = null;
let inflight = null;
let generation = 0;
let lastSeq = 0;
// The last session.measure seen: {session, used: {kind: percent}, costUsd}.
// Only session.measure reads or replaces it; session.end clears it.
let baseline = null;
// Set by any spend_limit window; blocks every tail until session.end.
let spendLimit = false;

function costTotal(e) {
  const cost = e?.cost;
  if (cost === null || typeof cost !== 'object') return null;
  const usd = cost.usd;
  return typeof usd === 'number' && Number.isFinite(usd) && usd >= 0 ? usd : null;
}

function rateList(e) {
  const list = e?.rateLimits;
  return Array.isArray(list) ? list : [];
}

// The first finite used value per kind; any other value counts as absent.
function usedValues(list) {
  const used = {};
  for (const limit of list) {
    const kind = limit?.kind;
    if (!kinds.includes(kind) || kind in used) continue;
    const p = limit.percentUsed;
    if (typeof p === 'number' && Number.isFinite(p)) used[kind] = p;
  }
  return used;
}

// Evaluates freshness and replaces the baseline (design D2).
function observe(id, e) {
  const list = rateList(e);
  if (list.some((limit) => limit?.kind === 'spend_limit')) spendLimit = true;
  const used = usedValues(list);
  const costUsd = costTotal(e);
  const changed = Array.isArray(e?.changed) ? e.changed : [];
  const previous = baseline;
  baseline = { session: id, used, costUsd };
  if (previous === null || previous.session !== id || spendLimit) return false;
  const moved = changed.includes('rateLimits')
    && kinds.some((kind) => kind in used && used[kind] !== previous.used[kind]);
  const grew = changed.includes('cost')
    && costUsd !== null && previous.costUsd !== null && costUsd > previous.costUsd;
  return moved || grew;
}

// Integer tenths as text, or null for a value outside 0 to 100 or with more
// than one decimal.
function percentText(p) {
  if (typeof p !== 'number' || !Number.isFinite(p) || p < 0 || p > 100) return null;
  const t = Math.round(p * 10);
  if (Math.abs(p * 10 - t) > 1e-6) return null;
  if (t % 10 === 0) return String(t / 10);
  return String((t - t % 10) / 10) + '.' + String(t % 10);
}

function leap(year) {
  return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
}

// Epoch seconds of an ISO 8601 reset time, the fraction truncated, or null.
function epochSeconds(text) {
  if (typeof text !== 'string') return null;
  const m = isoPattern.exec(text);
  if (m === null) return null;
  const year = Number(m[1]);
  const month = Number(m[2]);
  const day = Number(m[3]);
  const hour = Number(m[4]);
  const minute = Number(m[5]);
  const second = Number(m[6]);
  if (month < 1 || month > 12 || hour > 23 || minute > 59 || second > 59) return null;
  const monthDays = [31, leap(year) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (day < 1 || day > monthDays[month - 1]) return null;
  let offset = 0;
  if (m[8] !== 'Z') {
    const offsetHours = Number(m[8].slice(1, 3));
    const offsetMinutes = Number(m[8].slice(4, 6));
    if (offsetHours > 23 || offsetMinutes > 59) return null;
    offset = (m[8][0] === '-' ? -1 : 1) * (offsetHours * 3600 + offsetMinutes * 60);
  }
  // Days from the civil date (proleptic Gregorian), integer arithmetic only.
  const y = month <= 2 ? year - 1 : year;
  const era = Math.floor(y / 400);
  const yoe = y - era * 400;
  const doy = Math.floor((153 * (month + (month > 2 ? -3 : 9)) + 2) / 5) + day - 1;
  const doe = yoe * 365 + Math.floor(yoe / 4) - Math.floor(yoe / 100) + doy;
  const days = era * 146097 + doe - 719468;
  return days * 86400 + hour * 3600 + minute * 60 + second - offset;
}

// The rate-limit argv tail for a fresh measurement sent with `seq`: each kept
// window as kind, used and reset, five_hour first. A repeated kind drops it.
function rateTail(e, seq) {
  const whole = (seq - seq % 1000000) / 1000000;
  const kept = {};
  const seen = {};
  for (const limit of rateList(e)) {
    const kind = limit?.kind;
    if (!kinds.includes(kind)) continue;
    if (seen[kind]) return [];
    seen[kind] = true;
    const used = percentText(limit.percentUsed);
    const reset = epochSeconds(limit.resetsAt);
    if (used === null || reset === null) continue;
    if (!(whole < reset && reset <= whole + durations[kind] + 3600)) continue;
    kept[kind] = [kind, used, String(reset)];
  }
  return kinds.filter((kind) => kind in kept).flatMap((kind) => kept[kind]);
}

async function contextWindow($, source, e) {
  if (source === 'measure') return e?.context?.window;
  return (await $.session.usage())?.context?.window;
}

async function sample($, source, e) {
  if (!nativeRuntime) return;
  if ((await $.env.get('HERDR_ENV')) !== '1') return;
  const pane = await $.env.get('HERDR_PANE_ID');
  if (typeof pane !== 'string' || !pane) return;
  let id = await $.session.id();
  if (typeof id !== 'string') return;
  if (id.endsWith('.jsonl')) id = id.slice(0, -'.jsonl'.length);
  // Before every later guard, so a skipped measurement still moves the baseline.
  const fresh = source === 'measure' && observe(id, e);
  if (source === 'classic' && e?.session_id !== id) return;
  const size = await contextWindow($, source, e);
  if (!(Number.isSafeInteger(size) && size >= 1 && size <= windowLimit)) return;
  const key = id + ':' + size;
  // Only start and classic events are deduplicated; every turn reports.
  if (source !== 'measure' && key === confirmed) return;
  const now = await $.clock.now();
  if (typeof now !== 'number' || !Number.isSafeInteger(Math.floor(now)) || now < 1e12) return;
  // Skipped, not held: the next turn carries the latest window. A run is
  // stale once the clock has moved more than staleMs either way from its
  // start, so overlapping hooks whose readings arrive out of order still see it.
  if (inflight !== null && Math.abs(now - inflight.startedAt) <= staleMs) return;
  const seq = Math.max(lastSeq + 1, Math.floor(now) * 1000);
  // A microsecond or nanosecond clock overflows the sequence: skip, keep lastSeq.
  if (!Number.isSafeInteger(seq)) return;
  lastSeq = seq;
  const tail = fresh ? rateTail(e, seq) : [];
  const run = $.process.run(
    [nativeRuntime, '--report', 'claude', pane, String(seq), id, String(size), ...tail],
    { timeoutMs: 2000 },
  );
  if (run === null || typeof run !== 'object' || typeof run.then !== 'function') return;
  const token = ++generation;
  inflight = { token, startedAt: now };
  Promise.resolve(run)
    .then((result) => {
      if (token === generation && result !== null && typeof result === 'object' && result.exitCode === 0) {
        confirmed = key;
      }
    })
    .finally(() => {
      if (token === generation) inflight = null;
    })
    .catch(() => {});
}

function quiet() {}

export function register(on) {
  on('session.measure', async ($, e, next) => {
    try { await sample($, 'measure', e); } catch { /* never affects Claude Code */ }
    return next(e);
  }).catch(quiet);
  on('session.start', async ($, e, next) => {
    try { await sample($, 'start', e); } catch { /* never affects Claude Code */ }
    return next(e);
  }).catch(quiet);
  on('classic.SessionStart', async ($, e, next) => {
    try { await sample($, 'classic', e); } catch { /* never affects Claude Code */ }
    return next(e);
  }).catch(quiet);
  on('session.end', async ($, e, next) => {
    try { confirmed = null; baseline = null; spendLimit = false; } catch { /* never affects Claude Code */ }
    return next(e);
  }).catch(quiet);
}

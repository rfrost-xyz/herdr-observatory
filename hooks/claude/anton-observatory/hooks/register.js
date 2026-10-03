// herdr-observatory Claude Code mod v1; installed by the Omarchy plugin
// Reports only the context window to the installed runtime, as argv. It never
// blocks, alters or answers an event: every hook returns next(e).
const nativeRuntime = '';

const windowLimit = 100000000;
const staleMs = 3000;
// Shared by every hook of this module.
let confirmed = null;
let inflight = null;
let generation = 0;
let lastSeq = 0;

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
  const run = $.process.run(
    [nativeRuntime, '--report', 'claude', pane, String(seq), id, String(size)],
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
    try { confirmed = null; } catch { /* never affects Claude Code */ }
    return next(e);
  }).catch(quiet);
}

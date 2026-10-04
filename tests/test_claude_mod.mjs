// Claude Code mod (design D2, D7). Loads the payload with the runtime
// placeholder replaced, records `on` and drives each hook with a stubbed `$`.
// No Claude Code binary, real home or Herdr socket is used.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';

const path = 'hooks/claude/anton-observatory/hooks/register.js';
const source = fs.readFileSync(path, 'utf8');
const placeholder = "const nativeRuntime = '';";
const runtime = '/synthetic/plugin/anton-runtime';
const events = ['session.measure', 'session.start', 'classic.SessionStart', 'session.end'];
const start = 1_700_000_000_000;
let loads = 0;

// A fresh module (and so fresh module state) per call.
async function load(runtimePath = runtime) {
  let text = source;
  if (runtimePath !== null) {
    text = source.replace(placeholder, `const nativeRuntime = ${JSON.stringify(runtimePath)};`);
    assert.notEqual(text, source, 'the runtime placeholder is replaced');
  }
  const module = await import(`data:text/javascript,${encodeURIComponent(`${text}\n// load ${++loads}\n`)}`);
  const hooks = {};
  const catches = [];
  module.register((name, hook) => {
    assert.equal(typeof hook, 'function');
    hooks[name] = hook;
    const registration = {
      catch(handler) {
        assert.equal(typeof handler, 'function');
        catches.push(name);
        return registration;
      },
    };
    return registration;
  });
  return { hooks, catches };
}

// A stubbed `$` whose runs settle only when the test says so.
function host() {
  const state = {
    env: { HERDR_ENV: '1', HERDR_PANE_ID: 'w1:p1' },
    id: 'session-a',
    window: 200000,
    now: start,
    runs: [],
  };
  const $ = {
    env: { get: async (name) => state.env[name] },
    session: {
      id: async () => state.id,
      usage: async () => ({ startedAt: start, context: { tokens: 10, window: state.window, percent: 0 } }),
    },
    clock: { now: async () => state.now },
    process: {
      run(argv, init) {
        let settle;
        const promise = new Promise((resolve, reject) => { settle = { resolve, reject }; });
        state.runs.push({ argv, init, ...settle });
        return promise;
      },
    },
  };
  return { $, state };
}

const turn = () => new Promise(setImmediate);
// A session.measure event in the SessionMeasureInput shape. The defaults (no
// rate-limit windows, only the context changed, a constant cost total) keep
// every run at the four change 3 values.
const cost = 3.217;
const measure = (window, fields = {}) => ({
  context: { tokens: 10, window, percent: 0 },
  rateLimits: [],
  cost: { usd: cost },
  changed: ['context'],
  ...fields,
});
const absent = Symbol('absent');
const both = ['context', 'cost'];
const all = ['context', 'rateLimits', 'cost'];
// The whole second of `start` and ISO text for an epoch second.
const S = start / 1000;
const iso = (seconds) => new Date(seconds * 1000).toISOString();
const limit = (kind, percentUsed, seconds) => ({ kind, percentUsed, resetsAt: iso(seconds) });
const five = (percentUsed, seconds = S + 3600) => limit('five_hour', percentUsed, seconds);
const seven = (percentUsed, seconds = S + 86400) => limit('seven_day', percentUsed, seconds);
const head = (seq) => [runtime, '--report', 'claude', 'w1:p1', String(seq), 'session-a', '200000'];

// Fires one hook and asserts it resolved to the value of next(e). An event
// passed explicitly, including undefined, reaches the hook as given; only an
// omitted one becomes {}.
async function fire(hooks, name, $, ...event) {
  const e = event.length > 0 ? event[0] : {};
  const passed = { passed: e };
  const result = await hooks[name]($, e, (value) => {
    assert.equal(value, e);
    return passed;
  });
  assert.equal(result, passed, `${name} resolves to next(e)`);
}

async function exit(run, exitCode) {
  run.resolve({ exitCode, stdout: '', stderr: '' });
  await turn();
}

// Fires one session.measure, settles the run it started with exit 0 and
// returns that run's argv after the four change 3 values, or null when no run
// started.
async function step(hooks, $, state, fields = {}, window = 200000) {
  const before = state.runs.length;
  const e = measure(window, fields);
  // A field given as `absent` is left out of the event entirely.
  for (const key of Object.keys(e)) if (e[key] === absent) delete e[key];
  await fire(hooks, 'session.measure', $, e);
  if (state.runs.length === before) return null;
  assert.equal(state.runs.length, before + 1);
  const run = state.runs.at(-1);
  await exit(run, 0);
  return run.argv.slice(7);
}

test('registers exactly the four events, each with a catch handler', async () => {
  const { hooks, catches } = await load();
  assert.deepEqual(Object.keys(hooks), events);
  assert.deepEqual(catches, events);
  const names = [...source.matchAll(/\bon\(\s*([^,]+),/g)].map((m) => m[1]);
  assert.deepEqual(names, events.map((name) => `'${name}'`));
});

test('the unreplaced payload, a missing HERDR_ENV or a missing pane starts nothing', async () => {
  for (const variant of ['payload', 'env', 'env-zero', 'pane', 'pane-empty']) {
    const { hooks } = await load(variant === 'payload' ? null : runtime);
    const { $, state } = host();
    if (variant === 'env') delete state.env.HERDR_ENV;
    if (variant === 'env-zero') state.env.HERDR_ENV = '0';
    if (variant === 'pane') delete state.env.HERDR_PANE_ID;
    if (variant === 'pane-empty') state.env.HERDR_PANE_ID = '';
    await fire(hooks, 'session.measure', $, measure(200000));
    await fire(hooks, 'session.start', $);
    await fire(hooks, 'classic.SessionStart', $, { session_id: 'session-a' });
    assert.equal(state.runs.length, 0, variant);
  }
});

test('runs the reporter with argv only and a two second timeout', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, measure(200000));
  assert.equal(state.runs.length, 1);
  const [{ argv, init }] = state.runs;
  assert.deepEqual(argv, [runtime, '--report', 'claude', 'w1:p1', String(start * 1000), 'session-a', '200000']);
  assert.ok(argv.every((value) => typeof value === 'string'));
  assert.deepEqual(init, { timeoutMs: 2000 });
});

test('strips one .jsonl suffix and passes other ids through', async () => {
  for (const [id, sent] of [
    ['session-a.jsonl', 'session-a'],
    ['session-a.jsonl.jsonl', 'session-a.jsonl'],
    ['../other', '../other'],
    ['session-a', 'session-a'],
  ]) {
    const { hooks } = await load();
    const { $, state } = host();
    state.id = id;
    await fire(hooks, 'session.measure', $, measure(200000));
    assert.equal(state.runs[0].argv[5], sent, id);
  }
  for (const id of [undefined, null, 42, { id: 'x' }]) {
    const { hooks } = await load();
    const { $, state } = host();
    state.id = id;
    await fire(hooks, 'session.measure', $, measure(200000));
    assert.equal(state.runs.length, 0, String(id));
  }
});

test('skips invalid windows and a measure event with no context', async () => {
  for (const window of [0, -1, 1.5, '200000', 100000001, Number.NaN, Infinity, undefined, null]) {
    const { hooks } = await load();
    const { $, state } = host();
    await fire(hooks, 'session.measure', $, measure(window));
    state.window = window;
    await fire(hooks, 'session.start', $);
    await fire(hooks, 'classic.SessionStart', $, { session_id: 'session-a' });
    assert.equal(state.runs.length, 0, String(window));
  }
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, { rateLimits: {}, cost: {} });
  await fire(hooks, 'session.measure', $, undefined);
  assert.equal(state.runs.length, 0);
  for (const window of [1, 100000000]) {
    await fire(hooks, 'session.measure', $, measure(window));
    await exit(state.runs.at(-1), 0);
  }
  assert.deepEqual(state.runs.map((run) => run.argv[6]), ['1', '100000000']);
});

test('a window that moved a whole point sends both windows after the window (path A)', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  const changed = ['context', 'rateLimits'];
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(12), seven(40.5)], changed }), []);
  const second = await step(hooks, $, state, { rateLimits: [seven(40.5), five(13)], changed });
  assert.deepEqual(second, ['five_hour', '13', String(S + 3600), 'seven_day', '40.5', String(S + 86400)]);
  assert.deepEqual(state.runs[1].argv, [...head(start * 1000 + 1), ...second]);
  assert.deepEqual(state.runs[1].init, { timeoutMs: 2000 });
  // A seven_day window that appears is fresh too; an unchanged one is not.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(13)], changed }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(13), seven(41)], changed }), [
    'five_hour', '13', String(S + 3600), 'seven_day', '41', String(S + 86400),
  ]);
});

test('a strictly grown cost total sends the unchanged windows (path B)', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  const rateLimits = [five(23.5), seven(7)];
  assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 1.5 } }), []);
  const grown = await step(hooks, $, state, { rateLimits, cost: { usd: 1.75 }, changed: ['context', 'cost'] });
  assert.deepEqual(grown, ['five_hour', '23.5', String(S + 3600), 'seven_day', '7', String(S + 86400)]);
  // The same total named as changed is not fresh.
  assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 1.75 }, changed: ['context', 'cost'] }), []);
});

test('rewinds, compactions and a total named without growth are not fresh; growth is', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  const rateLimits = [five(30), seven(60)];
  const tail = ['five_hour', '30', String(S + 3600), 'seven_day', '60', String(S + 86400)];
  const drop = { tokens: 4, window: 200000, percent: 0 };
  assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 2 }, changed: all }), []);
  // Rewind: only the context changed, same total and windows.
  assert.deepEqual(await step(hooks, $, state, { rateLimits, context: drop, cost: { usd: 2 } }), []);
  // Compaction without cost growth, also with the cost named at an equal or lower total.
  assert.deepEqual(await step(hooks, $, state, { rateLimits, context: drop, cost: { usd: 2 }, changed: both }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits, context: drop, cost: { usd: 1.5 }, changed: both }), []);
  // A larger total without the cost named is not fresh, and it still moves the baseline.
  assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 4 } }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 3 }, changed: both }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 4.5 }, changed: both }), tail);
  // A rewind-shaped measurement whose total grew (a priced call folded in) is fresh.
  assert.deepEqual(await step(hooks, $, state, { rateLimits, context: drop, cost: { usd: 4.75 }, changed: both }), tail);
  assert.equal(state.runs.length, 8, 'every measurement still reports its window');
  assert.ok(state.runs.every((run) => run.argv[6] === '200000'));
});

test('the first measurement after load, session.end or for another session id is not fresh', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  const tail = (used) => ['five_hour', used, String(S + 3600), 'seven_day', '60', String(S + 86400)];
  const at = (used, usd, changed = all) => ({ rateLimits: [five(used), seven(60)], cost: { usd }, changed });
  assert.deepEqual(await step(hooks, $, state, at(31, 5)), []);
  assert.deepEqual(await step(hooks, $, state, at(32, 6)), tail('32'));
  await fire(hooks, 'session.end', $, { reason: 'clear' });
  assert.deepEqual(await step(hooks, $, state, at(33, 7)), []);
  assert.deepEqual(await step(hooks, $, state, at(33, 8, both)), tail('33'));
  // Cost growth for another session id is that session's first measurement.
  state.id = 'session-b';
  assert.deepEqual(await step(hooks, $, state, at(34, 9)), []);
  assert.equal(state.runs.at(-1).argv[5], 'session-b');
  assert.deepEqual(await step(hooks, $, state, at(34, 10, both)), tail('34'));
  state.id = 'session-a';
  assert.deepEqual(await step(hooks, $, state, at(35, 11)), []);
  assert.deepEqual(await step(hooks, $, state, at(35, 12, both)), tail('35'));
});

test('an absent or invalid cost on either side is not fresh by the cost path', async () => {
  const tail = ['five_hour', '5', String(S + 3600), 'seven_day', '6', String(S + 86400)];
  const rateLimits = [five(5), seven(6)];
  for (const bad of [absent, undefined, null, 5, 'x', {}, { usd: '5' }, { usd: null }, { usd: Number.NaN },
    { usd: Infinity }, { usd: -1 }, { usd: -Infinity }]) {
    const label = typeof bad === 'symbol' ? 'absent' : JSON.stringify(bad) ?? String(bad);
    const { hooks } = await load();
    const { $, state } = host();
    assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 1 } }), [], label);
    // Invalid now, over a valid baseline.
    assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: bad, changed: both }), [], label);
    // Valid now, over the invalid baseline.
    assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 2 }, changed: both }), [], label);
    // Valid growth over a valid baseline.
    assert.deepEqual(await step(hooks, $, state, { rateLimits, cost: { usd: 2.5 }, changed: both }), tail, label);
  }
});

test('a spend_limit window blocks every tail until session.end, cost growth or not', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  const tail = (used) => ['five_hour', used, String(S + 3600), 'seven_day', '20', String(S + 86400)];
  const spend = { kind: 'spend_limit', percentUsed: 104.5 };
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(10), seven(20)], cost: { usd: 1 } }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(11), seven(20)], cost: { usd: 2 }, changed: all }), tail('11'));
  // The window run still goes ahead, with four values.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(12), seven(20), spend], cost: { usd: 3 }, changed: all }), []);
  // Later measurements without spend_limit still send no tail.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(13), seven(20)], cost: { usd: 4 }, changed: all }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(13), seven(20)], cost: { usd: 5 }, changed: both }), []);
  await fire(hooks, 'session.end', $, { reason: 'logout' });
  state.id = 'session-b';
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(14), seven(20)], cost: { usd: 1 }, changed: all }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(14), seven(20)], cost: { usd: 2 }, changed: both }), tail('14'));
  assert.equal(state.runs.length, 7);
});

test('a fresh measurement without a five_hour or seven_day window sends the four-value run; the cost never leaves', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  const costs = [12.345, 13.345, 14.345, 15.345];
  const other = { kind: 'opus_weekly', percentUsed: 5, resetsAt: iso(S + 3600) };
  assert.deepEqual(await step(hooks, $, state, { cost: { usd: costs[0] } }), []);
  assert.deepEqual(await step(hooks, $, state, { cost: { usd: costs[1] }, changed: both }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [other], cost: { usd: costs[2] }, changed: all }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [other, seven(5)], cost: { usd: costs[3] }, changed: both }), [
    'seven_day', '5', String(S + 86400),
  ]);
  assert.deepEqual(state.runs.map((run) => run.argv.length), [7, 7, 7, 10]);
  for (const { argv } of state.runs) {
    for (const value of argv) {
      assert.ok(!value.includes('345'), value);
      assert.ok(!costs.some((usd) => value === String(usd)), value);
    }
  }
});

test('changed units without rateLimits or cost, a window leaving, status-only and reset-only changes are not fresh', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  const flagged = ['rateLimits'];
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(40), seven(50)] }), []);
  // A moved window with only the context named.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(41), seven(50)] }), []);
  // seven_day left.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(41)], changed: flagged }), []);
  // A limit status change with the same used values.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(41)], changed: flagged }), []);
  // A reset time that moved alone.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(41, S + 7200)], changed: flagged }), []);
  // seven_day appearing again is fresh, and carries the moved reset time.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(41, S + 7200), seven(50)], changed: flagged }), [
    'five_hour', '41', String(S + 7200), 'seven_day', '50', String(S + 86400),
  ]);
});

test('start and classic events never send rate limits or touch the baseline', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(10), seven(20)], cost: { usd: 1 } }), []);
  const loaded = { session_id: 'session-a', rateLimits: [five(99), seven(99)], cost: { usd: 50 }, changed: all };
  // New windows, so neither event is deduplicated on the confirmed key.
  state.window = 100000;
  await fire(hooks, 'session.start', $, loaded);
  await exit(state.runs.at(-1), 0);
  state.window = 150000;
  await fire(hooks, 'classic.SessionStart', $, loaded);
  await exit(state.runs.at(-1), 0);
  assert.deepEqual(state.runs.map((run) => run.argv.length), [7, 7, 7]);
  // Neither cleared nor replaced the baseline: growth over 1 with the old values is fresh.
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(10), seven(20)], cost: { usd: 1.5 }, changed: both }), [
    'five_hour', '10', String(S + 3600), 'seven_day', '20', String(S + 86400),
  ]);
});

test('a measurement skipped while a run is in flight moves the baseline and is not replayed', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, measure(200000, { rateLimits: [five(20), seven(30)], cost: { usd: 1 } }));
  state.now = start + 1000;
  const skipped = { rateLimits: [five(21), seven(30)], cost: { usd: 2 }, changed: all };
  await fire(hooks, 'session.measure', $, measure(200000, skipped));
  assert.equal(state.runs.length, 1, 'skipped while the first run is in flight');
  await exit(state.runs[0], 0);
  await turn();
  assert.equal(state.runs.length, 1, 'not replayed when the run settles');
  // The same values and total again: the skipped measurement is the baseline.
  assert.deepEqual(await step(hooks, $, state, skipped), []);
  const S1 = S + 1;
  assert.deepEqual(await step(hooks, $, state, { ...skipped, cost: { usd: 2.5 }, changed: both }), [
    'five_hour', '21', String(S + 3600), 'seven_day', '30', String(S + 86400),
  ]);
  assert.equal(Number(state.runs.at(-1).argv[4]) - (Number(state.runs.at(-1).argv[4]) % 1000000), S1 * 1000000);
});

test('percent text is built from integer tenths and other values drop the window', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  let usd = 1;
  assert.deepEqual(await step(hooks, $, state, { cost: { usd } }), []);
  const control = ['seven_day', '1', String(S + 86400)];
  for (const [p, text] of [[0, '0'], [7, '7'], [23.5, '23.5'], [99.9, '99.9'], [100, '100'], [0.7, '0.7'],
    [0.1 + 0.2, '0.3'], [23.500000001, '23.5'], [99.90000000001, '99.9'], [-0, '0'], [57, '57'], [1.1 * 3, '3.3']]) {
    usd += 1;
    assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(p), seven(1)], cost: { usd }, changed: both }),
      ['five_hour', text, String(S + 3600), ...control], String(p));
  }
  for (const p of [1.25, 0.05, 99.94999999999, 100.1, 100.05, -1, -0.1, 101, Number.NaN, Infinity, '5', null, undefined, 1e-3]) {
    usd += 1;
    assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(p), seven(1)], cost: { usd }, changed: both }),
      control, String(p));
  }
});

// Loads a fresh module with the clock at `now`, sets the baseline, and
// returns a function that sends one seven_day window with `resetsAt` as a
// fresh measurement and gives the tail.
async function resets(now) {
  const { hooks } = await load();
  const { $, state } = host();
  state.now = now;
  let usd = 1;
  assert.deepEqual(await step(hooks, $, state, { cost: { usd } }), []);
  return async (resetsAt) => {
    usd += 1;
    return step(hooks, $, state, { rateLimits: [{ kind: 'seven_day', percentUsed: 1, resetsAt }], cost: { usd }, changed: both });
  };
}

test('ISO reset times match a Date.parse oracle, and invalid ones drop the window', async () => {
  const oracle = (text) => ['seven_day', '1', String(Math.floor(Date.parse(text) / 1000))];
  const blocks = [
    [start, [
      '2023-11-15T00:00:00Z', '2023-11-15T00:00:00.5Z', '2023-11-15T00:00:00.999999999Z', '2023-11-15T00:00:00.1Z',
      '2023-11-15T02:30:00+02:30', '2023-11-14T20:00:00-05:00', '2023-11-15T00:00:00+00:00',
      '2023-11-15T00:00:00-00:00', '2023-11-20T23:59:59+23:59', '2023-11-15T00:00:00-23:59', '2023-11-21T00:00:00Z',
    ], [
      '2023-11-15T24:00:00Z', '2023-11-15T23:60:00Z', '2023-11-15T23:59:60Z', '2023-11-16T00:00:00+24:00',
      '2023-11-16T00:00:00-24:00', '2023-11-16T00:00:00+02:60', '2023-11-15 00:00:00Z', '2023-11-15T00:00:00', '2023-11-15T00:00Z',
      '2023-11-15T00:00:00.Z', '2023-11-15T00:00:00.1234567890Z', '2023-11-15T00:00:00z', '2023-11-15T00:00:00+02:00Z',
      '+02023-11-15T00:00:00Z', ' 2023-11-15T00:00:00Z', '2023-11-15T00:00:00Z\n', '2023-11-15T00:00:00+0200',
      '2023-11-15T00:00:00+02', '2023-11-15', '20231115T000000Z', 1700050000, null, undefined, {},
    ]],
    [Date.parse('2023-02-27T00:00:00Z'), ['2023-02-28T23:59:59Z', '2023-03-01T00:00:00Z'], ['2023-02-29T00:00:00Z']],
    [Date.parse('2023-11-29T00:00:00Z'), ['2023-11-30T12:00:00Z', '2023-12-01T00:00:00Z'], ['2023-11-31T00:00:00Z', '2023-12-00T00:00:00Z']],
    [Date.parse('2023-12-31T12:00:00Z'), ['2024-01-01T01:00:00+01:00', '2024-01-02T00:00:00Z'], ['2023-12-32T00:00:00Z']],
    [Date.parse('2024-02-28T12:00:00Z'), ['2024-02-29T10:00:00Z', '2024-03-01T00:00:00Z'], ['2024-02-30T00:00:00Z']],
    [Date.parse('2100-02-27T00:00:00Z'), ['2100-02-28T12:00:00Z', '2100-03-01T00:00:00Z'], ['2100-02-29T00:00:00Z']],
  ];
  for (const [now, valid, invalid] of blocks) {
    const send = await resets(now);
    for (const text of valid) assert.deepEqual(await send(text), oracle(text), text);
    for (const text of invalid) assert.deepEqual(await send(text), [], String(text));
  }
});

test('reset bounds come from the sent seq, at both edges', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  state.now = start + 500;
  let usd = 1;
  assert.deepEqual(await step(hooks, $, state, { cost: { usd } }), []);
  const send = async (kind, seconds) => {
    usd += 1;
    return step(hooks, $, state, { rateLimits: [limit(kind, 1, seconds)], cost: { usd }, changed: both });
  };
  const kept = (kind, seconds) => [kind, '1', String(seconds)];
  // The first whole second after seq / 1e6 is kept, any earlier one dropped.
  assert.deepEqual(await send('five_hour', S + 1), kept('five_hour', S + 1));
  assert.deepEqual(await send('five_hour', S), []);
  assert.deepEqual(await send('seven_day', S - 1), []);
  // The last second inside S + D + 3600 is kept, the next dropped.
  assert.deepEqual(await send('five_hour', S + 21600), kept('five_hour', S + 21600));
  assert.deepEqual(await send('five_hour', S + 21601), []);
  assert.deepEqual(await send('seven_day', S + 608400), kept('seven_day', S + 608400));
  assert.deepEqual(await send('seven_day', S + 608401), []);
  // The clock steps back an hour, so lastSeq + 1 sets seq and its second.
  state.now = start - 3_600_000;
  const previous = Number(state.runs.at(-1).argv[4]);
  assert.deepEqual(await send('five_hour', S + 21600), kept('five_hour', S + 21600));
  assert.equal(state.runs.at(-1).argv[4], String(previous + 1));
  assert.deepEqual(await send('five_hour', S - 1800), []);
  assert.deepEqual(await send('five_hour', S + 1), kept('five_hour', S + 1));
});

test('a repeated kind drops the tail; five_hour comes first and other entries are ignored', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(1), seven(3)], cost: { usd: 1 } }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [five(1), five(2), seven(3)], cost: { usd: 2 }, changed: all }), []);
  assert.deepEqual(await step(hooks, $, state, { rateLimits: [seven(3), seven(4), five(2)], cost: { usd: 3 }, changed: all }), []);
  assert.deepEqual(
    await step(hooks, $, state, { rateLimits: [null, 5, 'five_hour', { kind: 'opus' }, seven(3), five(2)], cost: { usd: 4 }, changed: both }),
    ['five_hour', '2', String(S + 3600), 'seven_day', '3', String(S + 86400)],
  );
  assert.deepEqual(state.runs.map((run) => run.argv.length), [7, 7, 7, 13]);
});

test('start and classic skip a confirmed key while measure still reports', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.start', $);
  assert.equal(state.runs.length, 1);
  await exit(state.runs[0], 0);
  await fire(hooks, 'session.start', $);
  await fire(hooks, 'classic.SessionStart', $, { session_id: 'session-a' });
  assert.equal(state.runs.length, 1, 'deduplicated on the confirmed key');
  await fire(hooks, 'session.measure', $, measure(200000));
  assert.equal(state.runs.length, 2, 'session.measure is never deduplicated');
  await exit(state.runs[1], 0);
  await fire(hooks, 'session.measure', $, measure(200000));
  assert.equal(state.runs.length, 3);
  await exit(state.runs[2], 0);
  state.window = 100000;
  await fire(hooks, 'session.start', $);
  assert.equal(state.runs.length, 4, 'a new window is a new key');
});

test('a non-zero exit or a rejection is retried on the next event', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.start', $);
  await exit(state.runs[0], 3);
  await fire(hooks, 'session.start', $);
  assert.equal(state.runs.length, 2);
  state.runs[1].reject(new Error('runtime missing'));
  await turn();
  await fire(hooks, 'classic.SessionStart', $, { session_id: 'session-a' });
  assert.equal(state.runs.length, 3);
});

test('an event during a run starts nothing and nothing starts when it settles', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, measure(200000));
  state.now += 1000;
  await fire(hooks, 'session.measure', $, measure(100000));
  await fire(hooks, 'session.start', $);
  assert.equal(state.runs.length, 1);
  await exit(state.runs[0], 0);
  await turn();
  assert.equal(state.runs.length, 1, 'no held sample is started from the settled run');
  await fire(hooks, 'session.measure', $, measure(100000));
  assert.equal(state.runs.length, 2);
  assert.equal(state.runs[1].argv[6], '100000');
});

test('a run that never settles is abandoned after three seconds', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, measure(200000));
  state.now = start + 3000;
  await fire(hooks, 'session.measure', $, measure(100000));
  assert.equal(state.runs.length, 1, 'still in flight at exactly three seconds');
  state.now = start + 3001;
  await fire(hooks, 'session.measure', $, measure(100000));
  assert.equal(state.runs.length, 2);
  assert.equal(state.runs[1].argv[6], '100000');
});

test('a late first run neither confirms its key nor clears the newer run', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.start', $);
  state.now = start + 3001;
  await fire(hooks, 'session.measure', $, measure(100000));
  assert.equal(state.runs.length, 2);
  await exit(state.runs[0], 0);
  state.now = start + 4000;
  await fire(hooks, 'session.measure', $, measure(100000));
  assert.equal(state.runs.length, 2, 'the newer run is still in flight');
  await exit(state.runs[1], 1);
  await fire(hooks, 'session.start', $);
  assert.equal(state.runs.length, 3, 'the abandoned run did not confirm its key');
  assert.equal(state.runs[2].argv[6], '200000');
});

test('take-over: a skipped event is never sent and no third run starts', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, measure(200000));
  state.now = start + 1000;
  await fire(hooks, 'session.measure', $, measure(150000));
  state.now = start + 3500;
  await fire(hooks, 'session.measure', $, measure(100000));
  await exit(state.runs[1], 0);
  await turn();
  assert.deepEqual(state.runs.map((run) => run.argv[6]), ['200000', '100000']);
});

test('session.end clears the confirmed key', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.start', $);
  await exit(state.runs[0], 0);
  await fire(hooks, 'session.start', $);
  assert.equal(state.runs.length, 1);
  await fire(hooks, 'session.end', $, { reason: 'clear' });
  await fire(hooks, 'session.start', $);
  assert.equal(state.runs.length, 2);
});

test('the confirmed key carries the session id across session.end with a run in flight', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, measure(200000));
  await fire(hooks, 'session.end', $, { reason: 'clear' });
  state.id = 'session-b';
  state.now = start + 1000;
  // session.end leaves A's run in flight, so B's start within staleMs is skipped.
  await fire(hooks, 'classic.SessionStart', $, { session_id: 'session-b' });
  assert.equal(state.runs.length, 1, 'the run for A still counts as in flight after session.end');
  // The run for session A settles after the end and confirms A's key.
  await exit(state.runs[0], 0);
  await fire(hooks, 'classic.SessionStart', $, { session_id: 'session-b' });
  assert.equal(state.runs.length, 2, 'B with the same window is not taken as confirmed');
  assert.equal(state.runs[1].argv[5], 'session-b');
  assert.equal(state.runs[1].argv[6], '200000');
});

test('session.end keeps the last sequence sent', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, measure(200000));
  await exit(state.runs[0], 0);
  await fire(hooks, 'session.end', $, { reason: 'clear' });
  // The clock has not moved, so only lastSeq keeps the sequence increasing.
  await fire(hooks, 'session.measure', $, measure(200000));
  assert.equal(state.runs.length, 2);
  assert.equal(state.runs[1].argv[4], String(start * 1000 + 1));
});

test('a clock reading earlier than the run start treats the run as stale', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  await fire(hooks, 'session.measure', $, measure(200000));
  // The run never settles, and the clock steps back an hour.
  state.now = start - 3_600_000;
  await fire(hooks, 'session.measure', $, measure(100000));
  assert.equal(state.runs.length, 2);
  // The sequence still increases, from the last one sent.
  assert.equal(state.runs[1].argv[4], String(start * 1000 + 1));
  assert.equal(state.runs[1].argv[6], '100000');
});

test('overlapping dispatches start one run when the earlier reading arrives last', async () => {
  const { hooks } = await load();
  // Each hook gets its own $ whose clock the test resolves by hand, so the
  // result does not depend on which hook reads the clock first.
  const shared = host().state;
  const manual = () => {
    const { $ } = host();
    const clock = {};
    clock.reading = new Promise((resolve) => { clock.resolve = resolve; });
    $.clock = { now: () => clock.reading };
    $.process = { run: (argv, init) => shared.runs.push({ argv, init }) && new Promise(() => {}) };
    return { $, clock };
  };
  const b = manual();
  const a = manual();
  const firedB = fire(hooks, 'session.start', b.$);
  const firedA = fire(hooks, 'session.measure', a.$, measure(200000));
  a.clock.resolve(start + 5);
  await firedA;
  b.clock.resolve(start);
  await firedB;
  assert.equal(shared.runs.length, 1, 'a run five milliseconds old is in flight');
  assert.equal(shared.runs[0].argv[4], String((start + 5) * 1000));
});

test('sequences increase strictly, also for equal clock readings', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  const seqs = [];
  for (const now of [start, start, start + 0.5, start - 1000, start + 1]) {
    state.now = now;
    await fire(hooks, 'session.measure', $, measure(200000));
    seqs.push(Number(state.runs.at(-1).argv[4]));
    await exit(state.runs.at(-1), 0);
  }
  assert.deepEqual(seqs, [start * 1000, start * 1000 + 1, start * 1000 + 2, start * 1000 + 3, (start + 1) * 1000]);
});

test('a clock that is not epoch milliseconds starts nothing', async () => {
  for (const now of [1_700_000_000, 0, -1, '1700000000000', Number.NaN, Infinity, undefined, null, 1.7e15, 1.7e18]) {
    const { hooks } = await load();
    const { $, state } = host();
    state.now = now;
    await fire(hooks, 'session.measure', $, measure(200000));
    assert.equal(state.runs.length, 0, String(now));
  }
});

test('a reading past the safe sequence range leaves the last sequence unchanged', async () => {
  for (const bad of [1.7e15, 1.7e18]) {
    const { hooks } = await load();
    const { $, state } = host();
    state.now = bad;
    await fire(hooks, 'session.measure', $, measure(200000));
    assert.equal(state.runs.length, 0, String(bad));
    state.now = start;
    await fire(hooks, 'session.measure', $, measure(200000));
    assert.equal(state.runs.length, 1, String(bad));
    assert.equal(state.runs[0].argv[4], String(start * 1000), String(bad));
  }
});

test('an undefined or null event passes through every hook unchanged', async () => {
  for (const e of [undefined, null]) {
    for (const name of events) {
      const { hooks } = await load();
      const { $, state } = host();
      await fire(hooks, name, $, e);
      // session.start reads nothing from its event, so it still reports once.
      const runs = name === 'session.start' ? 1 : 0;
      assert.equal(state.runs.length, runs, `${name} with ${e}`);
    }
  }
});

test('classic.SessionStart with another session id is skipped', async () => {
  const { hooks } = await load();
  const { $, state } = host();
  for (const e of [{ session_id: 'session-b' }, {}, undefined, { session_id: 'session-a.jsonl' }]) {
    await fire(hooks, 'classic.SessionStart', $, e);
  }
  assert.equal(state.runs.length, 0);
  state.id = 'session-a.jsonl';
  await fire(hooks, 'classic.SessionStart', $, { session_id: 'session-a' });
  assert.equal(state.runs.length, 1);
});

test('failing clock, run and result calls leave no unhandled rejection', async () => {
  const rejections = [];
  const listener = (reason) => rejections.push(reason);
  process.on('unhandledRejection', listener);
  try {
    for (const failure of ['clock-throws', 'clock-rejects', 'run-throws', 'run-rejects', 'result-throws']) {
      const { hooks } = await load();
      const { $, state } = host();
      if (failure === 'clock-throws') $.clock.now = () => { throw new Error(failure); };
      if (failure === 'clock-rejects') $.clock.now = async () => { throw new Error(failure); };
      const run = $.process.run;
      if (failure === 'run-throws') $.process.run = () => { throw new Error(failure); };
      await fire(hooks, 'session.measure', $, measure(200000));
      if (failure === 'run-rejects') state.runs[0].reject(new Error(failure));
      if (failure === 'result-throws') state.runs[0].resolve({ get exitCode() { throw new Error(failure); } });
      await turn();
      await turn();
      assert.deepEqual(rejections, [], failure);
      // Consistent state: nothing is left in flight, so the next event runs.
      $.clock.now = async () => state.now;
      $.process.run = run;
      const before = state.runs.length;
      await fire(hooks, 'session.measure', $, measure(200000));
      assert.equal(state.runs.length, before + 1, failure);
      if (failure === 'run-throws') {
        assert.equal(Number(state.runs.at(-1).argv[4]), start * 1000 + 1, 'seq stays strictly increasing');
      }
      await exit(state.runs.at(-1), 0);
    }
  } finally {
    process.off('unhandledRejection', listener);
  }
});

test('every hook resolves to next(e) when every $ call throws or rejects', async () => {
  const rejections = [];
  const listener = (reason) => rejections.push(reason);
  process.on('unhandledRejection', listener);
  try {
    for (const mode of ['throws', 'rejects']) {
      const fail = mode === 'throws' ? () => { throw new Error(mode); } : async () => { throw new Error(mode); };
      const $ = {
        env: { get: fail },
        session: { id: fail, usage: fail, model: fail },
        clock: { now: fail },
        process: { run: fail },
      };
      const { hooks } = await load();
      for (const name of events) {
        await fire(hooks, name, $, name === 'session.measure' ? measure(200000) : { session_id: 'session-a' });
        await fire(hooks, name, {}, {});
        await fire(hooks, name, null, null);
      }
      const partial = host();
      partial.$.session.usage = fail;
      partial.$.process.run = fail;
      for (const name of events) await fire(hooks, name, partial.$, { session_id: 'session-a' });
      await fire(hooks, 'session.measure', partial.$, measure(200000));
      await turn();
    }
    assert.deepEqual(rejections, []);
  } finally {
    process.off('unhandledRejection', listener);
  }
});

test('the source uses only the mods API, with literal names', () => {
  for (const name of ['import', 'require', 'setTimeout', 'setInterval', 'Date', 'process', 'fetch', 'globalThis']) {
    const bare = new RegExp(`(?<!\\$\\.)\\b${name}\\b`);
    assert.doesNotMatch(source, bare, name);
  }
  assert.match(source, /\$\.process\.run\(/);
  const names = [...source.matchAll(/\$\.env\.get\(([^)]*)\)/g)].map((m) => m[1]);
  assert.deepEqual(names, ["'HERDR_ENV'", "'HERDR_PANE_ID'"]);
  assert.equal(source.split(placeholder).length, 2, 'exactly one runtime declaration');
  assert.match(source, /^export function register\(on\) \{$/m);
});

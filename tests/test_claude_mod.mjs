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
const measure = (window) => ({ context: { tokens: 10, window, percent: 0 }, rateLimits: {}, cost: {}, changed: [] });

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

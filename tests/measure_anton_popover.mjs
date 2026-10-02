#!/usr/bin/env node
// Anton popover measurement harness. Synthetic fixtures only: two fake Herdr
// sockets, a fake SSH that executes the native peer locally, a synthetic Omarchy
// theme and generated JS snapshots. No network, GPU, real accounts or private data.
//
// The harness is shared by the fix-anton-popover-correctness, allowance-contract
// and popover-architecture changes and must keep running against older and newer
// code. Missing files, snapshot fields or State.js exports are reported as null.
// Optional State.js exports it uses when present:
//   receiptTimeoutMs(raw)          SnapshotStore receipt timeout for a snapshot
//   viewSignature(view)            replacement signature (default JSON.stringify)
//   storeStep(store, event, nowMs) full store emulation; event is
//                                  {type: 'receipt', raw} or {type: 'tick'};
//                                  returns true when the visible view was replaced
// The architecture section (restructure-anton-popover) adds static QML counts,
// clock-only view changes and optional Qt 6 qmllint warning counts.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import net from 'node:net';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i < 0 ? fallback : args[i + 1]; };
const flag = name => args.includes(name);
if (flag('--help')) {
  console.log(`Usage: node tests/measure_anton_popover.mjs [options]
  --binary PATH        anton-runtime to measure (default: <source-root>/omarchy/anton-runtime/target/release/anton-runtime)
  --source-root DIR    checkout used for State.js and static metrics (default: this repository)
  --state-js PATH      State.js to project with (default: <source-root>/omarchy/herdr.observatory/State.js)
  --seconds N          fixed runtime window, 5 to 300 (default 30)
  --agents N           agents per synthetic host, 1 to 128 (default 32)
  --repeat N           runtime windows to run; numeric results are medians (default 1)
  --iterations N       JS projection iterations per size (default 400)
  --skip-runtime       JS and static metrics only
  --no-refresh-probe   skip the separate refresh latency and burst probe
  --no-allowance-probe skip the separate allowance wire probe
  --no-claude-probe    skip the separate Claude transcript probes
  --claude-agents N    Claude agents per synthetic host in the Claude probes, 1 to 16 (default 4)
  --claude-seconds N   fixed window of each Claude probe, 10 to 300 (default 30)
  --claude-large-mb N  bytes per transcript in the large Claude variant, in MB, 0 to 32 (default 6; 0 skips it)
  --claude-old-local PATH  also run each Claude variant with PATH as the local runtime and
                       the measured binary as the fake-SSH peer (old local, new peer)
  --colors-method M    colors.toml open counting: auto, inotify, strace or none (default auto)
  --json               print JSON only

colors.toml opens: auto uses inotifywait (inotify-tools) on the fixture theme
directory during the timed window. Without it, strace counts successful open
calls in a separate untimed window of the same length, so ptrace overhead never
reaches the CPU and RSS figures. With neither tool the count is 'unmeasured'.`);
  process.exit(0);
}
const here = path.dirname(fileURLToPath(import.meta.url));
const sourceRoot = path.resolve(option('--source-root', path.join(here, '..')));
const binary = path.resolve(option('--binary', path.join(sourceRoot, 'omarchy/anton-runtime/target/release/anton-runtime')));
const stateJs = path.resolve(option('--state-js', path.join(sourceRoot, 'omarchy/herdr.observatory/State.js')));
const seconds = Number(option('--seconds', 30));
const count = Number(option('--agents', 32));
const repeat = Number(option('--repeat', 1));
const iterations = Number(option('--iterations', 400));
const colorsMethod = option('--colors-method', 'auto');
if (!['auto', 'inotify', 'strace', 'none'].includes(colorsMethod)) throw new Error('Invalid --colors-method');
if (!(seconds >= 5 && seconds <= 300 && count >= 1 && count <= 128 && repeat >= 1 && repeat <= 9 && iterations >= 10))
  throw new Error('Invalid duration, agent count, repeat or iteration count');
const claudeCount = Number(option('--claude-agents', 4));
const claudeSeconds = Number(option('--claude-seconds', 30));
const claudeLargeMb = Number(option('--claude-large-mb', 6));
if (!(Number.isInteger(claudeCount) && claudeCount >= 1 && claudeCount <= 16 && claudeSeconds >= 10 && claudeSeconds <= 300 && claudeLargeMb >= 0 && claudeLargeMb <= 32))
  throw new Error('Invalid Claude agent count, window or transcript size');
const claudeOldLocal = option('--claude-old-local', null) === null ? null : path.resolve(option('--claude-old-local', null));
if (claudeOldLocal !== null && !fs.statSync(claudeOldLocal, { throwIfNoEntry: false })?.isFile()) throw new Error('--claude-old-local is not a regular file');

const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const median = values => { const v = values.filter(Number.isFinite).sort((a, b) => a - b); return v.length ? (v.length % 2 ? v[(v.length - 1) / 2] : (v[v.length / 2 - 1] + v[v.length / 2]) / 2) : null; };
const round = (value, places = 3) => Number.isFinite(value) ? Number(value.toFixed(places)) : value ?? null;
const readText = file => { try { return fs.readFileSync(file, 'utf8'); } catch { return null; } };
const which = name => { try { return execFileSync('/bin/sh', ['-c', `command -v ${name}`], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim() || null; } catch { return null; } };

// ---------------------------------------------------------------- runtime
function herdrSnapshot(prefix) {
  const binding = 'a'.repeat(64);
  return { protocol: 1, version: 'fixture', workspaces: [{ workspace_id: 'w1', label: 'Synthetic', worktree: { checkout_path: '/synthetic/branch' } }], agents: Array.from({ length: count }, (_, i) => ({
    pane_id: `w1:${prefix}${i + 1}`, workspace_id: 'w1', cwd: '/synthetic/branch', agent: 'pi', agent_status: i % 2 ? 'idle' : 'working',
    agent_session: { agent: 'pi', source: 'herdr:pi', kind: 'path', value: '/synthetic/session' },
    tokens: { obs_v: '2', obs_bind: binding, obs_seq: '1700000000000000', obs_event: 'output', obs_phase: 'output', obs_n0: '12345,678,12000,0', obs_n1: '4000,128000,1700000000000000,90000', obs_n2: '800,88000,0,2000', obs_n3: '2,3', obs_usage_source: 'pi-extension' }
  })) };
}
async function herdrServer(file, raw) {
  const server = net.createServer(socket => {
    server.sockets.add(socket); socket.on('close', () => server.sockets.delete(socket)); socket.on('error', () => {});
    let buffer = '';
    socket.on('data', bytes => { buffer += bytes; if (!buffer.includes('\n')) return;
      let request; try { request = JSON.parse(buffer.split('\n')[0]); } catch { socket.destroy(); return; }
      server.calls.push(performance.now());
      socket.end(JSON.stringify({ jsonrpc: '2.0', id: request.id, result: { snapshot: raw } }) + '\n');
    });
  });
  server.sockets = new Set(); server.calls = [];
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(file, resolve); });
  return server;
}
const ticks = Number(execFileSync('getconf', ['CLK_TCK'], { encoding: 'utf8' }).trim());
const pageKib = Number(execFileSync('getconf', ['PAGESIZE'], { encoding: 'utf8' }).trim()) / 1024;
function processes() {
  const rows = new Map();
  for (const id of fs.readdirSync('/proc').filter(v => /^\d+$/.test(v))) {
    try {
      const stat = fs.readFileSync(`/proc/${id}/stat`, 'utf8'); const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
      rows.set(Number(id), { pid: Number(id), ppid: Number(fields[1]), rss: Number(fields[21]) * pageKib });
    } catch {}
  }
  return rows;
}
// colors.toml opens by the local runtime and its fake-SSH peer (same HOME).
// inotify watches the timed window; strace needs its own window (see --help).
function colorsCounter() {
  const inotify = which('inotifywait'), strace = which('strace');
  if (colorsMethod === 'none') return { method: 'unmeasured' };
  if (inotify && colorsMethod !== 'strace') return { method: 'inotify', inotify };
  if (strace && colorsMethod !== 'inotify') return { method: 'strace', strace };
  return { method: 'unmeasured' };
}
async function themeWatcher(dir, counter) {
  const inotify = counter.inotify;
  if (counter.method !== 'inotify') return { method: counter.method, count: () => null, stop: async () => {} };
  const child = spawn(inotify, ['-m', '-q', '-e', 'open', '--format', '%f', dir], { stdio: ['ignore', 'pipe', 'pipe'] });
  let opens = 0, text = '';
  child.stdout.on('data', data => { text += data; let i; while ((i = text.indexOf('\n')) >= 0) { if (text.slice(0, i) === 'colors.toml') opens++; text = text.slice(i + 1); } });
  // -q suppresses "Watches established"; poll by opening a sentinel until seen.
  const sentinel = path.join(dir, '.ready');
  fs.writeFileSync(sentinel, '');
  let ready = false; child.stdout.on('data', data => { if (String(data).includes('.ready')) ready = true; });
  for (let i = 0; i < 100 && !ready; i++) { fs.closeSync(fs.openSync(sentinel, 'r')); await delay(20); }
  if (!ready) { child.kill(); return { method: 'unmeasured', count: () => null, stop: async () => {} }; }
  await delay(50); opens = 0;
  return { method: 'inotify', count: () => opens, stop: async () => { child.kill(); await new Promise(r => child.once('exit', r)); } };
}
function fixture(base, label, localBinary = binary) {
  const dir = path.join(base, label); const root = path.join(dir, 'plugin'); const peer = path.join(dir, 'peer'); const home = path.join(dir, 'home');
  const theme = path.join(home, '.local/state/omarchy/current/theme');
  for (const p of [dir, root, peer, path.join(dir, 'bin'), path.join(dir, 'state'), path.join(dir, 'peer-state'), home]) fs.mkdirSync(p, { recursive: true, mode: 0o700 });
  fs.mkdirSync(theme, { recursive: true });
  fs.writeFileSync(path.join(theme, 'colors.toml'), 'accent = "#123456"\nbackground = "#101010"\nforeground = "#eeeeee"\n');
  fs.writeFileSync(path.join(home, '.local/state/omarchy/current/theme.name'), 'synthetic\n');
  for (const p of [root, peer]) { fs.copyFileSync(p === root ? localBinary : binary, path.join(p, 'anton-runtime')); fs.chmodSync(path.join(p, 'anton-runtime'), 0o755); fs.writeFileSync(path.join(p, '.herdr-observatory-install'), 'herdr.observatory\n', { mode: 0o600 }); }
  const localSocket = path.join(dir, 'local.sock'), remoteSocket = path.join(dir, 'remote.sock');
  fs.writeFileSync(path.join(root, '.config.json'), JSON.stringify({ interval: 5, hosts: [{ id: 'local', socket_path: localSocket }, { id: 'remote', transport: 'ssh', target: 'fixture', socket_path: remoteSocket }] }), { mode: 0o600 });
  fs.writeFileSync(path.join(peer, '.config.json'), JSON.stringify({ hosts: [{ id: 'remote', socket_path: remoteSocket }] }), { mode: 0o600 });
  fs.writeFileSync(path.join(dir, 'bin/ssh'), '#!/bin/sh\nfor arg do last=$arg; done\ncase $last in\n *--probe*) exec "$ANTON_TEST_PEER/anton-runtime" --root "$ANTON_TEST_PEER" --state "$ANTON_TEST_PEER_STATE" --probe;;\n *) exit 99;;\nesac\n', { mode: 0o755 });
  const env = { ...process.env, HOME: home, CODEX_HOME: path.join(home, '.codex'), XDG_STATE_HOME: path.join(dir, 'xdg-state'), PATH: `${path.join(dir, 'bin')}:/usr/bin:/bin`, ANTON_TEST_PEER: peer, ANTON_TEST_PEER_STATE: path.join(dir, 'peer-state'), ANTON_TEST_TIMING: path.join(dir, 'time.txt') };
  delete env.OBSERVATORY_SSH_CONFIG;
  return { dir, root, theme, env, localSocket, remoteSocket, timefile: env.ANTON_TEST_TIMING };
}
function launch(f, trace = null) {
  const command = [path.join(f.root, 'anton-runtime'), '--root', f.root, '--state', path.join(f.dir, 'state')];
  // -ff writes one file per process so unfinished and resumed calls never interleave; -z keeps successful calls only.
  if (trace) command.unshift(trace.strace, '-f', '-ff', '-qq', '-z', '-e', 'trace=open,openat,openat2', '-o', trace.prefix);
  const child = spawn('/bin/bash', ['-c', 'TIMEFORMAT="%U %S"; { time "$@"; } 2>"$ANTON_TEST_TIMING"', 'anton-measure', ...command], { env: f.env, stdio: ['pipe', 'pipe', 'pipe'] });
  const run = { child, frames: [], errors: '', exit: false, listeners: new Set() };
  let buffer = '';
  child.stdout.on('data', data => { buffer += data; let i; while ((i = buffer.indexOf('\n')) >= 0) { const line = buffer.slice(0, i); buffer = buffer.slice(i + 1);
    const frame = { at: performance.now(), bytes: Buffer.byteLength(line) + 1, value: JSON.parse(line) }; run.frames.push(frame); for (const l of run.listeners) l(frame); } });
  child.stderr.on('data', data => { run.errors = (run.errors + data).slice(-4096); });
  run.done = new Promise((resolve, reject) => { child.once('error', reject); child.once('exit', (code, signal) => { run.exit = true; resolve({ code, signal }); }); });
  run.until = (predicate, ms) => new Promise(resolve => {
    const hit = run.frames.find(f => predicate(f)); if (hit) return resolve(hit);
    const timer = setTimeout(() => { run.listeners.delete(listener); resolve(null); }, ms);
    const listener = f => { if (predicate(f)) { clearTimeout(timer); run.listeners.delete(listener); resolve(f); } };
    run.listeners.add(listener);
  });
  run.close = async () => { if (!run.exit) { child.stdin.end(); const r = await Promise.race([run.done, delay(3000).then(() => null)]); if (!r) { child.kill('SIGKILL'); await run.done; throw new Error('Runtime did not stop on owner EOF'); } return r; } return run.done; };
  return run;
}
const online = frame => Array.isArray(frame.value?.hosts) && frame.value.hosts.length === 2 && frame.value.hosts.every(h => h.online === true && Array.isArray(h.agents) && h.agents.length === count);
async function runtimeWindow(base, index, counter) {
  const f = fixture(base, `window-${index}`);
  const servers = [await herdrServer(f.localSocket, herdrSnapshot('l')), await herdrServer(f.remoteSocket, herdrSnapshot('r'))];
  const watcher = await themeWatcher(f.theme, counter);
  const run = launch(f); const start = performance.now();
  let peakRss = 0;
  try {
    while (performance.now() - start < seconds * 1000) {
      if (run.exit) throw new Error(`Runtime exited early: ${run.errors}`);
      const all = processes(); const ids = new Set([run.child.pid]); let changed = true;
      while (changed) { changed = false; for (const p of all.values()) if (ids.has(p.ppid) && !ids.has(p.pid)) { ids.add(p.pid); changed = true; } }
      peakRss = Math.max(peakRss, [...ids].filter(id => id !== run.child.pid).reduce((sum, id) => sum + (all.get(id)?.rss || 0), 0));
      await delay(50);
    }
    const result = await run.close();
    if (result.code !== 0) throw new Error(`Runtime failed: ${JSON.stringify(result)} ${run.errors}`);
    const connected = run.frames.find(online);
    if (!connected) throw new Error('Both synthetic hosts never reported online; check the fake SSH peer');
    const [user, system] = (readText(f.timefile) || '').trim().split('\n').at(-1).split(' ').map(Number);
    const last = run.frames.at(-1).value;
    return {
      snapshots: run.frames.length,
      mean_snapshot_bytes: run.frames.reduce((s, fr) => s + fr.bytes, 0) / run.frames.length,
      runtime_cpu_seconds: Number.isFinite(user + system) ? user + system : null,
      peak_rss_kib: peakRss,
      colors_toml_opens: watcher.count(),
      colors_toml_method: watcher.method,
      herdr_rpc_local: servers[0].calls.length,
      herdr_rpc_remote: servers[1].calls.length,
      connected_ms: connected.at - start,
      snapshot_keys: Object.keys(last).sort(),
      host_keys: Object.keys(last.hosts[0] || {}).sort(),
      heartbeat_seconds: Number.isFinite(last.heartbeat_seconds) ? last.heartbeat_seconds : null
    };
  } finally {
    if (!run.exit) { run.child.kill('SIGKILL'); await run.done; }
    await watcher.stop();
    for (const s of servers) { for (const socket of s.sockets) socket.destroy(); s.close(); }
  }
}
// Untimed window of the same length under strace; returns successful colors.toml opens.
async function straceWindow(base, index, counter) {
  const f = fixture(base, `strace-${index}`);
  const servers = [await herdrServer(f.localSocket, herdrSnapshot('l')), await herdrServer(f.remoteSocket, herdrSnapshot('r'))];
  const traces = path.join(f.dir, 'strace'); fs.mkdirSync(traces, { mode: 0o700 });
  const run = launch(f, { strace: counter.strace, prefix: path.join(traces, 'open') });
  try {
    await delay(seconds * 1000);
    if (run.exit) throw new Error(`Traced runtime exited early: ${run.errors}`);
    await run.close();
    if (!run.frames.find(online)) throw new Error('Traced runtime hosts never reported online');
    return fs.readdirSync(traces).reduce((sum, file) => sum + readText(path.join(traces, file)).split('\n').filter(line => /colors\.toml"/.test(line) && !/= -1 /.test(line)).length, 0);
  } finally {
    if (!run.exit) { run.child.kill('SIGKILL'); await run.done; }
    for (const s of servers) { for (const socket of s.sockets) socket.destroy(); s.close(); }
  }
}
// Separate short run so the fixed CPU window is comparable across versions.
async function refreshProbe(base) {
  const f = fixture(base, 'refresh');
  const servers = [await herdrServer(f.localSocket, herdrSnapshot('l')), await herdrServer(f.remoteSocket, herdrSnapshot('r'))];
  const run = launch(f);
  try {
    if (!await run.until(online, 20000)) throw new Error('Refresh probe hosts never reported online');
    const localSampled = frame => frame.value?.hosts?.find(h => h.id === 'local')?.sampled_at ?? 0;
    // Send just after an emission so an unrelated heartbeat is not mistaken for a response.
    const anchor = await run.until(fr => fr.at > performance.now() - 5, 6000);
    // sampled_at is a full-precision wall-clock stamp taken before the Herdr
    // call, so only a sample started after the request can answer it.
    const sentWall = Date.now() / 1000, sent = performance.now();
    run.child.stdin.write('refresh\n');
    const fresh = await run.until(fr => fr.at > sent && localSampled(fr) >= sentWall, 6000);
    await delay(3000);
    const burstStart = performance.now(), before = servers[0].calls.length;
    for (let i = 0; i < 20; i++) { run.child.stdin.write('refresh\n'); await delay(10); }
    await delay(3000);
    const burstCalls = servers[0].calls.filter(t => t >= burstStart).length, burstWindow = performance.now() - burstStart;
    const malformed = performance.now();
    run.child.stdin.write('x'.repeat(8192) + '\nunknown-command\n');
    run.child.stdin.write(Buffer.from([0xff, 0xfe, 0x0a]));
    // Still publishing: a snapshot must follow the malformed input (heartbeat at most).
    const after = await run.until(fr => fr.at > malformed + 10, 7000);
    const alive = !run.exit && after !== null;
    const result = await run.close();
    return {
      refresh_latency_ms: fresh && anchor ? round(fresh.at - sent, 1) : null,
      refresh_answered_after_request: fresh !== null && anchor !== null,
      burst_refreshes: 20,
      burst_window_ms: round(burstWindow, 0),
      local_samples_during_burst_window: burstCalls,
      local_samples_before_probe: before,
      survives_malformed_input: alive && result.code === 0
    };
  } finally {
    if (!run.exit) { run.child.kill('SIGKILL'); await run.done; }
    for (const s of servers) { for (const socket of s.sockets) socket.destroy(); s.close(); }
  }
}

// ---------------------------------------------------------------- JS projection
function loadState() {
  if (!fs.existsSync(stateJs)) return null;
  const require = createRequire(import.meta.url);
  delete require.cache[stateJs];
  return require(stateJs);
}
const NOW = 1800000000000;
function jsSnapshot(threads, nowMs, sampledAt) {
  const hosts = ['alpha', 'beta'].map((id, h) => ({ id, label: `Host ${h + 1}`, online: true, connection_state: 'connected', error: null, sampled_at: sampledAt, metrics: null, trend: [], protocol: 1, version: 'fixture', navigation: { profile_id: id, route_key: String(h).repeat(64) }, agents: [] }));
  for (let i = 0; i < threads; i++) {
    const host = hosts[i % 2], state = ['working', 'idle', 'blocked', 'done'][i % 4];
    host.agents.push({ id: `${host.id}:w1:p${i}`, host: host.id, status: state, project: `Project ${i % 7}`, title: `Task ${i}`, branch: `feature/${i}`, checkout: `branch-${i}`, harness: i % 3 ? 'codex' : 'pi', category: 'personal', since: NOW / 1000 - 600,
      technical: { session_generation: i + 1, state_change_seq: i + 10,
        telemetry: { seq: (NOW - 30000) * 1000, usage_seq: (NOW - 30000) * 1000, context: 42000, window: 128000, total_input: 2100000, total_output: 34000, total_cache_read: 1950000, total_uncached_input: 150000, total_cache_write: 0, compactions: 2, subagent_total: 4, subagent_done: 2, subagent_running: 1, subagent_interrupted: 0, subagent_failed: 1, subagent_unknown: 0, subagent_status_seq: (NOW - 30000) * 1000, subagent_starts: 3, subagent_stops: 2, subagent_seq: (NOW - 30000) * 1000 },
        turn_timing: { active: state === 'working', started_at_s: NOW / 1000 - 754, observed_at_s: NOW / 1000 - 1, last_duration_s: 420, last_outcome: 'completed', total_finished_duration_s: 2100, complete: true, freshness_seconds: 12 } } });
  }
  const allowance = (id, remaining) => ({ provider: 'codex', provider_label: 'Codex', account_id: id, label: id, available: true, window_seconds: 604800, sampled_at: NOW / 1000 - 30, weekly_remaining: remaining, weekly_resets_at: NOW / 1000 + 302400, reset_count: 1, reset_expires_at: null, plan: 'pro', lifetime_tokens: 1000000, peak_daily_tokens: 50000, daily_usage: [{ date: '2027-01-10', tokens: 40000 }, { date: '2027-01-11', tokens: 50000 }] });
  return { at: nowMs / 1000, interval: 5, heartbeat_seconds: 4, hosts, allowances: [allowance('one', 60), allowance('two', 30)], fleet_discovery: { state: 'disabled' } };
}
function leaves(value) { if (value === null || typeof value !== 'object') return 1; return Object.values(value).reduce((s, v) => s + leaves(v), 0); }
function projectionMetrics(State) {
  if (!State || typeof State.project !== 'function' || typeof State.stableThreads !== 'function') return null;
  const signature = typeof State.viewSignature === 'function' ? State.viewSignature : JSON.stringify;
  const sizes = {};
  for (const threads of [32, 128]) {
    const raw = jsSnapshot(threads, NOW, NOW / 1000 - 1);
    let previous = State.project(raw, NOW).threads; const samples = [];
    for (let i = 0; i < iterations; i++) {
      const t = performance.now();
      const next = State.project(raw, NOW + i);
      next.threads = State.stableThreads(previous, next.threads);
      signature(next);
      samples.push(performance.now() - t); previous = next.threads;
    }
    sizes[threads] = { median_ms: round(median(samples), 4), p95_ms: round(samples.sort((a, b) => a - b)[Math.floor(samples.length * 0.95)], 4) };
  }
  const view = State.project(jsSnapshot(32, NOW, NOW / 1000 - 1), NOW);
  const fields = obj => obj ? { keys: Object.keys(obj).length, leaves: leaves(obj), names: Object.keys(obj).sort() } : null;
  return { per_update: sizes, view_fields: fields(view), thread_fields: fields(view.threads[0]), host_fields: fields(view.hosts[0]), allowance_fields: fields(view.allowances[0]) };
}
// SnapshotStore emulation: a constant snapshot re-delivered at each runtime
// heartbeat (host sample time advances as the runtime resamples; agent and
// allowance source times stay fixed), with the QML 1 s tick while open.
function replacementMetrics(State) {
  if (!State || typeof State.project !== 'function') return null;
  const timeout = raw => typeof State.receiptTimeoutMs === 'function' ? State.receiptTimeoutMs(raw) : 6000;
  const signature = typeof State.viewSignature === 'function' ? State.viewSignature : JSON.stringify;
  const heartbeat = 4000, duration = 60000;
  const store = { raw: null, lastReceipt: 0, view: State.project(null, NOW), signature: '' };
  const update = nowMs => { const next = State.project(store.raw, nowMs); if (typeof State.stableThreads === 'function') next.threads = State.stableThreads(store.view.threads, next.threads);
    const sig = signature(next); if (sig !== store.signature) { store.signature = sig; store.view = next; return true; } return false; };
  const step = typeof State.storeStep === 'function' ? (event, nowMs) => State.storeStep(store, event, nowMs)
    : (event, nowMs) => {
      if (event.type === 'receipt') { store.raw = event.raw; store.lastReceipt = nowMs; return update(nowMs); }
      if (nowMs - store.lastReceipt > timeout(store.raw) && store.raw !== null) { store.raw = null; return update(nowMs); }
      return update(nowMs);
    };
  let replacements = 0, drops = 0, ticks = 0, receipts = 0;
  step({ type: 'receipt', raw: jsSnapshot(32, NOW, NOW / 1000) }, NOW);
  for (let t = 1; t <= duration; t++) {
    const now = NOW + t;
    if (t % heartbeat === 0) { receipts++; if (step({ type: 'receipt', raw: jsSnapshot(32, now, now / 1000) }, now)) replacements++; }
    if (t % 1000 === 0) { ticks++; const hadRaw = store.raw !== null; if (step({ type: 'tick' }, now)) replacements++; if (hadRaw && store.raw === null) drops++; }
  }
  return { simulated_seconds: duration / 1000, heartbeat_ms: heartbeat, ticks, receipts, view_replacements: replacements, snapshot_drops: drops,
           receipt_timeout_ms: timeout(jsSnapshot(32, NOW, NOW / 1000)), store_emulation: typeof State.storeStep === 'function' ? 'State.storeStep' : 'SnapshotStore 746ca31 logic' };
}

// ---------------------------------------------------------------- static code
function componentTypes(text) {
  const types = [];
  const clean = text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '').replace(/"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'/g, '""');
  for (const match of clean.matchAll(/(?:^|[\s:])([A-Z][A-Za-z0-9_]*)\s*\{/gm)) types.push(match[1]);
  return types;
}
function tooltipCount(dir, file, seen = []) {
  const text = readText(path.join(dir, file)); if (text === null || seen.includes(file)) return { tooltips: 0, surfaces: 0 };
  let tooltips = 0, surfaces = 0;
  for (const type of componentTypes(text)) {
    if (type === 'ToolTip') tooltips++;
    if (type === 'AntonSurface') surfaces++;
    if (fs.existsSync(path.join(dir, `${type}.qml`)) && `${type}.qml` !== file) { const inner = tooltipCount(dir, `${type}.qml`, [...seen, file]); tooltips += inner.tooltips; surfaces += inner.surfaces; }
  }
  return { tooltips, surfaces };
}
function staticMetrics() {
  const dir = path.join(sourceRoot, 'omarchy/herdr.observatory');
  const lines = file => { const t = readText(path.join(dir, file)); return t === null ? null : t.split('\n').length - (t.endsWith('\n') ? 1 : 0); };
  const qml = fs.existsSync(dir) ? fs.readdirSync(dir).filter(f => f.endsWith('.qml')) : [];
  const requiredUi = qml.reduce((s, f) => s + (readText(path.join(dir, f)).match(/required property var ui\b/g) || []).length, 0);
  const card = tooltipCount(dir, 'ThreadCard.qml');
  const ciText = readText(path.join(sourceRoot, '.github/workflows/checks.yml'));
  return { panel_qml_lines: lines('Panel.qml'), state_js_lines: lines('State.js'), popup_content_qml_lines: lines('PopupContent.qml'), total_qml_lines: qml.reduce((s, f) => s + lines(f), 0), qml_files: qml.length,
           required_property_var_ui: requiredUi, tooltips_per_thread_card: card.tooltips, anton_surfaces_per_thread_card: card.surfaces,
           ci_runs_qml_tests: ciText === null ? null : /run-qml\.sh/.test(ciText) };
}

// ---------------------------------------------------------------- allowance contract
// Added for generalise-anton-allowance-windows. These metrics are additive: the
// definitions above are unchanged. Wire bytes come from a separate short run with
// a fake read-only Codex app-server and a fake-SSH peer, so the fixed runtime
// windows above keep their original configuration and stay comparable.
const ALLOWANCE_ACCOUNTS = { local: 'synthetic-local-account', peer: 'synthetic-peer-account' };
const accountHash = id => execFileSync('sha256sum', { input: `observatory-codex-account-v1:${id}`, encoding: 'utf8' }).split(' ')[0];
async function allowanceProbe(base) {
  const f = fixture(base, 'allowances');
  const keys = Object.fromEntries(Object.entries(ALLOWANCE_ACCOUNTS).map(([k, id]) => [k, accountHash(id)]));
  const accounts = { [keys.local]: 'Personal', [keys.peer]: 'Work' };
  const peer = f.env.ANTON_TEST_PEER;
  const local = JSON.parse(fs.readFileSync(path.join(f.root, '.config.json'), 'utf8'));
  local.allowances = { accounts, sources: [{ target: 'fixture' }] };
  fs.writeFileSync(path.join(f.root, '.config.json'), JSON.stringify(local), { mode: 0o600 });
  fs.writeFileSync(path.join(peer, '.config.json'), JSON.stringify({ hosts: [{ id: 'remote', socket_path: f.remoteSocket }], allowances: { accounts } }), { mode: 0o600 });
  // The peer answers as a different synthetic account so two rows reach the wire.
  fs.writeFileSync(path.join(f.dir, 'bin/ssh'), '#!/bin/sh\nfor arg do last=$arg; done\ncase $last in\n *--allowances-probe*) ANTON_TEST_ACCOUNT=peer exec "$ANTON_TEST_PEER/anton-runtime" --root "$ANTON_TEST_PEER" --state "$ANTON_TEST_PEER_STATE" --allowances-probe;;\n *--probe*) exec "$ANTON_TEST_PEER/anton-runtime" --root "$ANTON_TEST_PEER" --state "$ANTON_TEST_PEER_STATE" --probe;;\n *) exit 99;;\nesac\n', { mode: 0o755 });
  const reset = Math.floor(Date.now() / 1000) + 302400;
  const reply = id => JSON.stringify({ id: 2, result: { accountId: id, rateLimits: { planType: 'pro', primary: { windowDurationMins: 300, usedPercent: 10, resetsAt: reset - 290000 }, secondary: { windowDurationMins: 10080, usedPercent: 40, resetsAt: reset } }, rateLimitResetCredits: { availableCount: 1, credits: [{ id: 'synthetic-pass', status: 'available', resetType: 'codexRateLimits', expiresAt: reset + 86400 }] } } });
  const usage = JSON.stringify({ id: 3, result: { summary: { lifetimeTokens: 1000000, peakDailyTokens: 50000 }, dailyUsageBuckets: [{ startDate: '2026-01-10', tokens: 40000 }, { startDate: '2026-01-11', tokens: 50000 }] } });
  fs.writeFileSync(path.join(f.dir, 'bin/codex'), `#!/bin/sh\ncase "\${ANTON_TEST_ACCOUNT:-local}" in peer) R='${reply(ALLOWANCE_ACCOUNTS.peer)}';; *) R='${reply(ALLOWANCE_ACCOUNTS.local)}';; esac\nwhile IFS= read -r line; do\n case "$line" in\n *'"method":"initialize"'*) printf '%s\\n' '{"id":1,"result":{}}';;\n *'"method":"account/rateLimits/read"'*) printf '%s\\n' "$R";;\n *'"method":"account/usage/read"'*) printf '%s\\n' '${usage}';;\n esac\ndone\n`, { mode: 0o755 });
  const servers = [await herdrServer(f.localSocket, herdrSnapshot('l')), await herdrServer(f.remoteSocket, herdrSnapshot('r'))];
  const run = launch(f); const start = performance.now();
  const present = row => row && typeof row === 'object' && (row.available === true || row.status === 'available');
  try {
    const frame = await run.until(fr => Array.isArray(fr.value?.allowances) && fr.value.allowances.length === 2 && fr.value.allowances.every(present), 30000);
    await run.close();
    if (!frame) return { rows: null, error: 'allowance rows never became available' };
    const rows = frame.value.allowances, bytes = rows.map(r => Buffer.byteLength(JSON.stringify(r)));
    const windows = rows.flatMap(r => Array.isArray(r.windows) ? r.windows : []);
    return { rows: rows.length, bytes_per_row: round(bytes.reduce((a, b) => a + b, 0) / rows.length, 1), allowances_bytes: Buffer.byteLength(JSON.stringify(rows)),
             row_keys: Object.keys(rows[0]).sort(), window_keys: windows.length ? Object.keys(windows[0]).sort() : null, windows_per_row: windows.length / rows.length,
             time_to_rows_ms: round(frame.at - start, 0) };
  } finally {
    if (!run.exit) { run.child.kill('SIGKILL'); await run.done; }
    for (const s of servers) { for (const socket of s.sockets) socket.destroy(); s.close(); }
  }
}
// Provider-neutral rows as specified by generalise-anton-allowance-windows. Older
// State.js versions receive the same rows; whatever they project is reported.
function neutralRow(id, overrides, window) {
  return { provider: 'codex', provider_label: 'Codex', account_id: id, label: id, status: 'available', status_text: null, plan: 'pro', sampled_at: NOW / 1000 - 30,
           reset_count: 1, reset_expires_at: null, windows: window === null ? [] : [{ kind: 'weekly', label: 'Weekly', used_percent: 40, resets_at: NOW / 1000 + 302400, duration_s: 604800, pacing: true, ...window }], ...overrides };
}
function allowanceContractMetrics(State) {
  if (!State || typeof State.project !== 'function') return null;
  const hosts = jsSnapshot(0, NOW, NOW / 1000 - 1).hosts;
  const view = rows => State.project({ at: NOW / 1000, interval: 5, heartbeat_seconds: 4, hosts, allowances: rows, fleet_discovery: { state: 'disabled' } }, NOW).allowances;
  const cases = {
    codex_weekly: neutralRow('one', {}, {}),
    synthetic_monthly: neutralRow('team', { provider: 'synthetic', provider_label: 'Synthetic' }, { kind: 'monthly', label: 'Monthly', used_percent: 25, resets_at: NOW / 1000 + 1296000, duration_s: 2592000 }),
    auth_needed: neutralRow('auth', { provider: 'synthetic', provider_label: 'Synthetic', status: 'auth_needed', status_text: 'Sign in required', sampled_at: null, reset_count: null }, null)
  };
  const result = {};
  for (const [name, row] of Object.entries(cases)) {
    const out = view([row])[0];
    result[name] = out ? { projected: true, remaining: out.remaining ?? null, time_remaining: out.timeRemaining === null || out.timeRemaining === undefined ? null : round(out.timeRemaining, 3), reset: out.reset ?? null, provider_label: out.providerLabel ?? null } : { projected: false };
  }
  const first = view([cases.codex_weekly])[0];
  result.view_fields = first ? { keys: Object.keys(first).length, leaves: leaves(first), names: Object.keys(first).sort() } : null;
  result.wire_row_bytes = Buffer.byteLength(JSON.stringify(cases.codex_weekly));
  return result;
}
// Presentation coupling to one provider: occurrences (not lines) in State.js and
// omarchy/herdr.observatory/*.qml. 'files naming a provider' is a proxy for the
// presentation files that would need to change to add a provider.
function providerCoupling() {
  const dir = path.join(sourceRoot, 'omarchy/herdr.observatory');
  const files = fs.existsSync(dir) ? fs.readdirSync(dir).filter(f => f === 'State.js' || f.endsWith('.qml')).sort() : [];
  const patterns = { weekly_field: /weekly_/g, codex_week_seconds: /604800/g, provider_equality: /provider\s*===?/g, codex_literal: /["']codex["']/gi };
  const totals = Object.fromEntries(Object.keys(patterns).map(k => [k, 0])), naming = [];
  for (const file of files) {
    const text = readText(path.join(dir, file)); let any = false;
    for (const [k, re] of Object.entries(patterns)) { const n = (text.match(re) || []).length; totals[k] += n; if (n) any = true; }
    if (any) naming.push(file);
  }
  return { scope: 'State.js and omarchy/herdr.observatory/*.qml, occurrences', ...totals, presentation_files_naming_a_provider: naming.length, files: naming };
}

// ---------------------------------------------------------------- architecture
// Added for restructure-anton-popover. Additive only: every definition above is
// unchanged. Static counts are occurrences in omarchy/herdr.observatory/*.qml.
// qmllint is Qt 6 qmllint (QMLLINT, /usr/lib/qt6/bin/qmllint, qmllint6 or a Qt 6
// qmllint on PATH) with <source-root>/tests/qml/anton as the only extra import
// path, so qs.Ui stays unresolved for Panel.qml in every measured tree.
function qt6Qmllint() {
  const candidates = [process.env.QMLLINT, '/usr/lib/qt6/bin/qmllint', which('qmllint6'), which('qmllint')].filter(Boolean);
  for (const candidate of candidates) {
    try { if (/qmllint 6\./.test(execFileSync(candidate, ['--version'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }))) return candidate; } catch {}
  }
  return null;
}
function qmllintMetrics(dir) {
  const tool = qt6Qmllint();
  const files = fs.existsSync(dir) ? fs.readdirSync(dir).filter(f => f.endsWith('.qml')).sort() : [];
  if (!tool || !files.length) return null;
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'anton-qmllint-')), out = path.join(scratch, 'lint.json');
  try {
    // qmllint exits non-zero when it warns; the JSON report is still written.
    try { execFileSync(tool, ['-I', path.join(sourceRoot, 'tests/qml/anton'), '--json', out, ...files.map(f => path.join(dir, f))], { stdio: 'ignore' }); } catch {}
    const report = JSON.parse(fs.readFileSync(out, 'utf8'));
    const byCategory = {}, byFile = {}; let total = 0;
    for (const file of report.files || []) {
      const warnings = file.warnings || []; byFile[path.basename(file.filename)] = warnings.length; total += warnings.length;
      for (const w of warnings) { const id = w.id || w.type || 'other'; byCategory[id] = (byCategory[id] || 0) + 1; }
    }
    return { tool: execFileSync(tool, ['--version'], { encoding: 'utf8' }).trim(), import_path: 'tests/qml/anton', total, by_category: byCategory, by_file: byFile };
  } catch { return null; } finally { fs.rmSync(scratch, { recursive: true, force: true }); }
}
// Clock-only view changes: one constant snapshot projected at 1 s steps for 60 s,
// counting steps whose replacement signature differs from the previous step.
function clockOnlyChanges(State) {
  if (!State || typeof State.project !== 'function') return null;
  const signature = typeof State.viewSignature === 'function' ? State.viewSignature : JSON.stringify;
  const raw = jsSnapshot(32, NOW, NOW / 1000);
  let previous = signature(State.project(raw, NOW)), changes = 0;
  for (let s = 1; s <= 60; s++) { const next = signature(State.project(raw, NOW + s * 1000)); if (next !== previous) changes++; previous = next; }
  return changes;
}
function architectureMetrics(State) {
  const dir = path.join(sourceRoot, 'omarchy/herdr.observatory');
  const qml = fs.existsSync(dir) ? fs.readdirSync(dir).filter(f => f.endsWith('.qml')) : [];
  const count = re => qml.reduce((s, f) => s + (readText(path.join(dir, f)).match(re) || []).length, 0);
  return { scope: 'omarchy/herdr.observatory/*.qml occurrences; clock-only changes use State.project and viewSignature (default JSON.stringify)',
           ui_member_references: count(/\bui\./g), preference_parse_calls: count(/\b(?:parseList|parseObject|JSON\.parse)\(/g),
           tooltip_declarations: qml.reduce((s, f) => s + componentTypes(readText(path.join(dir, f))).filter(t => t === 'ToolTip').length, 0),
           hardcoded_omarchy_state_paths: count(/\.local\/state\/omarchy/g), clock_only_view_changes_60s: clockOnlyChanges(State), qmllint: qmllintMetrics(dir) };
}

// Allowance readings (restructure-anton-popover, additive). The same three
// neutral rows as allowance_contract, projected at NOW. When State.allowanceReading
// exists (time-separated view) it is applied at NOW; otherwise the projected
// fields are read directly, which at baseline equals allowance_contract.
function allowanceReadingMetrics(State) {
  if (!State || typeof State.project !== 'function') return null;
  const hosts = jsSnapshot(0, NOW, NOW / 1000 - 1).hosts;
  const reading = typeof State.allowanceReading === 'function' ? account => State.allowanceReading(account, NOW) : account => account;
  const cases = {
    codex_weekly: neutralRow('one', {}, {}),
    synthetic_monthly: neutralRow('team', { provider: 'synthetic', provider_label: 'Synthetic' }, { kind: 'monthly', label: 'Monthly', used_percent: 25, resets_at: NOW / 1000 + 1296000, duration_s: 2592000 }),
    auth_needed: neutralRow('auth', { provider: 'synthetic', provider_label: 'Synthetic', status: 'auth_needed', status_text: 'Sign in required', sampled_at: null, reset_count: null }, null)
  };
  const result = { source: typeof State.allowanceReading === 'function' ? 'State.allowanceReading' : 'projected fields' };
  for (const [name, row] of Object.entries(cases)) {
    const projected = State.project({ at: NOW / 1000, interval: 5, heartbeat_seconds: 4, hosts, allowances: [row], fleet_discovery: { state: 'disabled' } }, NOW).allowances[0];
    const out = projected ? reading(projected) : null;
    result[name] = out ? { remaining: out.remaining ?? null, time_remaining: out.timeRemaining === null || out.timeRemaining === undefined ? null : round(out.timeRemaining, 3),
                           pace_difference: out.paceDifference === null || out.paceDifference === undefined ? null : round(out.paceDifference, 3), reset: out.reset ?? null, age: out.age ?? null } : null;
  }
  return result;
}

// ---------------------------------------------------------------- Claude transcripts
// Added for add-claude-thread-telemetry. Additive only: every definition above is
// unchanged except fixture()'s optional localBinary, which defaults to the
// measured binary, and the runtime windows above keep their Pi-only configuration. Each
// variant is one separate run with K Claude panes per host bound by Herdr id to
// synthetic transcripts under the fixture HOME's .claude/projects (local and fake-SSH
// peer share that HOME; ids differ per host). The panes carry no Herdr metadata
// tokens, so any technical.telemetry on them can only come from native replay.
const CLAUDE_METRICS = ['total_input', 'total_output', 'total_cache_read', 'total_cache_write', 'total_uncached_input', 'context', 'compactions', 'subagent_total'];
const claudeId = (variant, host, i) => `c1a0de00-${variant}${host}00-4000-8000-${(i + 1).toString(16).padStart(12, '0')}`;
// One synthetic main transcript: header, then turns of a human prompt, a two-line
// split response interleaved with an attachment, a tool result and an end_turn
// response closed by turn_duration. Turn 0 launches an async child that a
// task-notification completes in turn 1; turn 2 is followed by a compaction. Lines
// stay under 64 KiB; turns repeat until the transcript reaches target bytes.
// Records are 2 s apart and every ISO timestamp has the same width, so the
// record count does not depend on the start. A large transcript starts early
// enough that its last record is at least a minute old at any allowed size.
function claudeTranscript(id, target) {
  const now = Date.now(), usual = now - 6 * 3600 * 1000, first = claudeRecords(id, target, usual);
  const start = now - 60 * 1000 - 2000 * first.count;
  return start < usual ? claudeRecords(id, target, start).text : first.text;
}
function claudeRecords(id, target, start) {
  let t = start, n = 0;
  const at = () => new Date(t += 2000).toISOString();
  const uuid = () => `5e0c0000-0000-4000-8000-${(++n).toString(16).padStart(12, '0')}`;
  const base = { isSidechain: false, userType: 'external', cwd: '/synthetic/branch', sessionId: id, session_id: id, version: 'fixture' };
  const rec = (fields) => JSON.stringify({ ...base, uuid: uuid(), timestamp: at(), ...fields });
  const child = `synthetic-child-${id.slice(-4)}`, pad = 'x'.repeat(target ? 16000 : 64);
  const usage = i => { const u = { input_tokens: 6, cache_creation_input_tokens: 300, cache_read_input_tokens: 20000 + 100 * i, output_tokens: 120 + i }; return { ...u, iterations: [{ type: 'message', ...u }], service_tier: 'standard' }; };
  const assistant = (i, part, content, stop) => rec({ type: 'assistant', requestId: `req_synthetic_${i}_${part}`, message: { id: `msg_synthetic_${i}_${part}`, type: 'message', role: 'assistant', model: 'claude-synthetic-1', content, stop_reason: stop, stop_sequence: null, usage: usage(i) } });
  const lines = [JSON.stringify({ type: 'mode', mode: 'default', sessionId: id })];
  let bytes = lines[0].length + 1;
  const push = line => { lines.push(line); bytes += Buffer.byteLength(line) + 1; };
  for (let i = 0; i < 6 || bytes < target; i++) {
    push(i === 1 ? rec({ type: 'user', origin: { kind: 'task-notification' }, message: { role: 'user', content: `<task-notification>\n<task-id>${child}</task-id>\n<status>completed</status>\n<summary>Synthetic child finished</summary>\n</task-notification>` } })
      : rec({ type: 'user', origin: { kind: 'human' }, message: { role: 'user', content: `Synthetic prompt ${i}` } }));
    push(assistant(i, 'a', [{ type: 'text', text: pad }], null));
    push(rec({ type: 'attachment', attachment: { type: 'synthetic_reminder' } }));
    push(assistant(i, 'a', [{ type: 'tool_use', id: `toolu_synthetic_${i}`, name: i === 0 ? 'Agent' : 'Read', input: {} }], 'tool_use'));
    push(rec({ type: 'user', message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: `toolu_synthetic_${i}`, content: 'ok' }] }, toolUseResult: i === 0 ? { status: 'async_launched', agentId: child } : { stdout: 'ok' } }));
    push(assistant(i, 'b', [{ type: 'text', text: `Synthetic answer ${i}` }], 'end_turn'));
    push(rec({ type: 'system', subtype: 'turn_duration', durationMs: 8000, isMeta: false }));
    if (i === 2) { push(rec({ type: 'system', subtype: 'compact_boundary', content: 'Conversation compacted', compactMetadata: { trigger: 'auto', preTokens: 50000 } })); push(rec({ type: 'user', isCompactSummary: true, message: { role: 'user', content: 'Synthetic summary' } })); }
  }
  return { text: lines.join('\n') + '\n', count: n };
}
function claudeHerdr(prefix, ids) {
  return { protocol: 1, version: 'fixture', workspaces: [{ workspace_id: 'w1', label: 'Synthetic', worktree: { checkout_path: '/synthetic/branch' } }], agents: ids.map((id, i) => ({
    pane_id: `w1:${prefix}${i + 1}`, workspace_id: 'w1', cwd: '/synthetic/branch', agent: 'claude', agent_status: i % 2 ? 'idle' : 'working',
    agent_session: { agent: 'claude', source: 'herdr:claude', kind: 'id', value: id } })) };
}
function familyRss(pid) {
  const all = processes(); const ids = new Set([pid]); let changed = true;
  while (changed) { changed = false; for (const p of all.values()) if (ids.has(p.ppid) && !ids.has(p.pid)) { ids.add(p.pid); changed = true; } }
  return [...ids].filter(id => id !== pid).reduce((sum, id) => sum + (all.get(id)?.rss || 0), 0);
}
function claudeEmpty(error) {
  const nil = { local: null, peer: null };
  return { error, transcript_bytes: null, claude_agents: nil, claude_agents_with_native_telemetry: nil, usage_source_claude_transcript: nil,
           present: Object.fromEntries([...CLAUDE_METRICS, 'turn_timing'].map(k => [k, nil])), first_native_ms: nil, all_native_ms: nil, runtime_cpu_seconds: null, peak_rss_kib: null };
}
async function claudeProbe(base, variant, target, localBinary = binary) {
  const f = fixture(base, `claude-${variant}`, localBinary);
  // Never let a parent CLAUDE_CONFIG_DIR point the runtime at a real transcript root.
  delete f.env.CLAUDE_CONFIG_DIR;
  if (Object.keys(f.env).some(k => k === 'CLAUDE_CONFIG_DIR')) throw new Error('CLAUDE_CONFIG_DIR leaked into the Claude probe');
  const ids = { local: [], peer: [] }, sizes = [];
  for (const [h, host] of ['local', 'peer'].entries()) {
    const dir = path.join(f.env.HOME, '.claude/projects', `-synthetic-claude-${host}`);
    fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
    for (let i = 0; i < claudeCount; i++) {
      const id = claudeId(target ? 1 : 0, h, i), text = claudeTranscript(id, target);
      fs.writeFileSync(path.join(dir, `${id}.jsonl`), text, { mode: 0o600 }); sizes.push(Buffer.byteLength(text)); ids[host].push(id);
    }
  }
  const servers = [await herdrServer(f.localSocket, claudeHerdr('l', ids.local)), await herdrServer(f.remoteSocket, claudeHerdr('r', ids.peer))];
  const run = launch(f); const start = performance.now();
  let peakRss = 0;
  try {
    while (performance.now() - start < claudeSeconds * 1000) {
      if (run.exit) throw new Error(`Runtime exited early: ${run.errors}`);
      peakRss = Math.max(peakRss, familyRss(run.child.pid));
      await delay(50);
    }
    const result = await run.close();
    if (result.code !== 0) throw new Error(`Runtime failed: ${JSON.stringify(result)} ${run.errors}`);
    const claude = (frame, id) => (frame.value?.hosts?.find(h => h.id === id)?.agents || []).filter(a => a.harness === 'claude');
    const hosts = { local: 'local', peer: 'remote' };
    const seen = frame => Array.isArray(frame.value?.hosts) && Object.values(hosts).every(id => frame.value.hosts.find(h => h.id === id)?.online === true && claude(frame, id).length === claudeCount);
    const last = run.frames.filter(seen).at(-1);
    if (!last) return claudeEmpty('Claude agents never appeared on both synthetic hosts');
    const per = fn => Object.fromEntries(Object.entries(hosts).map(([k, id]) => [k, fn(claude(last, id))]));
    const native = a => a.technical?.telemetry !== null && typeof a.technical?.telemetry === 'object';
    const reach = all => Object.fromEntries(Object.entries(hosts).map(([k, id]) => { const hit = run.frames.find(fr => { const n = claude(fr, id).filter(native).length; return all ? n === claudeCount : n > 0; }); return [k, hit ? round(hit.at - start, 0) : null]; }));
    const [user, system] = (readText(f.timefile) || '').trim().split('\n').at(-1).split(' ').map(Number);
    return { error: null, transcript_bytes: { per_transcript: median(sizes), total: sizes.reduce((a, b) => a + b, 0) },
      claude_agents: per(a => a.length), claude_agents_with_native_telemetry: per(a => a.filter(native).length),
      usage_source_claude_transcript: per(a => a.filter(x => x.technical?.telemetry?.usage_source === 'claude-transcript').length),
      present: { ...Object.fromEntries(CLAUDE_METRICS.map(k => [k, per(a => a.filter(x => Number.isFinite(x.technical?.telemetry?.[k])).length)])),
                 turn_timing: per(a => a.filter(x => x.technical?.turn_timing !== null && typeof x.technical?.turn_timing === 'object').length) },
      first_native_ms: reach(false), all_native_ms: reach(true),
      runtime_cpu_seconds: Number.isFinite(user + system) ? round(user + system, 3) : null, peak_rss_kib: peakRss };
  } finally {
    if (!run.exit) { run.child.kill('SIGKILL'); await run.done; }
    for (const s of servers) { for (const socket of s.sockets) socket.destroy(); s.close(); }
  }
}
async function claudeMetrics(base) {
  const guard = async (variant, target, local) => { try { return await claudeProbe(base, variant, target, local); } catch (error) { return claudeEmpty(String(error?.message ?? error).slice(0, 400)); } };
  return { scope: `One ${claudeSeconds}s run per variant, ${claudeCount} Claude agents per host (local and fake-SSH peer). Native telemetry = technical.telemetry present on a Claude agent; the fixture panes carry no Herdr metadata, so only native replay can supply it. present counts agents with a numeric telemetry field (turn_timing: object present) in the last snapshot. CPU is user+sys of the runtime and reaped descendants (fake-SSH peer probes running locally); RSS is sampled every 50 ms over the runtime family. Remote hosts, Qt and GPU are not measured.`,
           agents_per_host: claudeCount, window_seconds: claudeSeconds, standard: await guard('standard', 0), large: claudeLargeMb > 0 ? { target_bytes: claudeLargeMb * 1e6, ...await guard('large', claudeLargeMb * 1e6) } : null,
           // Old local, new peer: the old local re-serialises cursor rows without the claude block, so the
           // peer replays every Claude row from the header on each probe. Hashes only; no binary paths.
           old_local: claudeOldLocal === null ? null : { local_binary_sha256: sha256(claudeOldLocal), peer_binary_sha256: sha256(binary),
             standard: await guard('old-local-standard', 0, claudeOldLocal),
             large: claudeLargeMb > 0 ? { target_bytes: claudeLargeMb * 1e6, ...await guard('old-local-large', claudeLargeMb * 1e6, claudeOldLocal) } : null } };
}
const sha256 = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');

// ---------------------------------------------------------------- report
const base = fs.mkdtempSync(path.join(os.tmpdir(), 'anton-measure-'));
fs.chmodSync(base, 0o700);
try {
  const State = loadState();
  const report = { scope: 'Synthetic fixtures only. Runtime CPU is user+sys of the runtime and reaped descendants (fake-SSH peer probes) over the fixed window; RSS is sampled every 50 ms over the runtime family. colors.toml opens include local and peer samples sharing the fixture HOME. Remote hosts, Qt and GPU are not measured.',
    source_root: sourceRoot, git_head: (() => { try { return execFileSync('git', ['-C', sourceRoot, 'rev-parse', '--short', 'HEAD'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim(); } catch { return null; } })(),
    runtime: null, refresh: null, projection: projectionMetrics(State), replacements: replacementMetrics(State), static: staticMetrics(), allowance_contract: allowanceContractMetrics(State), provider_coupling: providerCoupling(), allowance_wire: null, architecture: architectureMetrics(State), allowance_readings: allowanceReadingMetrics(State), claude: null };
  if (!flag('--skip-runtime')) {
    if (!fs.existsSync(binary)) throw new Error(`Runtime binary not found: ${binary}`);
    const counter = colorsCounter();
    const windows = []; for (let i = 0; i < repeat; i++) windows.push(await runtimeWindow(base, i, counter));
    if (counter.method === 'strace') for (let i = 0; i < repeat; i++) windows[i].colors_toml_opens = await straceWindow(base, i, counter);
    const pick = key => repeat === 1 ? windows[0][key] : median(windows.map(w => w[key]));
    report.runtime = { duration_seconds: seconds, hosts: 2, agents_per_host: count, repeats: repeat,
      snapshots: pick('snapshots'), mean_snapshot_bytes: round(pick('mean_snapshot_bytes'), 1), runtime_cpu_seconds: round(pick('runtime_cpu_seconds'), 3), peak_rss_kib: pick('peak_rss_kib'),
      colors_toml_opens: counter.method === 'unmeasured' ? 'unmeasured' : pick('colors_toml_opens'), colors_toml_method: counter.method,
      herdr_rpc_local: pick('herdr_rpc_local'), herdr_rpc_remote: pick('herdr_rpc_remote'), connected_ms: round(pick('connected_ms'), 0),
      heartbeat_seconds: windows[0].heartbeat_seconds, snapshot_keys: windows[0].snapshot_keys, host_keys: windows[0].host_keys,
      windows: repeat === 1 ? undefined : windows.map(w => ({ snapshots: w.snapshots, runtime_cpu_seconds: round(w.runtime_cpu_seconds, 3), peak_rss_kib: w.peak_rss_kib, colors_toml_opens: w.colors_toml_opens, mean_snapshot_bytes: round(w.mean_snapshot_bytes, 1) })) };
    if (!flag('--no-refresh-probe')) report.refresh = await refreshProbe(base);
    if (!flag('--no-allowance-probe')) report.allowance_wire = await allowanceProbe(base);
    if (!flag('--no-claude-probe')) report.claude = await claudeMetrics(base);
  }
  if (flag('--json')) console.log(JSON.stringify(report, null, 2));
  else {
    const r = report.runtime || {}, p = report.projection || {}, v = report.replacements || {}, s = report.static, q = report.refresh || {};
    const rows = [
      ['runtime snapshots', r.snapshots], ['runtime mean snapshot bytes', r.mean_snapshot_bytes], ['runtime CPU seconds (user+sys)', r.runtime_cpu_seconds], ['runtime peak RSS KiB', r.peak_rss_kib],
      ['colors.toml opens', r.colors_toml_opens], ['local Herdr samples', r.herdr_rpc_local], ['snapshot heartbeat_seconds', r.heartbeat_seconds],
      ['refresh latency ms', q.refresh_latency_ms], ['refresh answered by a post-request sample', q.refresh_answered_after_request], ['local samples in 20-refresh burst window', q.local_samples_during_burst_window], ['survives malformed stdin', q.survives_malformed_input],
      ['update median ms (32 threads)', p.per_update?.[32]?.median_ms], ['update median ms (128 threads)', p.per_update?.[128]?.median_ms],
      ['fields per thread (keys/leaves)', p.thread_fields && `${p.thread_fields.keys}/${p.thread_fields.leaves}`], ['fields per host (keys/leaves)', p.host_fields && `${p.host_fields.keys}/${p.host_fields.leaves}`],
      ['fields per allowance (keys/leaves)', p.allowance_fields && `${p.allowance_fields.keys}/${p.allowance_fields.leaves}`], ['view replacements in 60 s open', v.view_replacements], ['snapshot drops in 60 s', v.snapshot_drops], ['receipt timeout ms', v.receipt_timeout_ms],
      ['Panel.qml lines', s.panel_qml_lines], ['State.js lines', s.state_js_lines], ['PopupContent.qml lines', s.popup_content_qml_lines], ["'required property var ui'", s.required_property_var_ui],
      ['ToolTips per ThreadCard', s.tooltips_per_thread_card], ['AntonSurfaces per ThreadCard', s.anton_surfaces_per_thread_card], ['CI runs QML tests', s.ci_runs_qml_tests],
      ['allowance wire bytes per row', report.allowance_wire?.bytes_per_row], ['allowance wire keys per row', report.allowance_wire?.row_keys?.length],
      ['neutral-row allowance fields (keys/leaves)', report.allowance_contract?.view_fields && `${report.allowance_contract.view_fields.keys}/${report.allowance_contract.view_fields.leaves}`],
      ["'weekly_' / '604800' / 'provider ==' / 'codex' in presentation", report.provider_coupling && [report.provider_coupling.weekly_field, report.provider_coupling.codex_week_seconds, report.provider_coupling.provider_equality, report.provider_coupling.codex_literal].join(' / ')],
      ['presentation files naming a provider', report.provider_coupling?.presentation_files_naming_a_provider],
      ['ui. member references in QML', report.architecture?.ui_member_references], ['preference parse calls in QML', report.architecture?.preference_parse_calls],
      ['ToolTip declarations in QML', report.architecture?.tooltip_declarations], ['hard-coded ~/.local/state/omarchy paths in QML', report.architecture?.hardcoded_omarchy_state_paths],
      ['clock-only view changes in 60 s', report.architecture?.clock_only_view_changes_60s], ['qmllint warnings (Qt 6)', report.architecture?.qmllint?.total],
      ...[['standard', report.claude?.standard], ['large', report.claude?.large], ['old-local standard', report.claude?.old_local?.standard], ['old-local large', report.claude?.old_local?.large]].filter(([name, c]) => c || !name.startsWith('old-local')).flatMap(([name, c]) => [
        [`Claude ${name}: native agents local/peer`, c && `${c.claude_agents_with_native_telemetry.local}/${c.claude_agents_with_native_telemetry.peer} of ${c.claude_agents.local}/${c.claude_agents.peer}`],
        [`Claude ${name}: runtime CPU seconds`, c?.runtime_cpu_seconds]])];
    const width = Math.max(...rows.map(x => x[0].length));
    console.log(`Anton popover measurement (${report.git_head ?? 'unknown head'}, ${r.duration_seconds ?? 0}s x${r.repeats ?? 0}, ${count} agents/host)`);
    for (const [k, value] of rows) console.log(`${k.padEnd(width)}  ${value ?? 'null'}`);
    console.log(JSON.stringify(report, null, 2));
  }
} finally { fs.rmSync(base, { recursive: true, force: true }); }

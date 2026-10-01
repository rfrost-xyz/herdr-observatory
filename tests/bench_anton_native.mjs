#!/usr/bin/env node
// Reproducible, private-data-free native collector benchmark. Optional baseline
// package runs its own preserved adapter; the native run has no Python dependency.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import net from 'node:net';
import crypto from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i < 0 ? fallback : args[i + 1]; };
if (args.includes('--help')) {
  console.log('Usage: node tests/bench_anton_native.mjs [--binary PATH] [--seconds 60] [--agents 32] [--baseline-package DIR]\nTwo synthetic hosts, real fixture Unix RPC, fake SSH executing the native peer. No GPU, network or real accounts. Optional legacy package comparison is isolated from live configuration.');
  process.exit(0);
}
const binary = path.resolve(option('--binary', path.join(path.dirname(fileURLToPath(import.meta.url)), '../omarchy/anton-runtime/target/release/anton-runtime')));
const seconds = Number(option('--seconds', 60));
const count = Number(option('--agents', 32));
if (!(seconds >= 5 && seconds <= 300 && count >= 1 && count <= 128)) throw new Error('Invalid fixture duration or agent count');
const base = fs.mkdtempSync(path.join(os.tmpdir(), 'anton-native-bench-'));
fs.chmodSync(base, 0o700);
const sha = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const write = (file, value, mode = 0o600) => { fs.writeFileSync(file, value); fs.chmodSync(file, mode); };
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const binding = crypto.createHash('sha256').update('pi:path:/synthetic/session').digest('hex');
const raw = { protocol: 1, version: 'fixture', workspaces: [{ workspace_id: 'w1', label: 'Synthetic', worktree: { checkout_path: '/synthetic/branch' } }], agents: Array.from({ length: count }, (_, i) => ({
  pane_id: `w1:p${i + 1}`, workspace_id: 'w1', cwd: '/synthetic/branch', agent: 'pi', agent_status: i % 2 ? 'idle' : 'working',
  agent_session: { agent: 'pi', source: 'herdr:pi', kind: 'path', value: '/synthetic/session' },
  tokens: { obs_v: '2', obs_bind: binding, obs_seq: '1700000000000000', obs_event: 'output', obs_phase: 'output', obs_n0: '12345,678,12000,0', obs_n1: '4000,128000,1700000000000000,90000', obs_n2: '800,88000,0,2000', obs_n3: '2,3', obs_usage_source: 'pi-extension' }
})) };
const sockets = new Set();
const server = net.createServer(socket => {
  sockets.add(socket); socket.on('close', () => sockets.delete(socket)); socket.on('error', () => {});
  let buffer = '';
  socket.on('data', bytes => { buffer += bytes; if (!buffer.includes('\n')) return;
    const request = JSON.parse(buffer.split('\n')[0]);
    if (request.method !== 'session.snapshot') throw new Error('Unexpected fixture RPC');
    socket.end(JSON.stringify({ jsonrpc: '2.0', id: request.id, result: { snapshot: raw } }) + '\n');
  });
});
await new Promise((resolve, reject) => { server.once('error', reject); server.listen(path.join(base, 'herdr.sock'), resolve); });
const ticks = Number(execFileSync('getconf', ['CLK_TCK'], { encoding: 'utf8' }).trim());
function processes() {
  const rows = new Map();
  for (const id of fs.readdirSync('/proc').filter(v => /^\d+$/.test(v))) {
    try {
      const stat = fs.readFileSync(`/proc/${id}/stat`, 'utf8'); const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
      rows.set(Number(id), { pid: Number(id), ppid: Number(fields[1]), cpu: (Number(fields[11]) + Number(fields[12])) / ticks, rss: Number(fields[21]) * 4096 / 1024 });
    } catch {}
  }
  return rows;
}
async function run(label, baseline) {
  const dir = path.join(base, label); const root = path.join(dir, 'plugin'); const peer = path.join(dir, 'herdr.observatory-peer');
  for (const p of [dir, root, peer, path.join(dir, 'bin'), path.join(dir, 'home'), path.join(dir, 'state')]) fs.mkdirSync(p, { mode: 0o700 });
  if (baseline) for (const file of ['anton-runtime', 'native-adapter.py', 'runtime.zip', 'runtime.py']) fs.copyFileSync(path.join(baseline, file), path.join(root, file));
  else fs.copyFileSync(binary, path.join(root, 'anton-runtime'));
  fs.chmodSync(path.join(root, 'anton-runtime'), 0o755);
  fs.copyFileSync(binary, path.join(peer, 'anton-runtime')); fs.chmodSync(path.join(peer, 'anton-runtime'), 0o755);
  for (const p of [root, peer]) write(path.join(p, '.herdr-observatory-install'), 'herdr.observatory\n');
  const socket_path = path.join(base, 'herdr.sock');
  write(path.join(root, '.config.json'), JSON.stringify({ interval: 2, hosts: [{ id: 'local', socket_path }, { id: 'remote', transport: 'ssh', target: 'fixture', socket_path }] }));
  write(path.join(peer, '.config.json'), JSON.stringify({ hosts: [{ id: 'remote', socket_path }] }));
  write(path.join(dir, 'bin/ssh'), '#!/bin/sh\nfor arg do last=$arg; done\ncase "$last" in\n "python3 -") exec /usr/bin/python3 -;;\n *--probe*) exec "$ANTON_TEST_PEER/anton-runtime" --root "$ANTON_TEST_PEER" --probe;;\n *) exit 99;;\nesac\n', 0o755);
  const env = { ...process.env, HOME: path.join(dir, 'home'), CODEX_HOME: path.join(dir, 'home/.codex'), PATH: `${path.join(dir, 'bin')}:/usr/bin:/bin`, XDG_STATE_HOME: path.join(dir, 'state'), ANTON_TEST_PEER: peer };
  delete env.OBSERVATORY_SSH_CONFIG;
  const timefile = path.join(dir, 'time.txt');
  env.ANTON_TEST_TIMING = timefile;
  const child = spawn('/bin/bash', ['-c', 'TIMEFORMAT="%U %S"; { time "$@"; } 2>"$ANTON_TEST_TIMING"', 'anton-benchmark', path.join(root, 'anton-runtime'), '--root', root, '--state', path.join(dir, 'state')], { env, stdio: ['pipe', 'pipe', 'pipe'] });
  const start = performance.now(); let first = null, connected = null, frames = 0, buffer = '', errors = '', exit = false;
  let latest; child.stdout.on('data', data => { buffer += data; let i; while ((i = buffer.indexOf('\n')) >= 0) { const frame = JSON.parse(buffer.slice(0, i)); buffer = buffer.slice(i + 1); frames++; first ??= performance.now() - start; latest = frame; if (frame?.hosts?.every(h => h.online)) connected ??= performance.now() - start; } });
  child.stderr.on('data', data => errors += data.toString().slice(0, 4096));
  const done = new Promise((resolve, reject) => { child.once('error', reject); child.once('exit', (code, signal) => { exit = true; resolve({ code, signal }); }); });
  const observed = new Map(); let peakRss = 0, peakProcesses = 0, rssSum = 0, samples = 0, cpuAtTwo = 0, afterTwo = false;
  try {
    while (performance.now() - start < seconds * 1000) {
      if (exit) throw new Error(`${label} exited early: ${errors}`);
      const all = processes(); const ids = new Set([child.pid]); let changed = true;
      while (changed) { changed = false; for (const p of all.values()) if (ids.has(p.ppid) && !ids.has(p.pid)) { ids.add(p.pid); changed = true; } }
      const family = [...ids].filter(id => id !== child.pid).map(id => all.get(id)).filter(Boolean);
      for (const p of family) observed.set(p.pid, Math.max(p.cpu, observed.get(p.pid) || 0));
      const rss = family.reduce((sum, p) => sum + p.rss, 0); peakRss = Math.max(peakRss, rss); peakProcesses = Math.max(peakProcesses, family.length); rssSum += rss; samples++;
      if (!afterTwo && performance.now() - start >= 2000) { afterTwo = true; cpuAtTwo = [...observed.values()].reduce((a, b) => a + b, 0); }
      await delay(50);
    }
    const close = performance.now(); child.stdin.end();
    const result = await Promise.race([done, delay(3000).then(() => { throw new Error(`${label} EOF timeout`); })]);
    if (result.code !== 0 || !connected) throw new Error(`${label} failed: ${JSON.stringify(result)} ${errors}`);
    for (const host of latest.hosts) {
      if (host.agents.length !== count || host.agents[0].technical.telemetry.input !== 12345) throw new Error(`${label} fixture telemetry mismatch`);
    }
    const [user, system] = fs.readFileSync(timefile, 'utf8').trim().split('\n').at(-1).split(' ').map(Number);
    if (!Number.isFinite(user + system)) throw new Error('Invalid process-family timing');
    return { label, binary_sha256: sha(path.join(root, 'anton-runtime')), ...(baseline ? { adapter_sha256: sha(path.join(root, 'native-adapter.py')), runtime_zip_sha256: sha(path.join(root, 'runtime.zip')) } : {}), duration_seconds: seconds, hosts: 2, agents_per_host: count, total_family_cpu_seconds: user + system, observed_steady_cpu_seconds: [...observed.values()].reduce((a, b) => a + b, 0) - cpuAtTwo, steady_duration_seconds: seconds - 2, peak_family_rss_kib: peakRss, mean_family_rss_kib: rssSum / samples, peak_processes: peakProcesses, first_snapshot_ms: first, connected_ms: connected, eof_ms: performance.now() - close, snapshots: frames };
  } finally {
    if (!exit) { child.stdin.end(); await Promise.race([done, delay(3000)]); if (!exit) { for (const pid of observed.keys()) { try { process.kill(pid, 'SIGKILL'); } catch {} } child.kill('SIGKILL'); await done; } }
  }
}
try {
  const baseline = option('--baseline-package', null); const results = [];
  if (baseline) results.push(await run('hybrid', path.resolve(baseline)));
  results.push(await run('native', null));
  console.log(JSON.stringify({ scope: 'Synthetic local plus fake SSH peer, no allowances/music/publication; system time includes reaped descendants. RSS sampled at roughly 50 ms; observed steady CPU can miss short-lived descendants. No private user data.', results }, null, 2));
} finally { for (const socket of sockets) socket.destroy(); server.close(); fs.rmSync(base, { recursive: true, force: true }); }

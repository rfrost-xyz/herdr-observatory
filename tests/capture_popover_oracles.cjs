#!/usr/bin/env node
// Captures the pre-restructure presentation oracles used by
// restructure-anton-popover. Run it only against the unchanged State.js of
// 53f2407 (origin/main before the change):
//
//   git show 53f2407:omarchy/herdr.observatory/State.js > <scratch>/State.js
//   node tests/capture_popover_oracles.cjs <scratch>/State.js tests/fixtures
//
// Synthetic data only. Each time case projects one raw snapshot at instants on
// both sides of every time threshold (host maxAge, 120 s telemetry staleness,
// turn freshness and one-second skew, 600 s allowance freshness, reset passing,
// pass expiry, window-length coverage and the age and reset label boundaries).
// The diagnostics oracle evaluates Panel.qml's IpcHandler.diagnostics body.
'use strict';
const fs = require('fs');
const path = require('path');

const [stateJs, outDir] = process.argv.slice(2);
if (!stateJs || !outDir) throw new Error('Usage: capture_popover_oracles.cjs STATE_JS OUT_DIR');
const State = require(path.resolve(stateJs));

const NOW = 1800000000000;
const S = NOW / 1000;
const US = ms => ms * 1000;

function telemetry(stampMs, extra) {
  return Object.assign({ seq: US(stampMs), usage_seq: US(stampMs), context: 42000, window: 128000, total_input: 2100000, total_output: 34000,
    total_cache_read: 1950000, total_uncached_input: 150000, total_cache_write: 0, compactions: 2,
    subagent_total: 4, subagent_done: 2, subagent_running: 1, subagent_interrupted: 0, subagent_failed: 1, subagent_unknown: 0, subagent_status_seq: US(stampMs),
    subagent_starts: 3, subagent_stops: 2, subagent_seq: US(stampMs) }, extra || {});
}
function timing(observedS, extra) {
  return Object.assign({ active: true, started_at_s: observedS - 754, observed_at_s: observedS, last_duration_s: 420, last_outcome: 'completed',
    total_finished_duration_s: 2100, complete: true, freshness_seconds: 12 }, extra || {});
}
function agent(id, status, technical) {
  return { id, status, project: 'Project ' + id, title: 'Task ' + id, branch: 'feature/' + id, checkout: 'branch-' + id, harness: 'codex', technical };
}
function host(id, sampledAt, agents, extra) {
  return Object.assign({ id, label: 'Host ' + id, online: true, connection_state: 'connected', sampled_at: sampledAt,
    navigation: { profile_id: id, route_key: 'a'.repeat(64) }, agents }, extra || {});
}
function allowance(id, sampledAt, window, extra) {
  return Object.assign({ provider: 'codex', provider_label: 'Codex', account_id: id, label: id, status: 'available', status_text: null, plan: 'pro',
    sampled_at: sampledAt, reset_count: 1, reset_expires_at: null,
    windows: [Object.assign({ kind: 'weekly', label: 'Weekly', used_percent: 40, resets_at: S + 302400, duration_s: 604800, pacing: true }, window || {})] }, extra || {});
}
function snapshot(hosts, allowances, extra) {
  return Object.assign({ at: S, interval: 5, heartbeat_seconds: 4, hosts, allowances, fleet_discovery: { state: 'disabled' } }, extra || {});
}
const at = offsets => offsets.map(s => NOW + Math.round(s * 1000));

const cases = [
  { name: 'host-max-age-default-interval', raw: snapshot([host('alpha', S, [agent('a', 'working', { telemetry: telemetry(NOW), turn_timing: timing(S) })])], []),
    at: at([0, 1, 24, 24.5, 24.999, 25, 26, 59, 60, 61]) },
  { name: 'host-max-age-interval-60', raw: snapshot([host('alpha', S, [agent('a', 'idle', {})])], [], { interval: 60 }), at: at([0, 79, 79.999, 80, 81]) },
  { name: 'host-max-age-invalid-interval', raw: snapshot([host('alpha', S, [agent('a', 'idle', {})])], [], { interval: 1 }), at: at([0, 24.999, 25]) },
  { name: 'host-sampled-in-future', raw: snapshot([host('alpha', S + 2, [agent('a', 'idle', {})])], []), at: at([0, 1, 1.999, 2, 3]) },
  // Host interval 60 keeps the host reporting for 80 s; telemetry is 100 s old at 0.
  { name: 'telemetry-staleness-120s', raw: snapshot([host('alpha', S, [agent('a', 'working', { telemetry: telemetry(NOW - 100000) })])], [], { interval: 60 }),
    at: at([0, 1, 19, 19.999, 20, 20.5, 21, 40, 79]) },
  { name: 'telemetry-age-label-hours', raw: snapshot([host('alpha', S, [agent('a', 'idle', { telemetry: telemetry(NOW - 3590000) })])], [], { interval: 60 }),
    at: at([0, 9, 9.999, 10, 11]) },
  { name: 'telemetry-age-label-seconds', raw: snapshot([host('alpha', S, [agent('a', 'idle', { telemetry: telemetry(NOW - 50000) })])], [], { interval: 60 }),
    at: at([0, 9, 9.999, 10, 11]) },
  { name: 'telemetry-stamped-in-future', raw: snapshot([host('alpha', S, [agent('a', 'working', { telemetry: telemetry(NOW + 2000) })])], []),
    at: at([0, 1, 1.999, 2, 3]) },
  { name: 'turn-freshness-and-elapsed', raw: snapshot([host('alpha', S, [
      agent('a', 'working', { turn_timing: timing(S) }),
      agent('b', 'idle', { turn_timing: timing(S, { active: false, last_outcome: 'aborted' }) }),
      agent('c', 'working', { turn_timing: timing(S, { freshness_seconds: 3, complete: false }) })])], [], { interval: 60 }),
    at: at([0, 0.5, 1, 2.999, 3, 3.5, 4, 11, 11.5, 11.999, 12, 12.5, 13, 20]) },
  { name: 'turn-observed-in-future', raw: snapshot([host('alpha', S, [agent('a', 'working', { turn_timing: timing(S + 3) })])], [], { interval: 60 }),
    at: at([0, 1, 1.999, 2, 2.5, 3, 4]) },
  { name: 'allowance-freshness-600s', raw: snapshot([], [allowance('one', S), allowance('two', S + 0.5), allowance('three', S + 2)]),
    at: at([0, 1, 1.999, 2, 599, 599.999, 600, 600.5, 601]) },
  { name: 'allowance-reset-passing', raw: snapshot([], [allowance('one', S, { resets_at: S + 3 }), allowance('two', S, { resets_at: S + 3600 }), allowance('three', S, { resets_at: S + 90000 })]),
    at: at([0, 1, 2.999, 3, 4, 599]) },
  { name: 'allowance-reset-label-boundaries', raw: snapshot([], [allowance('one', S, { resets_at: S + 3605 }), allowance('two', S, { resets_at: S + 86405 })]),
    at: at([0, 4, 4.999, 5, 6]) },
  { name: 'allowance-pass-expiry', raw: snapshot([], [allowance('one', S, {}, { reset_count: 2, reset_expires_at: S + 5 })]), at: at([0, 4.999, 5, 6]) },
  { name: 'allowance-window-coverage', raw: snapshot([], [allowance('one', S, { resets_at: S + 700, duration_s: 600 })]), at: at([0, 99, 99.999, 100, 101, 300]) },
  { name: 'allowance-monthly-window', raw: snapshot([], [allowance('team', S, { kind: 'monthly', label: 'Monthly', used_percent: 25, resets_at: S + 1296000, duration_s: 2592000 }, { provider: 'synthetic', provider_label: 'Synthetic' })]),
    at: at([0, 1, 300, 599]) },
  { name: 'mixed-fleet', raw: snapshot([
      host('alpha', S, [agent('a', 'working', { telemetry: telemetry(NOW - 100000), turn_timing: timing(S - 5) }), agent('b', 'done', { session_generation: 2, state_change_seq: 9, telemetry: telemetry(NOW) })]),
      host('beta', S - 20, [agent('c', 'blocked', { telemetry: telemetry(NOW - 30000), turn_timing: timing(S - 1, { active: false }) })]),
      host('gamma', S, [], { online: false, connection_state: 'unreachable' }),
      host('delta', S, [], { connection_state: 'setup_needed' })],
      [allowance('one', S - 30), allowance('two', S - 590, { used_percent: 100 }), allowance('auth', null, {}, { status: 'auth_needed', status_text: 'Sign in required', windows: [] })],
      { fleet_discovery: { state: 'discovering' } }),
    at: at([0, 1, 5, 6, 7, 10, 20, 21, 30, 60]) }
];

const timeOracle = { source: 'State.js at 53f2407', generator: 'tests/capture_popover_oracles.cjs', nowBaseMs: NOW,
  cases: cases.map(c => ({ name: c.name, raw: c.raw, projections: c.at.map(nowMs => ({ nowMs, view: State.project(c.raw, nowMs) })) })) };

// Panel.qml IpcHandler.diagnostics body at 53f2407, applied to a projected view.
function diagnostics(overview) {
  return JSON.stringify({
    connected: overview.connected,
    hosts: overview.hosts.map(h => ({ id: h.id, connection: h.connectionState, reporting: h.reporting })),
    threads: overview.threads.length,
    usageReported: overview.threads.filter(t => t.usage && t.usage.inputTokens !== null).length,
    timingReported: overview.threads.filter(t => t.timing && t.timing.elapsed !== null).length,
    timingCurrent: overview.threads.filter(t => t.timing && t.timing.active && !t.timing.stale && t.timing.elapsed !== null).length,
    timingTotals: overview.threads.filter(t => t.timing && t.timing.total !== null).length,
    timingStale: overview.threads.filter(t => t.timing && t.timing.stale).length,
    allowances: overview.allowances.map(a => ({ label: a.label, available: a.remaining !== null }))
  });
}
const diagnosticsOracle = { source: 'Panel.qml IpcHandler.diagnostics at 53f2407', generator: 'tests/capture_popover_oracles.cjs',
  cases: [{ name: 'disconnected', raw: null, nowMs: NOW }].concat(cases.filter(c => c.name === 'mixed-fleet' || c.name === 'turn-freshness-and-elapsed')
    .flatMap(c => c.at.slice(0, 6).map(nowMs => ({ name: c.name, raw: c.raw, nowMs }))))
    .map(c => ({ name: c.name, raw: c.raw, nowMs: c.nowMs, diagnostics: diagnostics(State.project(c.raw, c.nowMs)) })) };

fs.writeFileSync(path.join(outDir, 'popover-time-oracle.json'), JSON.stringify(timeOracle) + '\n');
fs.writeFileSync(path.join(outDir, 'popover-diagnostics-oracle.json'), JSON.stringify(diagnosticsOracle, null, 1) + '\n');
console.log(`time cases ${timeOracle.cases.length}, projections ${timeOracle.cases.reduce((s, c) => s + c.projections.length, 0)}, diagnostics ${diagnosticsOracle.cases.length}`);

#!/usr/bin/env node
// Stand-in for anton-runtime in the installed-shell harness. Every invocation
// appends its arguments to ANTON_FAKE_LOG, so the harness can prove that no
// --open-thread call happened. Without arguments it acts as the collector: it
// prints the synthetic mixed-fleet snapshot from the time oracle, shifted to
// the current time, every 2 s and on each stdin line, and exits on stdin EOF
// (owner close). A synthetic Claude thread with transcript telemetry (no
// context window) is added to the first host. Any other invocation exits at
// once and launches nothing.
import fs from 'node:fs';

fs.appendFileSync(process.env.ANTON_FAKE_LOG, JSON.stringify(process.argv.slice(2)) + '\n');
if (process.argv.length > 2) process.exit(0);

const oracle = JSON.parse(fs.readFileSync(process.env.ANTON_FIXTURE, 'utf8'));
const base = structuredClone(oracle.cases.find((entry) => entry.name === 'mixed-fleet').raw);
// Microsecond sequences are not shifted, so the Claude stamps are set per emit.
const stamped = ['seq', 'usage_seq', 'subagent_status_seq', 'subagent_seq'];
base.hosts[0].agents.push({
  id: 'claude-a', status: 'working', project: 'Project claude', title: 'Task claude', branch: 'feature/claude',
  checkout: 'branch-claude', harness: 'claude',
  technical: {
    telemetry: {
      event: 'session', phase: 'ready', usage_source: 'claude-transcript',
      model: 'claude-synthetic-1', context: 48000, last_input: 2000, last_output: 300,
      total_input: 90000, total_output: 4000, total_cache_read: 70000, total_cache_write: 12000, total_uncached_input: 8000,
      subagent_total: 2, subagent_done: 1, subagent_running: 1, subagent_interrupted: 0, subagent_failed: 0, subagent_unknown: 0,
      subagent_starts: 2, subagent_stops: 1
    },
    turn_timing: {
      active: true, started_at_s: base.at - 90, observed_at_s: base.at - 5, last_duration_s: 60, last_outcome: 'completed',
      total_finished_duration_s: 300, complete: true, freshness_seconds: 12
    }
  }
});

// Shift Unix-second and millisecond instants; sequence numbers stay as they are.
function shift(value, seconds) {
  if (Array.isArray(value)) return value.map((item) => shift(item, seconds));
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, shift(item, seconds)]));
  }
  if (typeof value === 'number' && value > 1.7e9 && value < 1.9e9) return value + seconds;
  if (typeof value === 'number' && value > 1.7e12 && value < 1.9e12) return value + seconds * 1000;
  return value;
}
function emit() {
  const now = Math.floor(Date.now() / 1000);
  const snapshot = shift(base, now - base.at);
  const telemetry = snapshot.hosts[0].agents.find((agent) => agent.harness === 'claude').technical.telemetry;
  for (const key of stamped) telemetry[key] = (now - 30) * 1e6;
  process.stdout.write(JSON.stringify(snapshot) + '\n');
}

emit();
setInterval(emit, 2000);
process.stdin.on('data', emit);
process.stdin.on('end', () => process.exit(0));

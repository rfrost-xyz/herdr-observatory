#!/usr/bin/env node
// Stand-in for anton-runtime in the installed-shell harness. Every invocation
// appends its arguments to ANTON_FAKE_LOG, so the harness can prove that no
// --open-thread call happened. Without arguments it acts as the collector: it
// prints the synthetic mixed-fleet snapshot from the time oracle, shifted to
// the current time, every 2 s and on each stdin line, and exits on stdin EOF
// (owner close). Any other invocation exits at once and launches nothing.
import fs from 'node:fs';

fs.appendFileSync(process.env.ANTON_FAKE_LOG, JSON.stringify(process.argv.slice(2)) + '\n');
if (process.argv.length > 2) process.exit(0);

const oracle = JSON.parse(fs.readFileSync(process.env.ANTON_FIXTURE, 'utf8'));
const base = oracle.cases.find((entry) => entry.name === 'mixed-fleet').raw;

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
  process.stdout.write(JSON.stringify(shift(base, Math.floor(Date.now() / 1000) - base.at)) + '\n');
}

emit();
setInterval(emit, 2000);
process.stdin.on('data', emit);
process.stdin.on('end', () => process.exit(0));

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(__dirname, '../omarchy/herdr.observatory/State.js'), 'utf8');
const sandbox = { module: { exports: {} } };
vm.runInNewContext(source, sandbox, { filename: 'State.js' });
const { project } = sandbox.module.exports;
const now = 1_800_000_000_000;

function host(overrides = {}) {
  return {
    id: 'iapetus', label: 'iapetus', online: true, sampled_at: now / 1000 - 5,
    agents: [{ id: 'iapetus:4', project: 'Example', title: 'Synthetic task', status: 'working', harness: 'codex' }],
    metrics: { gpu: { percent: 76 } }, ...overrides
  };
}

test('unavailable hosts do not become zero-thread evidence', () => {
  const view = project({ interval: 5, hosts: [host(), host({ id: 'ws-255', label: 'ws-255', online: false, agents: [] })], allowances: [] }, now);
  assert.equal(view.working, 1);
  assert.equal(view.partial, true);
  assert.equal(view.hosts[1].reporting, false);
  assert.equal(view.note, '1 source unavailable');
});

test('expired sample removes its threads and GPU reading', () => {
  const view = project({ interval: 5, hosts: [host({ sampled_at: now / 1000 - 30 })], allowances: [] }, now);
  assert.equal(view.threads.length, 0);
  assert.equal(view.gpu, null);
  assert.equal(view.partial, true);
});

test('all unavailable hosts leave the active count unknown', () => {
  const view = project({ interval: 5, hosts: [host({ online: false, agents: [] })], allowances: [] }, now);
  assert.equal(view.connected, true);
  assert.equal(view.working, null);
  assert.equal(view.partial, true);
  assert.equal(view.note, 'No sources reporting');
});

test('mapped weekly allowance preserves zero and rejects expired reset', () => {
  const view = project({ interval: 5, hosts: [host()], allowances: [
    { label: 'Personal', available: true, weekly_remaining: 0, weekly_resets_at: now / 1000 + 100, sampled_at: now / 1000 - 12 },
    { label: 'Work', available: true, weekly_remaining: 45, weekly_resets_at: now / 1000 - 1, sampled_at: now / 1000 - 12 }
  ] }, now);
  assert.equal(view.allowances[0].remaining, 0);
  assert.equal(view.allowances[1].remaining, null);
});

test('GPU is device evidence and inference remains unavailable', () => {
  const view = project({ interval: 5, hosts: [host(), host({ id: 'ws-255', label: 'ws-255' })], allowances: [] }, now);
  assert.equal(view.gpu.percent, 76);
  assert.equal(view.gpu.host, 'ws-255');
  assert.equal(view.inference, 'Inference use unavailable');
});

test('laptop GPU is not presented as ws-255 inference hardware', () => {
  const view = project({ interval: 5, hosts: [host()], allowances: [] }, now);
  assert.equal(view.gpu, null);
});

test('disconnection discards the last snapshot', () => {
  const view = project(null, now);
  assert.equal(view.connected, false);
  assert.equal(view.working, null);
  assert.equal(view.threads.length, 0);
});

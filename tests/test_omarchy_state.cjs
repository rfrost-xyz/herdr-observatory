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

test('host map identity stays correct when display labels match', () => {
  const view = project({ interval: 5, hosts: [
    host({ id: 'iapetus', label: 'shared' }),
    host({ id: 'ws-255', label: 'shared', agents: [] })
  ], allowances: [] }, now);
  assert.equal(view.threads[0].hostId, 'iapetus');
  assert.equal(view.threads.filter(thread => thread.hostId === view.hosts[0].id).length, 1);
  assert.equal(view.threads.filter(thread => thread.hostId === view.hosts[1].id).length, 0);
});

test('disconnection discards the last snapshot', () => {
  const view = project(null, now);
  assert.equal(view.connected, false);
  assert.equal(view.working, null);
  assert.equal(view.threads.length, 0);
});

test('burn pace compares balance with time remaining and does not assume a reset', () => {
  const allowance = overrides => ({ label: 'Personal', available: true, weekly_remaining: 70,
    weekly_resets_at: now / 1000 + 302400, sampled_at: now / 1000 - 5, ...overrides });
  const view = row => project({ hosts: [host()], allowances: [row] }, now).allowances[0];
  assert.equal(view(allowance()).timeRemaining, 50);
  assert.equal(view(allowance()).paceDifference, 20);
  assert.equal(view(allowance()).pace, 'reserve');
  assert.equal(view(allowance({ weekly_remaining: 30 })).pace, 'deficit');
  assert.equal(view(allowance({ weekly_remaining: 53 })).pace, 'reserve');
  assert.equal(view(allowance({ weekly_remaining: 47 })).pace, 'deficit');
  assert.equal(view(allowance({ weekly_remaining: 53.1 })).pace, 'reserve');
  assert.equal(view(allowance({ weekly_remaining: 46.9 })).pace, 'deficit');
  assert.equal(view(allowance()).reset, '3d 12h');
  assert.equal(view(allowance({ weekly_remaining: 0 })).paceDifference, -50);
  assert.equal(view(allowance({ weekly_resets_at: now / 1000 + 604801 })).timeRemaining, null);
  const expired = view(allowance({ weekly_resets_at: now / 1000 }));
  assert.equal(expired.remaining, null);
  assert.equal(expired.timeRemaining, null);
  assert.equal(expired.paceDifference, null);
  assert.equal(expired.pace, 'unknown');
});

test('activity charts preserve reported dates and reject invalid or stale observations', () => {
  const daily = [{ date: '2026-09-20', tokens: 20 }, { date: '2026-09-22', tokens: 0 }];
  const allowance = { label: 'Personal', available: true, weekly_remaining: 70,
    weekly_resets_at: now / 1000 + 1000, sampled_at: now / 1000 - 5, daily_usage: daily };
  const view = overrides => project({ hosts: [host()], allowances: [{ ...allowance, ...overrides }] }, now).allowances[0];
  const activity = view({}).activity;
  assert.equal(activity.count, 2);
  assert.equal(activity.daily[0].ratio, 1);
  assert.equal(activity.daily[1].ratio, 0);
  assert.equal(activity.daily[1].date, '2026-09-22');
  assert.equal(view({ daily_usage: [...daily, daily[0]] }).activity, null);
  assert.equal(view({ daily_usage: [{ date: '2026-02-30', tokens: 1 }] }).activity, null);
  assert.equal(view({ daily_usage: [{ date: '2099-01-01', tokens: 1 }] }).activity, null);
  assert.equal(view({ daily_usage: [{ date: '2026-09-20', tokens: -1 }] }).activity, null);
  assert.equal(view({ sampled_at: now / 1000 - 601 }).activity, null);
});

test('fleet gauges preserve measured zeroes and suppress invalid or expired metrics', () => {
  const metrics = { cpu_percent: 0, memory: { used: 1, total: 4 }, gpu: { percent: 10, used: 8, total: 10 } };
  const view = overrides => project({ hosts: [host({ metrics, ...overrides })], allowances: [] }, now).hosts[0];
  assert.equal(view({}).cpu, 0);
  assert.equal(view({}).memory, 25);
  assert.equal(view({}).vram, 80);
  assert.equal(view({ sampled_at: now / 1000 - 30 }).gpu, null);
  assert.equal(view({ online: false }).cpu, null);
  assert.equal(view({ metrics: { cpu_percent: 101, memory: { used: 2, total: 1 }, gpu: { percent: -1 } } }).memory, null);
  assert.equal(view({ metrics: { cpu_percent: 101 } }).cpu, null);
});

test('thread instruments use session counters and the original usage timestamp', () => {
  const telemetry = { seq: now * 1000, usage_seq: (now - 10000) * 1000,
    context: 60, window: 100, context_percent: 55,
    total_input: 1000, total_output: 0, total_cache_read: 800, total_uncached_input: 150, total_cache_write: 50 };
  const view = overrides => project({ hosts: [host({ agents: [{ id: 't1', status: 'working',
    project: 'Example', branch: 'feat/example', checkout: 'unrelated-directory',
    technical: { telemetry: { ...telemetry, ...overrides } } }] })], allowances: [] }, now).threads[0];
  const thread = view({});
  assert.equal(thread.branch, 'feat/example');
  assert.equal(thread.usage.contextPercent, 55);
  assert.equal(thread.usage.inputTokens, 1000);
  assert.equal(thread.usage.outputTokens, 0);
  assert.equal(thread.usage.cachePercent, 80);
  const lastReported = view({ usage_seq: (now - 121000) * 1000 }).usage;
  assert.equal(lastReported.inputTokens, 1000);
  assert.equal(lastReported.stale, true);
  assert.equal(lastReported.age, '2m ago');
  assert.equal(thread.usage.stale, false);
  assert.equal(view({ usage_seq: (now + 1) * 1000 }).usage.inputTokens, null);
  assert.equal(view({ seq: (now + 1) * 1000 }).usage.contextPercent, null);
  assert.equal(view({ total_uncached_input: 201 }).usage.cachePercent, null);
  assert.equal(view({ total_uncached_input: 201 }).usage.uncachedTokens, null);
  assert.equal(view({}).usage.uncachedTokens, 150);
  assert.equal(view({ context: 101 }).usage.contextPercent, null);
});

test('checkout directory names are never presented as Git branches', () => {
  const thread = project({ hosts: [host({ agents: [{ id: 't1', project: 'Example', checkout: 'feat-example' }] })], allowances: [] }, now).threads[0];
  assert.equal(thread.branch, '');
  assert.equal(thread.checkout, 'feat-example');
  assert.equal(thread.usage.inputTokens, null);
});

test('menubar state uses blocked, working, done, idle precedence without counts', () => {
  const { dominantState } = sandbox.module.exports;
  const states = (...names) => names.map(state => ({ state }));
  assert.equal(dominantState(states('idle', 'done', 'working', 'blocked')), 'blocked');
  assert.equal(dominantState(states('done', 'working', 'idle')), 'working');
  assert.equal(dominantState(states('idle', 'done')), 'done');
  assert.equal(dominantState(states('unknown', 'idle')), 'idle');
  assert.equal(dominantState([]), 'idle');
  const expired = project({ hosts: [host({ sampled_at: now / 1000 - 90 })], allowances: [], interval: 5 }, now);
  assert.equal(dominantState(expired.threads), 'idle');
});


test('allowance reset metadata and bounded pace strength retain source validity', () => {
  const base = { label: 'Personal', available: true, sampled_at: now / 1000 - 10,
    weekly_remaining: 60, weekly_resets_at: now / 1000 + 302400, reset_count: 0, reset_expires_at: null };
  const view = changes => project({ hosts: [host()], allowances: [{ ...base, ...changes }] }, now).allowances[0];
  assert.equal(view({}).paceStrength, 2/3);
  assert.equal(view({}).reset, '3d 12h');
  assert.equal(view({}).resetCount, 0);
  assert.equal(view({ reset_count: 4 }).resetCount, 4);
  assert.equal(view({ reset_count: 4, reset_expires_at: now / 1000 }).resetCount, null);
  assert.equal(view({ reset_count: -1 }).resetCount, null);
  assert.equal(view({ reset_count: 1.5 }).resetCount, null);
  assert.equal(view({ sampled_at: now / 1000 - 601 }).resetCount, null);
  assert.equal(view({ sampled_at: now / 1000 - 601 }).reset, null);
  assert.equal(view({ weekly_remaining: null }).reset, '3d 12h');
  assert.equal(view({ weekly_remaining: null }).paceStrength, 0);
  assert.equal(view({ weekly_resets_at: now / 1000 + 7200 }).reset, '0d 2h');
  assert.equal(view({ weekly_resets_at: now / 1000 + 60 }).reset, '0d <1h');
});

test('fleet active counts include working and blocked, with unknown distinct from zero', () => {
  const agents = ['working', 'blocked', 'idle', 'done', 'unknown'].map((status, i) => ({id: String(i), status}));
  const view = project({hosts:[host({agents}),host({id:'offline',online:false}),host({id:'empty',agents:[]})],allowances:[]},now);
  assert.equal(view.hosts[0].activeThreads,2);
  assert.equal(view.hosts[1].activeThreads,null);
  assert.equal(view.hosts[2].activeThreads,0);
});

test('subagent observations keep stop counts and original age without inventing a completion ratio', () => {
  const base={seq:now*1000,subagent_starts:2,subagent_stops:3,subagent_seq:(now-1000)*1000};
  const view = overrides => project({hosts:[host({agents:[{id:'t',technical:{telemetry:{...base,...overrides}}}]})],allowances:[]},now).threads[0].children;
  assert.equal(view({}).stops,3); // Multiple stops can be observed for one resumed child.
  assert.equal(view({}).starts,2);
  assert.equal(view({subagent_seq:(now-121000)*1000}).stale,true);
  assert.equal(view({subagent_seq:(now+1)*1000}),null);
  assert.equal(view({subagent_starts:1000}),null);
  assert.equal(view({subagent_stops:null}),null);
  assert.equal(view({subagent_seq:null}),null);
  assert.equal(view({subagent_starts:0,subagent_stops:0,subagent_seq:null}).starts,0);
});

test('FX match thread identity across reorder and ignore initial, expired, unchanged and reset observations', () => {
  const {transitions}=sandbox.module.exports;
  const thread=(id,state,children=null)=>({id,hostId:'host',state,children});
  const a=thread('a','working'), b=thread('b','idle');
  assert.equal(Object.keys(transitions([], [a,b])).length,0);
  assert.equal(Object.keys(transitions([a,b],[b,a])).length,0);
  assert.equal(transitions([a,b],[thread('b','working'),thread('a','done')])['host:a'].state,true);
  const c=(starts,stops,stamp,stale=false)=>({starts,stops,stamp,stale});
  const change=(before,after)=>transitions([thread('a','working',before)],[thread('a','working',after)]);
  assert.equal(change(c(0,0,null),c(2,0,10))['host:a'].children,true);
  assert.equal(change(c(2,0,10),c(2,1,20))['host:a'].children,true);
  assert.equal(Object.keys(change(c(2,0,10),c(2,1,20,true))).length,0);
  assert.equal(Object.keys(change(c(2,1,20),c(0,0,null))).length,0);
  assert.equal(Object.keys(change(c(2,1,20),c(2,1,20))).length,0);
  assert.equal(Object.keys(transitions([a],[])).length,0);
});

test('completion dial uses native distinct-child counts rather than stop-event ratios', () => {
  const base={seq:now*1000,subagent_total:4,subagent_done:2,subagent_status_seq:(now-1000)*1000};
  const view=change=>project({hosts:[host({agents:[{id:'t',technical:{telemetry:{...base,...change}}}]})],allowances:[]},now).threads[0].completion;
  assert.equal(view({}).done,2);
  assert.equal(view({}).total,4);
  assert.equal(view({subagent_done:5}),null);
  assert.equal(view({subagent_status_seq:(now+1)*1000}),null);
  assert.equal(view({subagent_status_seq:(now-121000)*1000}).stale,true);
  assert.equal(view({subagent_total:0,subagent_done:0}).done,0);
});


test('machine groups preserve host order, navigation targets and empty or unavailable hosts', () => {
  const view = project({ interval: 5, hosts: [
    host({id:'first',label:'Same',agents:[{id:'one',status:'idle',project:'Z'}]}),
    host({id:'second',label:'Same',agents:[{id:'two',status:'working',project:'A'}]}),
    host({id:'empty',agents:[]}),
    host({id:'offline',online:false,agents:[{id:'hidden',status:'working'}]})
  ], allowances:[] }, now);
  const groups = sandbox.module.exports.groupThreads(view);
  assert.deepEqual(Array.from(groups, g => g.host.id), ['first','second','empty','offline']);
  assert.deepEqual(Array.from(groups, g => Array.from(g.indices, i => view.threads[i].id)), [['one'],['two'],[],[]]);
  assert.equal(groups[2].host.reporting, true);
  assert.equal(groups[3].host.reporting, false);
  assert.equal(sandbox.module.exports.groupThreads(project(null, now)).length, 0);
});

test('status filters and collapsed machines preserve totals and remove hidden navigation targets', () => {
  const view = project({hosts:[host({agents:[
    {id:'idle',status:'idle'}, {id:'working',status:'working'},
    {id:'blocked',status:'blocked'}, {id:'done',status:'done'}
  ]})],allowances:[]},now);
  const filtered = sandbox.module.exports.groupThreads(view, ['idle','done'], []);
  assert.equal(filtered[0].total,4);
  assert.equal(filtered[0].matching,2);
  assert.deepEqual(Array.from(filtered[0].indices,i=>view.threads[i].id),['working','blocked']);
  const collapsed = sandbox.module.exports.groupThreads(view, ['idle'], ['iapetus']);
  assert.equal(collapsed[0].collapsed,true);
  assert.equal(collapsed[0].matching,3);
  assert.equal(collapsed[0].indices.length,0);
  const hidden = sandbox.module.exports.groupThreads(view,['idle','working','blocked','done'],[]);
  assert.equal(hidden[0].total,4); assert.equal(hidden[0].matching,0);
  assert.equal(view.working,1); // A filter never alters the menubar aggregate.
});


test('every measured deficit is visible and pacing never divides by a tiny time balance', () => {
  const allowance = {provider:'claude',provider_label:'Claude',account_id:'office',label:'Office',available:true,
    sampled_at:now/1000,weekly_remaining:49.99,weekly_resets_at:now/1000+1800,window_seconds:3600};
  const account = project({hosts:[host()],allowances:[allowance]},now).allowances[0];
  assert.equal(account.pace,'deficit');
  assert.equal(account.timeRemaining,50);
  assert.ok(account.paceStrength>0 && account.paceStrength<1);
  assert.equal(account.provider,'claude');
  assert.equal(account.id,'office');
  assert.equal(account.pacePercent,undefined);
  const nearReset=project({hosts:[host()],allowances:[{...allowance,weekly_remaining:20,weekly_resets_at:now/1000+1}]},now).allowances[0];
  assert.equal(nearReset.paceStrength,1);
  assert.equal(nearReset.pace,'reserve');
  const even=project({hosts:[host()],allowances:[{...allowance,weekly_remaining:50}]},now).allowances[0];
  assert.equal(even.pace,'even');
});

test('providers group any configured accounts and retain legacy identities', () => {
  const rows=[{label:'Personal'},{provider:'claude',provider_label:'Claude',account_id:'team',label:'Team'},
    {label:'Work'},{provider:'codex',account_id:'third',label:'Third'}];
  const view=project({hosts:[host()],allowances:rows},now);
  const groups=sandbox.module.exports.providerGroups(view.allowances);
  assert.equal(groups.length,3); assert.equal(groups[0].accounts.length,3);
  assert.equal(groups[2].id,"notion");
  assert.equal(groups[2].accounts[0].setupRequired,true);
  assert.equal(groups[1].label,'Claude');
  assert.equal(groups[0].accounts[0].id,'Personal');
  assert.equal(groups[0].accounts[2].id,'third');
});

test('subagent outcomes must form a complete partition, legacy remainder is unresolved', () => {
  const base={seq:now*1000,subagent_total:5,subagent_done:1,subagent_status_seq:now*1000,
    subagent_running:1,subagent_interrupted:1,subagent_failed:1,subagent_unknown:1};
  const view=change=>project({hosts:[host({agents:[{id:'t',technical:{telemetry:{...base,...change}}}]})],allowances:[]},now).threads[0].completion;
  assert.equal(view({}).outcomes.failed,1);
  assert.match(sandbox.module.exports.childHint(view({})),/1 completed.*1 running.*1 interrupted.*1 failed.*1 unavailable/);
  assert.equal(view({subagent_running:2}).outcomes,null);
  assert.match(sandbox.module.exports.childHint(view({subagent_running:null})),/4 unresolved/);
});

test('row order and arrival detection survive changing states, reconnect and filters', () => {
  const {stableThreads,arrivals}=sandbox.module.exports;
  const a={id:'a',hostId:'h',state:'idle'}, b={id:'b',hostId:'h',state:'working'}, c={id:'c',hostId:'h',state:'working'};
  assert.deepEqual(Array.from(stableThreads([a,b],[b,a,c]),x=>x.id),['a','b','c']);
  const snapshot=(threads,reporting=true)=>({threads,hosts:[{id:'h',reporting}]});
  assert.deepEqual(Object.keys(arrivals(snapshot([a,b]),snapshot([b,a,c]))),['h:c']);
  assert.deepEqual(Object.keys(arrivals(snapshot([],false),snapshot([a,b]))),[]);
  assert.deepEqual(Object.keys(arrivals({threads:[],hosts:[]},snapshot([a,b]))),[]);
  assert.deepEqual(Object.keys(arrivals(snapshot([a,b]),snapshot([b,a]))),[]);
});

test('acknowledgement suppresses only the matching done episode and honours higher priority', () => {
  const {dominantState,threadKey,completionEpisode}=sandbox.module.exports;
  const done={id:'a',hostId:'h',state:'done',generation:1,statusGeneration:10};
  const acknowledged={[threadKey(done)]:completionEpisode(done)};
  assert.equal(dominantState([done],acknowledged),'idle');
  assert.equal(dominantState([{...done,statusGeneration:11}],acknowledged),'done');
  assert.equal(dominantState([{...done,generation:2}],acknowledged),'done');
  assert.equal(dominantState([done,{state:'working'}],acknowledged),'working');
  assert.equal(dominantState([done,{state:'blocked'}],acknowledged),'blocked');
});

test('compaction effects require a measured increase in the same session', () => {
  const thread=(compactions,generation=1,stale=false)=>({id:'a',hostId:'h',state:'working',generation,usage:{compactions,stale}});
  const changes=(before,after)=>sandbox.module.exports.transitions([before],[after]);
  assert.equal(changes(thread(0),thread(1))['h:a'].compaction,true);
  assert.deepEqual(Object.keys(changes(thread(null),thread(1))),[]);
  assert.deepEqual(Object.keys(changes(thread(0),thread(1,2))),[]);
  assert.deepEqual(Object.keys(changes(thread(0),thread(1,1,true))),[]);
});


test('provider names do not collide with JavaScript object properties', () => {
  const groups=sandbox.module.exports.providerGroups([{provider:'constructor',providerLabel:'Example',id:'one'}]);
  assert.equal(groups.length,1); assert.equal(groups[0].accounts[0].id,'one');
});


test('allowance pace bands use absolute percentage-point boundaries', () => {
  const band = sandbox.module.exports.allowancePaceBand;
  for (const [difference, expected] of [[100,'surplus'],[0,'surplus'],[-0,'surplus'],[-0.001,'caution'],[-5,'caution'],[-5.001,'warning'],[-9.999,'warning'],[-10,'deficit'],[-100,'deficit'],[null,'unknown'],[undefined,'unknown'],[NaN,'unknown'],[Infinity,'unknown'],['0','unknown']]) {
    assert.equal(band(difference), expected, String(difference));
  }
});


test('signed pace rounds symmetrically and shares displayed threshold meaning', () => {
  const reading = sandbox.module.exports.allowancePaceReading;
  for (const [value, text, band] of [[4.24,'+4.2%','surplus'],[-4.24,'−4.2%','caution'],[-5.04,'−5.0%','caution'],[-5.05,'−5.1%','warning'],[-9.95,'−10.0%','deficit'],[0,'0.0%','surplus'],[-0.04,'0.0%','surplus'],[0.04,'0.0%','surplus'],[-0.05,'−0.1%','caution'],[0.05,'+0.1%','surplus'],[null,'—','unknown'],[NaN,'—','unknown'],[Infinity,'—','unknown']]) {
    const result = reading(value);
    assert.equal(result.text, text);
    assert.equal(result.band, band);
    assert.equal(Object.is(result.difference, -0), false);
  }
});


test('native turn timing measures active wall time and accumulated finished turns', () => {
  const {turnTiming,timingHint,durationLabel}=sandbox.module.exports;
  const data={active:true,started_at_s:now/1000-75,observed_at_s:now/1000-2,freshness_seconds:12,
    last_duration_s:42,last_outcome:'aborted',total_finished_duration_s:400,complete:true};
  const timing=turnTiming({technical:{turn_timing:data}},now,true);
  assert.equal(timing.elapsed,75);assert.equal(timing.total,475);assert.equal(timing.stale,false);
  assert.equal(durationLabel(timing.elapsed),'1m 15s');assert.match(timingHint(timing),/Total turn time 7m 55s/);
  const finished=turnTiming({technical:{turn_timing:{...data,active:false}}},now,true);
  assert.equal(finished.elapsed,42);assert.equal(finished.total,400);assert.match(timingHint(finished),/Last wall-clock turn 42s · interrupted/);
});

test('stale active timing freezes at the verified observation and honours the declared source cadence', () => {
  const {turnTiming}=sandbox.module.exports;
  const data={active:true,started_at_s:now/1000-100,observed_at_s:now/1000-20,freshness_seconds:12,total_finished_duration_s:20,complete:true};
  const timing=(overrides,offset=0,reporting=true)=>turnTiming({technical:{turn_timing:{...data,...overrides}}},now+offset,reporting);
  assert.equal(timing({}).elapsed,80);assert.equal(timing({}).stale,true);assert.equal(timing({},50000).elapsed,80);
  assert.equal(timing({freshness_seconds:180}).elapsed,100);assert.equal(timing({freshness_seconds:180}).stale,false);
  assert.equal(timing({freshness_seconds:180},0,false).elapsed,80);
  assert.equal(timing({complete:false}).total,null);
  assert.equal(timing({observed_at_s:now/1000+1.001}),null);
  assert.equal(timing({freshness_seconds:181}),null);
  assert.equal(timing({started_at_s:now/1000}).elapsed,null);
});

test('thread time never derives from Herdr status age or incomplete native records', () => {
  const agent={id:'a',status:'working',since:now/1000-999,technical:{}};
  assert.equal(project({hosts:[host({agents:[agent]})],allowances:[]},now).threads[0].timing,null);
  const {turnTiming}=sandbox.module.exports;
  const partial={observed_at_s:now/1000,active:null,complete:false,total_finished_duration_s:null,last_duration_s:null};
  const reading=turnTiming({technical:{turn_timing:partial}},now,true);
  assert.equal(reading.elapsed,null);assert.equal(reading.total,null);
});

test('machine connection state distinguishes initial connection from established failure', () => {
  const view=project({hosts:[host({id:'connecting',online:false,connection_state:'connecting'}),
    host({id:'down',online:false,connection_state:'unreachable'}),host({id:'up',connection_state:'connected'})],allowances:[]},now);
  assert.deepEqual(Array.from(view.hosts,h=>h.connectionState),['connecting','unreachable','connected']);
  assert.deepEqual(Array.from(view.hosts,h=>h.connectionLabel),['Connecting','Unreachable','Connected']);
});

test('setup needed is a machine hint and never contributes blocked thread priority', () => {
  const view=project({fleet_discovery:{state:'available'},hosts:[host(),host({id:'new-profile',connection_state:'setup_needed',online:true,agents:[{id:'stale',status:'blocked'}]})],allowances:[]},now);
  assert.equal(view.hosts[1].connectionLabel,'Setup needed');
  assert.equal(view.hosts[1].reporting,false);
  assert.equal(view.hosts[1].activeThreads,null);
  assert.equal(view.threads.length,1);
  assert.equal(sandbox.module.exports.dominantState(view.threads),'working');
});

test('discovery health retains observed hosts and survives an empty accepted inventory', () => {
  const view=project({fleet_discovery:{state:'unavailable'},hosts:[host()],allowances:[]},now);
  assert.equal(view.discoveryLabel,'Discovery unavailable');
  assert.equal(view.hosts[0].connectionLabel,'Connected');
  assert.equal(view.threads.length,1);
  assert.equal(view.hosts[0].age,'5s ago');
  const empty=project({fleet_discovery:{state:'unavailable'},hosts:[],allowances:[]},now);
  assert.equal(empty.connected,true);
  assert.equal(empty.discoveryLabel,'Discovery unavailable');
  assert.equal(project({hosts:[],allowances:[]},now).discoveryLabel,'');
});

test('navigation retains opaque exact profile binding and rejects malformed bindings', () => {
  const navigation={profile_id:'profile-one',route_key:'a'.repeat(64)};
  const view=project({hosts:[host({id:'legacy-host',label:'Mutable label',navigation,agents:[{id:'legacy-host:w1:p2',status:'working',navigation}]})],allowances:[]},now);
  const args=sandbox.module.exports.navigationArgs(view.threads[0]);
  assert.deepEqual(Array.from(args.slice(0,3)),['--open-thread','legacy-host','legacy-host:w1:p2']);
  assert.deepEqual(JSON.parse(args[3]),navigation);
  const fallback=project({hosts:[host({navigation,agents:[{id:'iapetus:w1:p2',navigation:null}]})],allowances:[]},now);
  assert.deepEqual(JSON.parse(sandbox.module.exports.navigationArgs(fallback.threads[0])[3]),navigation);
  for(const binding of [{},false,[],{...navigation,route_key:'short'},{...navigation,target:'other'},{...navigation,profile_id:7}]){
    assert.equal(sandbox.module.exports.navigationArgs({...view.threads[0],navigation:binding}),null);
  }
  assert.equal(sandbox.module.exports.navigationArgs({hostId:'local',id:'local:w1:p2',navigation:null}).length,3);
});

test('inventory labels and discovery health do not replay thread entrance or status effects', () => {
  const navigation={profile_id:'p',route_key:'a'.repeat(64)};
  const raw={fleet_discovery:{state:'available'},hosts:[host({navigation})],allowances:[]};
  const before=project(raw,now);
  const after=project({...raw,fleet_discovery:{state:'unavailable'},hosts:[{...raw.hosts[0],label:'Renamed machine'}]},now);
  assert.deepEqual(Object.keys(sandbox.module.exports.arrivals(before,after)),[]);
  assert.deepEqual(Object.keys(sandbox.module.exports.transitions(before.threads,after.threads)),[]);
  assert.equal(sandbox.module.exports.groupThreads(after,[],['iapetus'])[0].indices.length,0);
  const moved=project({...raw,hosts:[host({navigation:{...navigation,route_key:'b'.repeat(64)},agents:[{id:'new',status:'working'}]})]},now);
  assert.deepEqual(Object.keys(sandbox.module.exports.arrivals(before,moved)),[]);
  assert.notEqual(sandbox.module.exports.completionEpisode(before.threads[0]),sandbox.module.exports.completionEpisode(moved.threads[0]));
});


test('native timing tolerates one second of fleet clock skew without renewing source age', () => {
  const {turnTiming,ageSeconds}=sandbox.module.exports;
  const source={active:true,started_at_s:now/1000-75,observed_at_s:now/1000+0.019,
    freshness_seconds:12,last_duration_s:30,total_finished_duration_s:100,complete:true};
  const agent={technical:{turn_timing:source}};
  const initial=turnTiming(agent,now,true);
  assert.equal(initial.elapsed,75);assert.equal(initial.age,'0s ago');assert.equal(initial.stale,false);
  assert.equal(initial.observedAt,source.observed_at_s);
  const justStarted=turnTiming({technical:{turn_timing:{...source,started_at_s:now/1000+0.01}}},now,true);
  assert.equal(justStarted.elapsed,0);assert.equal(justStarted.age,'0s ago');
  assert.equal(justStarted.observedAt,source.observed_at_s);
  const later=turnTiming(agent,now+2000,true);
  assert.equal(later.elapsed,77);assert.equal(later.age,'1s ago');assert.equal(later.observedAt,initial.observedAt);
  const stale=turnTiming(agent,now+13000,true);
  assert.equal(stale.elapsed,75);assert.equal(stale.stale,true);assert.equal(stale.observedAt,initial.observedAt);
  assert.equal(turnTiming({technical:{turn_timing:{...source,observed_at_s:now/1000+1}}},now,true).elapsed,75);
  assert.equal(turnTiming({technical:{turn_timing:{...source,observed_at_s:now/1000+1.001}}},now,true),null);
  assert.equal(turnTiming({technical:{turn_timing:{...source,started_at_s:now/1000+0.02}}},now,true).elapsed,null);
  assert.equal(ageSeconds(source.observed_at_s,now),null); // Generic freshness remains strict.
});

test('bounded allowance skew preserves usage rejection and expiry boundaries', () => {
  const agent={id:'a',status:'working',technical:{turn_timing:{active:true,started_at_s:now/1000-10,
    observed_at_s:now/1000+0.019,freshness_seconds:12,complete:false},
    telemetry:{seq:now*1000+19000,usage_seq:now*1000+19000,total_input:1000,total_output:10}}};
  const row={label:'Personal',available:true,sampled_at:now/1000+0.019,weekly_remaining:70,weekly_resets_at:now/1000+100};
  const view=project({hosts:[host({agents:[agent]})],allowances:[row]},now);
  assert.equal(view.threads[0].timing.elapsed,10);
  assert.equal(view.threads[0].usage.inputTokens,null);
  assert.equal(view.allowances[0].remaining,70);
  for (const offset of [1, -600]) {
    assert.equal(project({hosts:[host({agents:[]})],allowances:[{...row,sampled_at:now/1000+offset}]},now).allowances[0].remaining,70);
  }
  for (const offset of [1.001, -600.001]) {
    assert.equal(project({hosts:[host({agents:[]})],allowances:[{...row,sampled_at:now/1000+offset}]},now).allowances[0].remaining,null);
  }
  const expired=project({hosts:[host({agents:[]})],allowances:[{...row,weekly_resets_at:now/1000,reset_count:2,reset_expires_at:now/1000}]},now).allowances[0];
  assert.equal(expired.remaining,null);
  assert.equal(expired.resetCount,null);
});


test('configured navigation bindings stay opaque and reject ambiguous identities', () => {
  const state = sandbox.module.exports
  const binding = {host_id: 'custom-local', route_key: 'a'.repeat(64)}
  const thread = {hostId: 'custom-local', id: 'custom-local:w1:p2', navigation: binding}
  assert.deepEqual(Array.from(state.navigationArgs(thread)), ['--open-thread', thread.hostId, thread.id, JSON.stringify(binding)])
  assert.equal(state.navigationArgs({...thread, navigation: {...binding, profile_id: 'saved'}}), null)
  assert.equal(state.navigationArgs({...thread, navigation: {host_id: 'custom-local', extra: 'a'.repeat(64)}}), null)
})

test('Notion monthly allowance preserves zero and overage without weekly pace', () => {
  const row = {provider:'notion',provider_label:'Notion',account_id:'notion-monthly',label:'Work',available:true,window_seconds:0,sampled_at:now/1000,monthly_used_percent:1.61,monthly_resets_at:now/1000+86400};
  const view = value => project({hosts:[],allowances:[value]},now).allowances[0];
  assert.equal(view(row).used,1.61);
  assert.equal(view(row).remaining,98.39);
  assert.equal(view(row).paceDifference,null);
  assert.equal(view(row).timeRemaining,null);
  assert.ok(view(row).resetDate);
  assert.equal(view({...row,monthly_used_percent:0}).remaining,100);
  assert.equal(view({...row,monthly_used_percent:125}).remaining,0);
  for (const bad of [{monthly_used_percent:null},{monthly_used_percent:-1},{monthly_resets_at:now/1000},{sampled_at:now/1000-601},{available:false}]) {
    assert.equal(view({...row,...bad}).remaining,null);
  }
});

test('unconfigured Notion stays visible as setup without an invented balance', () => {
  const view = project({hosts:[],allowances:[]},now);
  const notion = view.allowances.find(a => a.provider === 'notion');
  assert.equal(notion.setupRequired,true);
  assert.equal(notion.remaining,null);
  assert.equal(notion.resetDate,null);
  const row={provider:'notion',account_id:'notion-monthly',label:'Work',available:false,window_seconds:0};
  const configured=project({hosts:[],allowances:[row]},now).allowances;
  assert.equal(configured.filter(a=>a.provider==='notion').length,1);
  assert.notEqual(configured[0].setupRequired,true);
});

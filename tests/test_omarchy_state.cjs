const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(__dirname, '../omarchy/herdr.observatory/State.js'), 'utf8');
const sandbox = { module: { exports: {} } };
vm.runInNewContext(source, sandbox, { filename: 'State.js' });
const { project, readView, allowanceReading } = sandbox.module.exports;
const now = 1_800_000_000_000;
// The view is structural; time-derived presentation comes from the readings.
const presented = (raw, t = now) => readView(project(raw, t), t);

function host(overrides = {}) {
  return {
    id: 'laptop', label: 'laptop', online: true, sampled_at: now / 1000 - 5,
    agents: [{ id: 'laptop:4', project: 'Example', title: 'Synthetic task', status: 'working', harness: 'codex' }],
    ...overrides
  };
}

// A D1 snapshot allowance row. `balance` is the remaining percentage, so
// existing expectations read as before; the wire carries the used percentage.
function weekly(overrides = {}) {
  return { kind: 'weekly', label: 'Weekly', used_percent: 30, resets_at: now / 1000 + 302400,
    duration_s: 604800, pacing: true, ...overrides };
}
function allowanceRow(overrides = {}, windowOverrides = {}) {
  return { provider: 'codex', provider_label: 'Codex', account_id: 'Personal', label: 'Personal',
    status: 'available', status_text: null, plan: null, sampled_at: now / 1000 - 5,
    reset_count: null, reset_expires_at: null, windows: [weekly(windowOverrides)], ...overrides };
}
function remainingRow(balance, reset, overrides = {}, windowOverrides = {}) {
  return allowanceRow(overrides, { used_percent: balance === null ? null : 100 - balance,
    resets_at: reset, ...windowOverrides });
}

test('unavailable hosts do not become zero-thread evidence', () => {
  const view = project({ interval: 5, hosts: [host(), host({ id: 'second', label: 'second', online: false, agents: [] })], allowances: [] }, now);
  assert.equal(view.working, 1);
  assert.equal(view.partial, true);
  assert.equal(view.hosts[1].reporting, false);
  assert.equal(view.note, '1 source unavailable');
});

test('expired sample removes its threads', () => {
  const view = project({ interval: 5, hosts: [host({ sampled_at: now / 1000 - 30 })], allowances: [] }, now);
  assert.equal(view.threads.length, 0);
  assert.equal(view.hosts[0].reporting, false);
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
    remainingRow(0, now / 1000 + 100, { sampled_at: now / 1000 - 12 }),
    remainingRow(45, now / 1000 - 1, { account_id: 'Work', label: 'Work', sampled_at: now / 1000 - 12 })
  ] }, now);
  assert.equal(view.allowances[0].remaining, 0);
  assert.equal(view.allowances[1].remaining, null);
});

test('host map identity stays correct when display labels match', () => {
  const view = project({ interval: 5, hosts: [
    host({ id: 'laptop', label: 'shared' }),
    host({ id: 'second', label: 'shared', agents: [] })
  ], allowances: [] }, now);
  assert.equal(view.threads[0].hostId, 'laptop');
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
  const allowance = (balance = 70, reset = now / 1000 + 302400) => remainingRow(balance, reset);
  const view = row => presented({ hosts: [host()], allowances: [row] }).allowances[0];
  assert.equal(view(allowance()).timeRemaining, 50);
  assert.equal(view(allowance()).paceDifference, 20);
  assert.ok(view(allowance()).paceDifference > 0);
  assert.ok(view(allowance(30)).paceDifference < 0);
  assert.ok(view(allowance(53)).paceDifference > 0);
  assert.ok(view(allowance(47)).paceDifference < 0);
  assert.ok(view(allowance(53.1)).paceDifference > 0);
  assert.ok(view(allowance(46.9)).paceDifference < 0);
  assert.equal(view(allowance()).reset, '3d 12h');
  assert.equal(view(allowance(0)).paceDifference, -50);
  assert.equal(view(allowance(70, now / 1000 + 604801)).timeRemaining, null);
  const expired = view(allowance(70, now / 1000));
  assert.equal(expired.remaining, null);
  assert.equal(expired.timeRemaining, null);
  assert.equal(expired.paceDifference, null);
});

test('thread instruments use session counters and the original usage timestamp', () => {
  const telemetry = { seq: now * 1000, usage_seq: (now - 10000) * 1000,
    context: 60, window: 100, context_percent: 55,
    total_input: 1000, total_output: 0, total_cache_read: 800, total_uncached_input: 150, total_cache_write: 50 };
  const view = overrides => presented({ hosts: [host({ agents: [{ id: 't1', status: 'working',
    project: 'Example', branch: 'feat/example', checkout: 'unrelated-directory',
    technical: { telemetry: { ...telemetry, ...overrides } } }] })], allowances: [] }).threads[0];
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


test('allowance reset metadata retains source validity', () => {
  const base = { sampled_at: now / 1000 - 10, reset_count: 0, reset_expires_at: null };
  const view = (changes, window = {}) => presented({ hosts: [host()],
    allowances: [remainingRow(60, now / 1000 + 302400, { ...base, ...changes }, window)] }).allowances[0];
  assert.equal(view({}).paceDifference, 10);
  assert.equal(view({}).reset, '3d 12h');
  assert.equal(view({}).resetCount, 0);
  assert.equal(view({ reset_count: 4 }).resetCount, 4);
  assert.equal(view({ reset_count: 4, reset_expires_at: now / 1000 }).resetCount, null);
  assert.equal(view({ reset_count: -1 }).resetCount, null);
  assert.equal(view({ reset_count: 1.5 }).resetCount, null);
  assert.equal(view({ sampled_at: now / 1000 - 601 }).resetCount, null);
  assert.equal(view({ sampled_at: now / 1000 - 601 }).reset, null);
  assert.equal(view({}, { used_percent: null }).reset, '3d 12h');
  assert.equal(view({}, { used_percent: null }).paceDifference, null);
  assert.equal(view({}, { used_percent: null }).resetCount, 0);
  assert.equal(view({}, { resets_at: now / 1000 + 7200 }).reset, '0d 2h');
  assert.equal(view({}, { resets_at: now / 1000 + 60 }).reset, '0d <1h');
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
  const collapsed = sandbox.module.exports.groupThreads(view, ['idle'], ['laptop']);
  assert.equal(collapsed[0].collapsed,true);
  assert.equal(collapsed[0].matching,3);
  assert.equal(collapsed[0].indices.length,0);
  const hidden = sandbox.module.exports.groupThreads(view,['idle','working','blocked','done'],[]);
  assert.equal(hidden[0].total,4); assert.equal(hidden[0].matching,0);
  assert.equal(view.working,1); // A filter never alters the menubar aggregate.
});


test('every measured deficit is visible and pacing never divides by a tiny time balance', () => {
  const identity={provider:'claude',provider_label:'Claude',account_id:'office',label:'Office',sampled_at:now/1000};
  const window={kind:'session',label:'Session',duration_s:3600};
  const allowance=(balance,reset)=>remainingRow(balance,reset,identity,window);
  const account = presented({hosts:[host()],allowances:[allowance(49.99,now/1000+1800)]}).allowances[0];
  assert.ok(account.paceDifference<0);
  assert.equal(account.timeRemaining,50);
  assert.equal(account.provider,'claude');
  assert.equal(account.id,'office');
  assert.equal(account.pacePercent,undefined);
  const nearReset=presented({hosts:[host()],allowances:[allowance(20,now/1000+1)]}).allowances[0];
  assert.ok(nearReset.paceDifference>0);
  const even=presented({hosts:[host()],allowances:[allowance(50,now/1000+1800)]}).allowances[0];
  assert.equal(even.paceDifference,0);
});

test('providers group any configured accounts in configured order', () => {
  const unavailable={status:'unavailable',sampled_at:null,windows:[]};
  const rows=[allowanceRow(unavailable),allowanceRow({...unavailable,provider:'claude',provider_label:'Claude',account_id:'team',label:'Team'}),
    allowanceRow({...unavailable,account_id:'Work',label:'Work'}),allowanceRow({...unavailable,account_id:'third',label:'Third'})];
  const view=project({hosts:[host()],allowances:rows},now);
  const groups=sandbox.module.exports.providerGroups(view.allowances);
  assert.equal(groups.length,2); assert.equal(groups[0].accounts.length,3);
  assert.equal(groups[0].label,'Codex');
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
  assert.equal(view.threads.length,1);
  assert.equal(sandbox.module.exports.dominantState(view.threads),'working');
});

test('discovery health retains observed hosts and survives an empty accepted inventory', () => {
  const view=project({fleet_discovery:{state:'unavailable'},hosts:[host()],allowances:[]},now);
  assert.equal(view.discoveryLabel,'Discovery unavailable');
  assert.equal(view.hosts[0].connectionLabel,'Connected');
  assert.equal(view.threads.length,1);
  assert.equal(view.hosts[0].reporting,true);
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
  const fallback=project({hosts:[host({navigation,agents:[{id:'laptop:w1:p2',navigation:null}]})],allowances:[]},now);
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
  assert.equal(sandbox.module.exports.groupThreads(after,[],['laptop'])[0].indices.length,0);
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
  const row=remainingRow(70,now/1000+100,{sampled_at:now/1000+0.019});
  const view=presented({hosts:[host({agents:[agent]})],allowances:[row]});
  assert.equal(view.threads[0].timing.elapsed,10);
  assert.equal(view.threads[0].usage.inputTokens,null);
  assert.equal(view.allowances[0].remaining,70);
  for (const offset of [1, -600]) {
    assert.equal(project({hosts:[host({agents:[]})],allowances:[{...row,sampled_at:now/1000+offset}]},now).allowances[0].remaining,70);
  }
  for (const offset of [1.001, -600.001]) {
    assert.equal(project({hosts:[host({agents:[]})],allowances:[{...row,sampled_at:now/1000+offset}]},now).allowances[0].remaining,null);
  }
  const expired=project({hosts:[host({agents:[]})],allowances:[remainingRow(70,now/1000,{sampled_at:now/1000+0.019,reset_count:2,reset_expires_at:now/1000})]},now).allowances[0];
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

test('projection omits unused presentation fields', () => {
  const view = project({ interval: 5, fleet_discovery: { state: 'available' }, hosts: [host({ metrics: { cpu_percent: 5, memory: { used: 1, total: 2 }, gpu: { percent: 5, used: 1, total: 2 } } })],
    allowances: [allowanceRow({ plan: 'pro', daily_usage: [{ date: '2026-09-20', tokens: 20 }] })] }, now);
  for (const key of ['gpu', 'inference', 'discoveryState']) assert.equal(key in view, false, key);
  for (const key of ['activeThreads', 'cpu', 'memory', 'gpu', 'vram']) assert.equal(key in view.hosts[0], false, key);
  for (const key of ['activity', 'paceStrength', 'pace']) assert.equal(key in view.allowances[0], false, key);
  for (const key of ['gpu', 'inference', 'discoveryState']) assert.equal(key in project(null, now), false, key);
  assert.equal(allowanceReading(view.allowances[0], now).paceDifference, 20);
  assert.equal(view.discoveryLabel, '');
});

test('receipt timeout derives from the stated heartbeat with a safe fallback', () => {
  const { receiptTimeoutMs } = sandbox.module.exports;
  assert.equal(receiptTimeoutMs({ heartbeat_seconds: 4 }), 6000);
  assert.equal(receiptTimeoutMs({ heartbeat_seconds: 10 }), 12000);
  assert.equal(receiptTimeoutMs({ heartbeat_seconds: 1 }), 3000);
  assert.equal(receiptTimeoutMs({ heartbeat_seconds: 60 }), 62000);
  for (const value of [undefined, null, '4', 0, -4, NaN, Infinity, 61, true, {}]) {
    assert.equal(receiptTimeoutMs({ heartbeat_seconds: value }), 6000, String(value));
  }
  assert.equal(receiptTimeoutMs({}), 6000);
  assert.equal(receiptTimeoutMs(null), 6000);
  assert.equal(receiptTimeoutMs(undefined), 6000);
});

test('keyboard focus keeps the same thread when an earlier thread disappears', () => {
  const { groupThreads, focusKeys, reconcileFocus, activationKey, threadForKey, threadKey } = sandbox.module.exports;
  const agents = ids => ids.map(id => ({ id, status: 'working', project: 'Project ' + id }));
  const before = project({ hosts: [host({ agents: agents(['a', 'b', 'c']) })], allowances: [] }, now);
  const after = project({ hosts: [host({ agents: agents(['b', 'c']) })], allowances: [] }, now);
  // The retired model stored a position in overview.threads.
  const indexOrder = view => [].concat(...groupThreads(view).map(group => group.indices));
  const focusedIndex = before.threads.findIndex(thread => thread.id === 'b');
  assert.equal(indexOrder(after).indexOf(focusedIndex) >= 0, true); // old reconciliation keeps it
  assert.equal(after.threads[focusedIndex].id, 'c'); // and Enter would open the neighbour
  // Keys identify the thread itself.
  const key = threadKey(before.threads[focusedIndex]);
  const keys = Array.from(focusKeys(after, groupThreads(after)));
  assert.deepEqual(keys, ['laptop:b', 'laptop:c']);
  assert.equal(reconcileFocus(keys, key), 'laptop:b');
  assert.equal(activationKey(keys, reconcileFocus(keys, key)), 'laptop:b');
  assert.equal(threadForKey(after, activationKey(keys, key)).id, 'b');
});

test('focus helpers clear, move and activate by key in visual order', () => {
  const { groupThreads, focusKeys, reconcileFocus, moveFocus, activationKey, threadForKey } = sandbox.module.exports;
  const view = project({ hosts: [
    host({ id: 'one', agents: [{ id: 'x', status: 'idle', project: 'X' }, { id: 'y', status: 'working', project: 'Y' }] }),
    host({ id: 'two', agents: [{ id: 'z', status: 'working', project: 'A' }] })
  ], allowances: [] }, now);
  const keys = Array.from(focusKeys(view, groupThreads(view)));
  assert.deepEqual(keys, ['one:y', 'one:x', 'two:z']); // host order, then thread order
  assert.deepEqual(Array.from(focusKeys(view, groupThreads(view, ['idle'], []))), ['one:y', 'two:z']);
  assert.deepEqual(Array.from(focusKeys(view, groupThreads(view, [], ['one']))), ['two:z']);
  assert.deepEqual(Array.from(focusKeys(view, [])), []);
  assert.deepEqual(Array.from(focusKeys(project(null, now), []), x => x), []);
  assert.equal(reconcileFocus(keys, 'one:x'), 'one:x');
  assert.equal(reconcileFocus(['one:y'], 'one:x'), '');
  assert.equal(reconcileFocus(keys, ''), '');
  assert.equal(moveFocus([], 'one:x', 1), '');
  assert.equal(moveFocus(keys, '', 1), 'one:y');
  assert.equal(moveFocus(keys, 'gone', -1), 'one:y');
  assert.equal(moveFocus(keys, 'one:y', 1), 'one:x');
  assert.equal(moveFocus(keys, 'one:x', 5), 'two:z');
  assert.equal(moveFocus(keys, 'one:x', -5), 'one:y');
  assert.equal(activationKey(keys, 'two:z'), 'two:z');
  assert.equal(activationKey(keys, 'gone'), 'one:y');
  assert.equal(activationKey(keys, ''), 'one:y');
  assert.equal(activationKey([], 'one:x'), '');
  assert.equal(threadForKey(view, 'two:z').id, 'z');
  assert.equal(threadForKey(view, 'gone'), null);
  assert.equal(threadForKey(view, ''), null);
});

// Provider-neutral allowance windows (generalise-anton-allowance-windows D1, D5).
const projectRaw = row => project({ hosts: [host()], allowances: [row] }, now).allowances[0];
const projectOne = row => { const account = projectRaw(row); return account ? allowanceReading(account, now) : account; };
const unknownBalance = account => {
  assert.equal(account.remaining, null);
  assert.equal(account.timeRemaining, null);
  assert.equal(account.paceDifference, null);
};

test('a long allowance window projects from its own duration', () => {
  const account = projectOne(allowanceRow({ provider: 'synthetic', provider_label: 'Synthetic', account_id: 'team', label: 'Team' },
    { kind: 'monthly', label: 'Monthly', used_percent: 25, resets_at: now / 1000 + 15 * 86400, duration_s: 30 * 86400 }));
  assert.equal(account.remaining, 75);
  assert.equal(account.timeRemaining, 50);
  assert.equal(account.paceDifference, 25);
  assert.equal(account.reset, '15d 0h');
  assert.equal(account.providerLabel, 'Synthetic');
});

test('an account needing authentication shows source text and no balance', () => {
  const account = projectOne(allowanceRow({ provider: 'synthetic', provider_label: 'Synthetic', status: 'auth_needed',
    status_text: 'Sign in required', sampled_at: null, windows: [] }));
  assert.equal(account.statusText, 'Sign in required');
  unknownBalance(account);
  assert.equal(account.reset, null);
  assert.equal(account.resetCount, null);
  assert.equal(account.age, 'source unavailable');
  // Status alone gates the balance: the same window shows none unless available.
  const gated = status => projectOne(allowanceRow({ status, reset_count: 1 }));
  assert.equal(gated('available').remaining, 70);
  for (const status of ['auth_needed', 'unavailable']) {
    unknownBalance(gated(status));
    assert.equal(gated(status).resetCount, null, status);
  }
});

test('an unavailable account without source text keeps a null status text', () => {
  const account = projectOne(allowanceRow({ status: 'unavailable', sampled_at: null, windows: [] }));
  assert.equal(account.statusText, null);
  unknownBalance(account);
});

test('the pacing window is the single flagged window, never list order', () => {
  const other = weekly({ kind: 'session', label: 'Session', used_percent: 90, duration_s: 18000, resets_at: now / 1000 + 9000, pacing: false });
  unknownBalance(projectOne(allowanceRow({ windows: [weekly({ pacing: false })] })));
  unknownBalance(projectOne(allowanceRow({ windows: [weekly(), weekly({ kind: 'monthly' })] })));
  const second = projectOne(allowanceRow({ windows: [other, weekly()] }));
  assert.equal(second.remaining, 70);
  assert.equal(second.timeRemaining, 50);
  assert.equal(second.reset, '3d 12h');
  // A non-pacing window never supplies a reset caption.
  assert.equal(projectOne(allowanceRow({ windows: [other] })).reset, null);
});

test('an unknown window kind projects generically', () => {
  const account = projectOne(allowanceRow({}, { kind: 'credits_pool', label: 'Credits pool' }));
  assert.equal(account.remaining, 70);
  assert.equal(account.paceDifference, 20);
});

test('malformed and oversized windows leave balance and pace unknown without throwing', () => {
  const nine = Array.from({ length: 9 }, (_, i) => weekly({ pacing: i === 0 }));
  unknownBalance(projectOne(allowanceRow({ windows: nine })));
  for (const used of [-1, 101, NaN, '40', Infinity]) {
    const account = projectOne(allowanceRow({}, { used_percent: used }));
    assert.equal(account.remaining, null, String(used));
    assert.equal(account.paceDifference, null, String(used));
  }
  for (const duration of [0, 31622401, 1.5, null, '604800', -604800]) unknownBalance(projectOne(allowanceRow({}, { duration_s: duration })));
  for (const kind of ['', 'Weekly', 'a'.repeat(25), 7]) unknownBalance(projectOne(allowanceRow({}, { kind })));
  for (const windowLabel of ['', 'x'.repeat(41), null]) unknownBalance(projectOne(allowanceRow({}, { label: windowLabel })));
  unknownBalance(projectOne(allowanceRow({}, { resets_at: now / 1000 - 1 })));
  assert.equal(projectOne(allowanceRow({}, { resets_at: now / 1000 - 1 })).reset, null);
  const missing = weekly(); delete missing.pacing;
  unknownBalance(projectOne(allowanceRow({ windows: [missing] })));
  unknownBalance(projectOne(allowanceRow({}, { pacing: 'true' })));
  for (const windows of [null, undefined, 'weekly', {}, [null], [7]]) unknownBalance(projectOne(allowanceRow({ windows })));
  // The window bound is inclusive: eight windows with one pacing entry are valid.
  const eight = Array.from({ length: 8 }, (_, i) => weekly({ kind: 'k' + i, pacing: i === 7 }));
  assert.equal(projectOne(allowanceRow({ windows: eight })).remaining, 70);
  assert.equal(projectOne(allowanceRow({}, { duration_s: 31622400 })).remaining, 70);
});

test('stale and future-skewed samples are not current', () => {
  unknownBalance(projectOne(allowanceRow({ sampled_at: now / 1000 - 600.001 })));
  unknownBalance(projectOne(allowanceRow({ sampled_at: now / 1000 + 1.001 })));
  assert.equal(projectOne(allowanceRow({ sampled_at: now / 1000 - 600 })).remaining, 70);
  assert.equal(projectOne(allowanceRow({ sampled_at: now / 1000 + 1 })).remaining, 70);
  unknownBalance(projectOne(allowanceRow({ sampled_at: null })));
});

test('zero stays distinct from unknown', () => {
  const account = projectOne(allowanceRow({ reset_count: 0 }, { used_percent: 100 }));
  assert.equal(account.remaining, 0);
  assert.equal(account.paceDifference, -50);
  assert.equal(account.resetCount, 0);
  assert.equal(projectOne(allowanceRow({}, { used_percent: 0 })).remaining, 100);
});

test('status text is bounded and never synthesised', () => {
  const text = value => projectOne(allowanceRow({ status: 'unavailable', status_text: value, sampled_at: null, windows: [] })).statusText;
  assert.equal(text('x'.repeat(80)), 'x'.repeat(80));
  for (const value of ['x'.repeat(81), '', 'Sign\nin', 'Sign\u0000in', 'Sign\u007fin', 'Sign\u009bin', 7, true, {}, ['Sign in']])
    assert.equal(text(value), null, JSON.stringify(value));
  assert.equal(projectOne(allowanceRow({ status_text: 'Degraded source' })).statusText, 'Degraded source');
});

test('a row without a valid provider or account id is skipped', () => {
  const rows = [allowanceRow({ provider: undefined }), allowanceRow({ provider: '' }), allowanceRow({ provider: 'Codex' }),
    allowanceRow({ provider: 'a'.repeat(33) }), allowanceRow({ provider: '-codex' }), allowanceRow({ provider: 7 }),
    allowanceRow({ account_id: 'bad id', label: 'bad id' }), allowanceRow({ account_id: 'x'.repeat(129) }),
    allowanceRow({ account_id: undefined, label: undefined }), allowanceRow({ provider: 'a'.repeat(32), account_id: 'kept' })];
  const view = project({ hosts: [host()], allowances: rows }, now);
  assert.deepEqual(Array.from(view.allowances, account => account.id), ['kept']);
  assert.equal(view.allowances[0].providerLabel, 'Codex');
  assert.equal(projectOne(allowanceRow({ provider: 'synthetic', provider_label: undefined })).providerLabel, 'synthetic');
});

test('an invalid provider or row label falls back to the provider or account id', () => {
  for (const bad of ['', 'x'.repeat(41), 'Syn\u0007thetic', 'Syn\nthetic', 7, null]) {
    const account = projectOne(allowanceRow({ provider: 'synthetic', provider_label: bad, account_id: 'team', label: bad }));
    assert.equal(account.providerLabel, 'synthetic');
    assert.equal(account.label, 'team');
  }
  const kept = projectOne(allowanceRow({ provider: 'synthetic', provider_label: 'x'.repeat(40), account_id: 'team', label: 'y'.repeat(40) }));
  assert.equal(kept.providerLabel, 'x'.repeat(40));
  assert.equal(kept.label, 'y'.repeat(40));
});

test('an unknown or missing status projects as unavailable', () => {
  assert.equal(projectOne(allowanceRow({ status: 'available', reset_count: 1 })).remaining, 70);
  for (const status of ['ok', 'Available', undefined, null, true]) {
    const account = projectOne(allowanceRow({ status, reset_count: 1 }));
    unknownBalance(account);
    assert.equal(account.resetCount, null);
  }
});

test('a legacy weekly_remaining row projects as unavailable', () => {
  const legacy = { provider: 'codex', provider_label: 'Codex', account_id: 'Personal', label: 'Personal', available: true,
    sampled_at: now / 1000 - 5, weekly_remaining: 70, weekly_resets_at: now / 1000 + 302400, window_seconds: 604800, reset_count: 1 };
  const account = projectOne(legacy);
  unknownBalance(account);
  assert.equal(account.reset, null);
  assert.equal(account.resetCount, null);
});

test('only providers present in the snapshot form groups', () => {
  const rows = [allowanceRow(), allowanceRow({ provider: 'synthetic', provider_label: 'Synthetic', account_id: 'team', label: 'Team' },
    { kind: 'monthly', label: 'Monthly', duration_s: 30 * 86400, resets_at: now / 1000 + 15 * 86400, used_percent: 25 })];
  const view = project({ hosts: [host()], allowances: rows }, now);
  const groups = sandbox.module.exports.providerGroups(view.allowances);
  assert.deepEqual(Array.from(groups, group => group.label), ['Codex', 'Synthetic']);
  const alone = project({ hosts: [host()], allowances: [rows[1]] }, now);
  assert.deepEqual(Array.from(alone.allowances, account => account.provider), ['synthetic']);
  assert.deepEqual(Array.from(sandbox.module.exports.providerGroups(alone.allowances), group => group.id), ['synthetic']);
  assert.equal(project({ hosts: [host()], allowances: [] }, now).allowances.length, 0);
});

test('the allowance reading has exactly the eleven contract keys', () => {
  const keys = ['id', 'provider', 'providerLabel', 'label', 'statusText', 'remaining', 'timeRemaining',
    'paceDifference', 'resetCount', 'reset', 'age'];
  const { allowanceReading } = sandbox.module.exports;
  for (const row of [allowanceRow({ reset_count: 1 }), allowanceRow({ status: 'auth_needed', status_text: 'Sign in required', sampled_at: null, windows: [] })])
    assert.deepEqual(Object.keys(allowanceReading(projectRaw(row), now)), keys);
});

test('account aliases prefer saved names, then the legacy table, then a stable hash', () => {
  const { accountAlias } = sandbox.module.exports;
  const pool = ['Gilfoyle', 'Jared Dunn', 'Monica Hall', 'Big Head'];
  const legacy = { 'codex:Personal': 'Richard Hendricks', 'codex:Work': 'Laurie Bream' };
  const today = (key, aliases) => { let hash = 0; for (let i = 0; i < key.length; i++) hash = ((hash * 31) + key.charCodeAt(i)) >>> 0; return aliases[hash % aliases.length]; };
  const personal = { provider: 'codex', id: 'Personal', label: 'Personal' };
  assert.equal(accountAlias(personal, { 'codex:Personal': 'Saved' }, legacy, pool), 'Saved');
  assert.equal(accountAlias(personal, {}, legacy, pool), 'Richard Hendricks');
  assert.equal(accountAlias({ provider: 'codex', id: 'Work', label: 'Work' }, {}, legacy, pool), 'Laurie Bream');
  const synthetic = { provider: 'synthetic', id: 'Personal', label: 'Personal' };
  assert.equal(accountAlias(synthetic, {}, legacy, pool), today('synthetic:Personal', pool));
  for (const account of [{ provider: 'codex', id: 'third', label: 'Third' }, { provider: 'claude', id: 'team', label: 'Team' }])
    assert.equal(accountAlias(account, null, null, pool), today(account.provider + ':' + account.id, pool));
  // Legacy matching stays by label, and an empty legacy preference falls through.
  assert.equal(accountAlias({ provider: 'codex', id: 'custom', label: 'Personal' }, {}, legacy, pool), 'Richard Hendricks');
  assert.equal(accountAlias(personal, {}, { 'codex:Personal': '' }, pool), today('codex:Personal', pool));
  assert.equal(accountAlias({ provider: 'constructor', id: 'toString', label: 'toString' }, {}, {}, pool), today('constructor:toString', pool));
});

// ---------------------------------------------------------------- A1 helpers
const plain = value => JSON.parse(JSON.stringify(value));
function seeded(seed) {
  let s = seed >>> 0;
  return () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; };
}

test('token formatter keeps the Panel boundaries', () => {
  const { tokens } = sandbox.module.exports;
  assert.equal(tokens(null), '—');
  assert.equal(tokens(undefined), '—');
  assert.equal(tokens(0), '0');
  assert.equal(tokens(999), '999');
  assert.equal(tokens(1000), '1K');
  assert.equal(tokens(1500), '1.5K');
  assert.equal(tokens(999999), '1000K');
  assert.equal(tokens(1.5e6), '1.5M');
  assert.equal(tokens(2e6), '2M');
  assert.equal(tokens(1e9), '1B');
  assert.equal(tokens(2.25e9), '2.3B');
});

test('percent reading and pace text keep the Panel boundaries', () => {
  const { percentReading, paceText } = sandbox.module.exports;
  assert.equal(percentReading(null), '—');
  assert.equal(percentReading(undefined), '—');
  assert.equal(percentReading(0), '0%');
  assert.equal(percentReading(0.4), '<1%');
  assert.equal(percentReading(1), '1%');
  assert.equal(percentReading(42.5), '43%');
  assert.equal(percentReading(99.5), '>99%');
  assert.equal(percentReading(100), '100%');
  assert.equal(paceText({ remaining: null, timeRemaining: 50 }), 'Pace unavailable');
  assert.equal(paceText({ remaining: 50, timeRemaining: null }), 'Pace unavailable');
  assert.equal(paceText({ remaining: 60, timeRemaining: 50 }), '60% left · 50% expected');
  assert.equal(paceText({ remaining: 60.25, timeRemaining: 49.96 }), '60.3% left · 50% expected');
});

test('preference parsing rejects invalid JSON and wrong types', () => {
  const { parseList, parseObject } = sandbox.module.exports;
  assert.deepEqual(plain(parseList('["a","b"]')), ['a', 'b']);
  for (const bad of ['', '{not json', '{}', '"a"', '1', 'null', undefined, null]) assert.deepEqual(plain(parseList(bad)), [], String(bad));
  assert.deepEqual(plain(parseObject('{"a":1}')), { a: 1 });
  for (const bad of ['', '{not json', '[]', '"a"', '1', 'null', undefined, null]) assert.deepEqual(plain(parseObject(bad)), {}, String(bad));
});

test('state colour names, account keys and list toggles', () => {
  const { stateColourName, accountKey, toggleListValue } = sandbox.module.exports;
  assert.deepEqual(['working', 'blocked', 'done', 'idle', 'unknown', undefined].map(stateColourName), ['yellow', 'red', 'green', 'muted', 'muted', 'muted']);
  assert.equal(accountKey({ provider: 'codex', id: 'Personal' }), 'codex:Personal');
  const list = ['a'];
  assert.deepEqual(plain(toggleListValue(list, 'b')), ['a', 'b']);
  assert.deepEqual(plain(toggleListValue(['a', 'b', 'c'], 'b')), ['a', 'c']);
  assert.deepEqual(list, ['a'], 'input unchanged');
});

test('acknowledgements are bounded to 256 by dropping the earliest keys', () => {
  const { boundAcknowledgements } = sandbox.module.exports;
  const input = {};
  for (let i = 0; i < 257; i++) input['h:t' + i] = '1:' + i;
  const bounded = boundAcknowledgements(input);
  assert.equal(Object.keys(bounded).length, 256);
  assert.equal('h:t0' in bounded, false);
  assert.equal(bounded['h:t1'], '1:1');
  assert.equal(bounded['h:t256'], '1:256');
  assert.equal(Object.keys(input).length, 257, 'input unchanged');
  assert.equal(Object.keys(boundAcknowledgements(input, 2)).join(), 'h:t255,h:t256');
});

test('acknowledgement reconcile drops changed listed threads and keeps absent ones', () => {
  const { reconcileAcknowledgements, completionEpisode } = sandbox.module.exports;
  const done = { id: 'a', hostId: 'h', state: 'done', generation: 1, statusGeneration: 10 };
  const acks = { 'h:a': completionEpisode(done), 'h:gone': '1:1' };
  let result = reconcileAcknowledgements(acks, [done]);
  assert.equal(result.changed, false);
  assert.deepEqual(plain(result.value), acks);
  result = reconcileAcknowledgements(acks, [{ ...done, state: 'working' }]);
  assert.equal(result.changed, true);
  assert.deepEqual(plain(result.value), { 'h:gone': '1:1' });
  result = reconcileAcknowledgements(acks, [{ ...done, statusGeneration: 11 }]);
  assert.equal(result.changed, true);
  assert.deepEqual(plain(result.value), { 'h:gone': '1:1' });
  assert.equal(Object.keys(acks).length, 2, 'input unchanged');
});

test('navigation acknowledges only a done target still in the same episode', () => {
  const { acknowledgeNavigation, completionEpisode } = sandbox.module.exports;
  const done = { id: 'a', hostId: 'h', state: 'done', generation: 1, statusGeneration: 10 };
  const target = { key: 'h:a', episode: completionEpisode(done), state: 'done' };
  const acks = { 'h:b': '1:1' };
  assert.deepEqual(plain(acknowledgeNavigation(acks, target, [done])), { 'h:b': '1:1', 'h:a': target.episode });
  assert.deepEqual(acks, { 'h:b': '1:1' }, 'input unchanged');
  assert.equal(acknowledgeNavigation(acks, { ...target, state: 'working' }, [done]), acks);
  assert.equal(acknowledgeNavigation(acks, target, [{ ...done, state: 'working' }]), acks, 'thread changed before exit');
  assert.equal(acknowledgeNavigation(acks, target, [{ ...done, statusGeneration: 11 }]), acks, 'new episode before exit');
  assert.equal(acknowledgeNavigation(acks, target, []), acks, 'thread gone before exit');
  assert.equal(acknowledgeNavigation(acks, null, [done]), acks);
});

test('alias assignment is a deterministic Fisher-Yates shuffle of the pool', () => {
  const { assignAliases } = sandbox.module.exports;
  const pool = ['A', 'B', 'C'];
  const accounts = ['one', 'two', 'three', 'four', 'five'].map(id => ({ provider: 'codex', id }));
  const reference = random => {
    const shuffled = pool.slice();
    for (let i = shuffled.length - 1; i > 0; i--) { const j = Math.floor(random() * (i + 1)); [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]]; }
    return Object.fromEntries(accounts.map((a, i) => ['codex:' + a.id, shuffled[i % shuffled.length]]));
  };
  for (const seed of [1, 2, 3, 99]) assert.deepEqual(plain(assignAliases(accounts, pool, seeded(seed))), reference(seeded(seed)));
  // random() always 0 swaps each position with the first: [B, C, A].
  assert.deepEqual(plain(assignAliases(accounts, pool, () => 0)),
    { 'codex:one': 'B', 'codex:two': 'C', 'codex:three': 'A', 'codex:four': 'B', 'codex:five': 'C' });
  assert.deepEqual(pool, ['A', 'B', 'C'], 'pool unchanged');
});

test('keyed edits turn any key list into any other without removing survivors', () => {
  const { keyedEdits } = sandbox.module.exports;
  const apply = (before, edits) => {
    const list = before.slice();
    for (const edit of edits) {
      if (edit.op === 'remove') { assert.ok(edit.index >= 0 && edit.index < list.length); list.splice(edit.index, 1); }
      else if (edit.op === 'insert') { assert.ok(edit.index >= 0 && edit.index <= list.length); list.splice(edit.index, 0, edit.key); }
      else if (edit.op === 'move') { assert.ok(edit.from >= 0 && edit.from < list.length && edit.to >= 0 && edit.to < list.length); const [key] = list.splice(edit.from, 1); list.splice(edit.to, 0, key); }
      else assert.fail('unknown op ' + edit.op);
    }
    return list;
  };
  const check = (before, after) => {
    const edits = plain(keyedEdits(before, after));
    assert.deepEqual(apply(before, edits), after, JSON.stringify({ before, after }));
    const removed = [], list = before.slice();
    for (const edit of edits) {
      if (edit.op === 'remove') removed.push(list.splice(edit.index, 1)[0]);
      else if (edit.op === 'insert') { assert.equal(before.includes(edit.key), false, 'survivor re-inserted'); list.splice(edit.index, 0, edit.key); }
      else { const [key] = list.splice(edit.from, 1); list.splice(edit.to, 0, key); }
    }
    for (const key of removed) assert.equal(after.includes(key), false, 'survivor removed: ' + key);
    return edits;
  };
  assert.deepEqual(check([], []), []);
  assert.deepEqual(check(['a', 'b'], ['a', 'b']), []);
  assert.deepEqual(check(['a', 'b', 'c'], ['b', 'c']), [{ op: 'remove', index: 0 }]);
  assert.deepEqual(check(['a', 'b'], ['a', 'x', 'b']), [{ op: 'insert', index: 1, key: 'x' }]);
  assert.deepEqual(check(['a', 'b'], ['b', 'a']), [{ op: 'move', from: 1, to: 0 }]);
  const random = seeded(7);
  const pick = n => Math.floor(random() * n);
  for (let round = 0; round < 2000; round++) {
    const universe = Array.from({ length: 12 }, (_, i) => 'k' + i);
    const sample = () => {
      const keys = universe.filter(() => random() < 0.6);
      for (let i = keys.length - 1; i > 0; i--) { const j = pick(i + 1); [keys[i], keys[j]] = [keys[j], keys[i]]; }
      return keys;
    };
    check(sample(), sample());
  }
});

test('group keys follow thread keys', () => {
  const { groupThreads, providerGroups } = sandbox.module.exports;
  const view = project({ interval: 5, hosts: [host({ agents: [{ id: 'a', status: 'working', project: 'A' }, { id: 'b', status: 'idle', project: 'B' }] })],
    allowances: [allowanceRow(), allowanceRow({ account_id: 'Work', label: 'Work' })] }, now);
  assert.deepEqual(plain(groupThreads(view, [], []).map(g => g.keys)), [['laptop:a', 'laptop:b']]);
  assert.deepEqual(plain(groupThreads(view, ['idle'], []).map(g => g.keys)), [['laptop:a']]);
  assert.deepEqual(plain(groupThreads(view, [], ['laptop']).map(g => g.keys)), [[]]);
  assert.deepEqual(plain(providerGroups(view.allowances).map(g => g.keys)), [['codex:Personal', 'codex:Work']]);
});

// ---------------------------------------------------------------- A2 readings
const timeOracle = require('./fixtures/popover-time-oracle.json');
const diagnosticsOracle = require('./fixtures/popover-diagnostics-oracle.json');
function withoutAges(view) {
  const copy = plain(view);
  copy.hosts.forEach(h => delete h.age);
  copy.threads.forEach(t => delete t.age);
  return copy;
}

test('readView of every oracle projection equals the baseline view without host and thread ages', () => {
  const { readView } = sandbox.module.exports;
  let checked = 0;
  for (const c of timeOracle.cases)
    for (const { nowMs, view } of c.projections) {
      assert.deepEqual(plain(readView(project(c.raw, nowMs), nowMs)), withoutAges(view), c.name + ' @ ' + nowMs);
      checked++;
    }
  assert.equal(checked, 112);
  assert.deepEqual(plain(readView(project(null, now), now)), withoutAges(project(null, now)));
});

test('diagnostics equal every recorded IPC string', () => {
  const { diagnostics } = sandbox.module.exports;
  assert.equal(diagnosticsOracle.cases.length, 13);
  for (const c of diagnosticsOracle.cases) assert.equal(diagnostics(project(c.raw, c.nowMs), c.nowMs), c.diagnostics, c.name + ' @ ' + c.nowMs);
});

test('the structural view carries source times and no time-derived presentation', () => {
  const agent = { id: 'a', status: 'working', technical: {
    telemetry: { seq: (now - 1000) * 1000, total_input: 10, total_output: 1, subagent_total: 1, subagent_done: 0, subagent_status_seq: (now - 1000) * 1000,
      subagent_running: 1, subagent_interrupted: 0, subagent_failed: 0, subagent_unknown: 0, subagent_starts: 1, subagent_stops: 0, subagent_seq: (now - 1000) * 1000 },
    turn_timing: { observed_at_s: now / 1000, active: true, started_at_s: now / 1000 - 10, freshness_seconds: 12, complete: true, total_finished_duration_s: 5 } } };
  const view = project({ interval: 5, hosts: [host({ agents: [agent] })], allowances: [allowanceRow()] }, now);
  const keys = value => Object.keys(value);
  assert.deepEqual(keys(view), ['connected', 'working', 'partial', 'threads', 'discoveryLabel', 'hosts', 'allowances', 'note']);
  assert.deepEqual(keys(view.hosts[0]), ['connectionState', 'connectionLabel', 'id', 'name', 'navigation', 'reporting']);
  const thread = view.threads[0];
  assert.deepEqual(keys(thread), ['id', 'hostId', 'project', 'navigation', 'title', 'host', 'branch', 'checkout', 'generation', 'statusGeneration',
    'state', 'harness', 'usage', 'children', 'completion', 'timing']);
  assert.deepEqual(keys(thread.usage), ['contextPercent', 'inputTokens', 'outputTokens', 'uncachedTokens', 'cachePercent', 'compactions', 'stale', 'at']);
  assert.deepEqual(keys(thread.children), ['starts', 'stops', 'stamp', 'stale', 'at']);
  assert.deepEqual(keys(thread.completion), ['total', 'done', 'stamp', 'stale', 'at', 'outcomes']);
  assert.deepEqual(keys(thread.timing), ['active', 'stale', 'complete', 'outcome', 'observedAt', 'startedAt', 'last', 'finishedTotal', 'settled']);
  assert.deepEqual(keys(view.allowances[0]), ['id', 'provider', 'providerLabel', 'label', 'statusText', 'remaining', 'resetCount', 'resetAt', 'durationS', 'sampledAt']);
  assert.equal(thread.usage.at, now - 1000);
  assert.equal(thread.children.at, now - 1000);
  assert.equal(thread.completion.at, now - 1000);
  assert.equal(thread.timing.startedAt, now / 1000 - 10);
  assert.equal(thread.timing.finishedTotal, 5);
  assert.equal(view.allowances[0].resetAt, now / 1000 + 302400);
  assert.equal(view.allowances[0].durationS, 604800);
  assert.equal(view.allowances[0].sampledAt, now / 1000 - 5);
});

test('an unknown current turn reports no elapsed time even with a last duration', () => {
  const { turnReading } = sandbox.module.exports;
  const agent = { id: 'a', status: 'idle', technical: { turn_timing: { observed_at_s: now / 1000, active: null, started_at_s: now / 1000 - 10,
    last_duration_s: 42, complete: true, total_finished_duration_s: 100, freshness_seconds: 12 } } };
  const timing = project({ interval: 5, hosts: [host({ agents: [agent] })], allowances: [] }, now).threads[0].timing;
  // Baseline (53f2407): active false, elapsed null, last 42, total 100.
  assert.equal(timing.settled, false);
  const reading = turnReading(timing, now + 5000);
  assert.equal(reading.active, false);
  assert.equal(reading.elapsed, null);
  assert.equal(reading.last, 42);
  assert.equal(reading.total, 100);
  const settled = { ...agent, technical: { turn_timing: { ...agent.technical.turn_timing, active: false } } };
  assert.equal(turnReading(project({ interval: 5, hosts: [host({ agents: [settled] })], allowances: [] }, now).threads[0].timing, now).elapsed, 42);
});

test('a pacing window without a used percentage keeps its reset countdown and expected balance', () => {
  const { allowanceReading } = sandbox.module.exports;
  const account = projectRaw(remainingRow(null, now / 1000 + 7200, {}, { duration_s: 10000 }));
  // Baseline (53f2407): remaining null, timeRemaining 72, reset '0d 2h', no pace.
  const reading = allowanceReading(account, now);
  assert.equal(reading.remaining, null);
  assert.equal(reading.timeRemaining, 72);
  assert.equal(reading.paceDifference, null);
  assert.equal(reading.reset, '0d 2h');
  assert.equal(reading.age, 'source unavailable');
  assert.equal(account.sampledAt, null);
});

test('readings advance with the display instant while the structure stays fixed', () => {
  const { turnReading, allowanceReading, usageReading } = sandbox.module.exports;
  const agent = { id: 'a', status: 'working', technical: {
    telemetry: { seq: (now - 30000) * 1000, usage_seq: (now - 30000) * 1000, total_input: 10, total_output: 1 },
    turn_timing: { observed_at_s: now / 1000, active: true, started_at_s: now / 1000 - 754, freshness_seconds: 12, complete: true, total_finished_duration_s: 100 } } };
  const view = project({ interval: 5, hosts: [host({ agents: [agent] })], allowances: [allowanceRow()] }, now);
  const thread = view.threads[0];
  assert.equal(turnReading(thread.timing, now).elapsed, 754);
  assert.equal(turnReading(thread.timing, now + 5000).elapsed, 759);
  assert.equal(turnReading(thread.timing, now + 5000).total, 859);
  assert.equal(usageReading(thread.usage, now).age, '30s ago');
  assert.equal(usageReading(thread.usage, now + 31000).age, '1m ago');
  const account = view.allowances[0];
  assert.equal(allowanceReading(account, now).reset, '3d 12h');
  assert.equal(allowanceReading(account, now + 1000).reset, '3d 11h');
  assert.ok(allowanceReading(account, now + 60000).timeRemaining < allowanceReading(account, now).timeRemaining);
  // A stale turn stays frozen at its observation.
  const stale = { ...thread.timing, stale: true };
  assert.equal(turnReading(stale, now + 60000).elapsed, 754);
});

// ---------------------------------------------------------------- A3 store
test('the view signature is constant from any oracle instant until its deadline', () => {
  const { viewSignature, nextDeadlineMs } = sandbox.module.exports;
  const random = seeded(11);
  for (const c of timeOracle.cases)
    for (const { nowMs } of c.projections) {
      const signature = viewSignature(project(c.raw, nowMs));
      const deadline = nextDeadlineMs(c.raw, nowMs);
      assert.ok(deadline === null || deadline > nowMs, c.name);
      const end = deadline === null ? nowMs + 86400000 : deadline;
      const points = [nowMs, end - 1];
      for (let i = 0; i < 8; i++) points.push(nowMs + Math.floor(random() * (end - nowMs)));
      for (const t of points) assert.equal(viewSignature(project(c.raw, t)), signature, `${c.name} @ ${nowMs}: changed at ${t} before ${deadline}`);
    }
  assert.equal(nextDeadlineMs(null, now), null);
});

test('the deadline is never later than the first structural change near every oracle threshold', () => {
  const { viewSignature, nextDeadlineMs } = sandbox.module.exports;
  let scanned = 0, changes = 0;
  for (const c of timeOracle.cases) {
    const centres = [...new Set(c.projections.map(p => p.nowMs))];
    const windows = [];
    for (const x of centres) {
      const last = windows[windows.length - 1];
      if (last && x - 2000 <= last[1]) last[1] = x + 2000; else windows.push([x - 2000, x + 2000]);
    }
    for (const [from, to] of windows) {
      const signatures = [];
      for (let t = from; t <= to; t++) signatures.push(viewSignature(project(c.raw, t)));
      // nextChange[i]: the first ms after from + i whose signature differs from the one before it.
      const nextChange = new Array(signatures.length).fill(Infinity);
      for (let i = signatures.length - 2; i >= 0; i--) nextChange[i] = signatures[i + 1] !== signatures[i] ? from + i + 1 : nextChange[i + 1];
      for (let i = 0; i < signatures.length; i++) {
        if (nextChange[i] === Infinity) continue;
        const deadline = nextDeadlineMs(c.raw, from + i);
        assert.ok(deadline !== null && deadline <= nextChange[i], `${c.name}: deadline ${deadline} after change at ${nextChange[i]} (from ${from + i})`);
        scanned++;
      }
      for (let i = 1; i < signatures.length; i++) if (signatures[i] !== signatures[i - 1]) changes++;
    }
  }
  // 39 structural changes, checked from 85010 start instants.
  assert.ok(changes >= 39, 'the scan crosses the oracle thresholds: ' + changes);
  assert.ok(scanned >= 85000, 'start instants checked: ' + scanned);
});

test('storeStep replaces the view on structural change only, with the harness object shape', () => {
  const { storeStep, viewSignature, receiptTimeoutMs } = sandbox.module.exports;
  const raw = { interval: 5, heartbeat_seconds: 4, hosts: [host({ sampled_at: now / 1000 - 1,
    agents: [{ id: 'a', status: 'working', technical: { turn_timing: { observed_at_s: now / 1000 - 1, active: true, started_at_s: now / 1000 - 60, freshness_seconds: 12 } } }] })],
    allowances: [], fleet_discovery: { state: 'disabled' } };
  const store = { raw: null, lastReceipt: 0, view: project(null, now), signature: '' };
  // A receipt replaces the view and records the receipt time.
  assert.equal(storeStep(store, { type: 'receipt', raw }, now), true);
  assert.equal(store.lastReceipt, now);
  assert.equal(store.signature, viewSignature(store.view));
  assert.equal(store.deadline, now + 11000 - 1, 'turn staleness is the earliest boundary (one ms early)');
  const view = store.view;
  // A tick before the deadline does nothing; an unchanged re-sent snapshot keeps the view.
  assert.equal(storeStep(store, { type: 'tick' }, now + 1000), false);
  assert.equal(storeStep(store, { type: 'receipt', raw: JSON.parse(JSON.stringify(raw)) }, now + 4000), false);
  assert.equal(store.view, view);
  assert.equal(store.lastReceipt, now + 4000);
  assert.equal(storeStep(store, { type: 'receipt', raw }, now + 8000), false);
  // A tick at the (early) deadline that changes nothing keeps the view but moves the deadline on.
  const early = store.deadline;
  assert.equal(storeStep(store, { type: 'tick' }, early), false);
  assert.equal(store.view, view);
  assert.ok(store.deadline > early);
  // The turn goes stale just after observed + 12 s: the next due tick replaces the view.
  assert.equal(storeStep(store, { type: 'tick' }, now + 12000), true);
  assert.equal(store.view.threads[0].timing.stale, true);
  // Without a deadline a tick returns false.
  const quiet = { raw: null, lastReceipt: 0, view: project(null, now), signature: '' };
  assert.equal(storeStep(quiet, { type: 'tick' }, now), false);
  assert.equal(storeStep(quiet, { type: 'update' }, now), true, 'the first update sets the empty view signature');
  assert.equal(quiet.deadline, null);
  assert.equal(storeStep(quiet, { type: 'tick' }, now + 1000), false);
  // A receipt timeout drops raw.
  const timeout = receiptTimeoutMs(raw);
  assert.equal(storeStep(store, { type: 'tick' }, now + 8000 + timeout), false, 'not yet beyond the timeout');
  assert.notEqual(store.raw, null);
  assert.equal(storeStep(store, { type: 'tick' }, now + 8000 + timeout + 1), true);
  assert.equal(store.raw, null);
  assert.equal(store.view.connected, false);
  // A malformed receipt nulls raw without touching lastReceipt.
  storeStep(store, { type: 'receipt', raw }, now + 20000);
  assert.equal(storeStep(store, { type: 'receipt', raw: null }, now + 21000), true);
  assert.equal(store.raw, null);
  assert.equal(store.lastReceipt, now + 20000);
  assert.equal(store.view.connected, false);
  // Update re-projects the raw the caller set (restart, refresh, collector exit).
  store.raw = raw;
  assert.equal(storeStep(store, { type: 'update' }, now + 22000), true);
  assert.equal(store.view.connected, true);
  // A collector exit: the caller nulls raw, then an update empties the view and clears the deadline.
  const receipt = store.lastReceipt;
  store.raw = null;
  assert.equal(storeStep(store, { type: 'update' }, now + 23000), true);
  assert.equal(store.view.connected, false);
  assert.equal(store.view.threads.length, 0);
  assert.equal(store.deadline, null);
  assert.equal(store.lastReceipt, receipt);
});

test('stable row order survives store updates', () => {
  const { storeStep } = sandbox.module.exports;
  const agents = order => order.map(id => ({ id, status: id === 'b' ? 'working' : 'idle', project: id }));
  const raw = order => ({ interval: 5, hosts: [host({ sampled_at: now / 1000, agents: agents(order) })], allowances: [] });
  const store = { raw: null, lastReceipt: 0, view: project(null, now), signature: '' };
  storeStep(store, { type: 'receipt', raw: raw(['a', 'c']) }, now);
  storeStep(store, { type: 'receipt', raw: raw(['a', 'b', 'c']) }, now + 1000);
  assert.deepEqual(Array.from(store.view.threads, t => t.id), ['a', 'c', 'b']);
});

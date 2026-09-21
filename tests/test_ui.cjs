const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
function harness() {
  const elements = new Map();
  const get = id => { if (!elements.has(id)) elements.set(id, {value:'all', innerHTML:'', style:{}, addEventListener(){}}); return elements.get(id); };
  const context = vm.createContext({document:{getElementById:get,documentElement:{style:{setProperty(){}}}},Date,performance:{now:()=>0},AbortSignal,fetch:()=>new Promise(()=>{}),setTimeout(){},setInterval(){}});
  vm.runInContext(fs.readFileSync('web/app.js','utf8'),context);
  return {get,run:code=>vm.runInContext(code,context)};
}
test('UI escapes titles and expires working agents on disconnect', () => {
 const h=harness();
 h.run(`state={profile:'personal',interval:5,theme:{name:'Test',colours:{}},history:[],hosts:[{id:'desktop',label:'Desktop',online:true,sampled_at:Date.now()/1000,version:'0.9.1',trend:[],metrics:null,agents:[{id:'a',host:'desktop',project:'<img src=x onerror=alert(1)>',title:'<script>evil()</script>',category:'personal',harness:'codex',status:'working',since:1}]}]};received=Date.now();render();`);
 assert.equal(h.get('working').textContent,1);
 assert.ok(h.get('agents').innerHTML.includes('&lt;script&gt;'));
 assert.ok(!h.get('agents').innerHTML.includes('<script>'));
 h.run('failed=true;render();');
 assert.equal(h.get('working').textContent,0);
 assert.ok(h.get('connection').textContent.includes('STALE'));
 assert.ok(!h.get('network').innerHTML.includes('machine-node active'));
});
test('UI expires an old host even while HTTP remains available',()=>{
 const h=harness();
 h.run(`state={profile:'work',interval:5,theme:{name:'Test',colours:{}},history:[],hosts:[{id:'desktop',label:'Desktop',online:true,sampled_at:1,version:'0.9.1',trend:[],metrics:null,agents:[{status:'working'}]}]};received=Date.now();render();`);
 assert.equal(h.get('working').textContent,0);
 assert.equal(h.get('category').hidden,true);
});
test('Unknown activity breaks the trace rather than plotting zero',()=>{
 const h=harness();
 const svg = h.run('sparkSegments([{working:2},{working:3},{working:null},{working:1},{working:2}])');
 assert.equal((svg.match(/<polyline/g)||[]).length,2);
 assert.ok(!svg.includes('50,28'));
});
test('Wall display bounds each page while totals cover all agents',()=>{
 const h=harness();
 h.run(`state={profile:'personal',interval:5,theme:{name:'Test',colours:{}},history:Array.from({length:50},(_,i)=>({at:1,host:'desktop',project:'Event '+i,status:'working'})),hosts:Array.from({length:8},(_,i)=>({id:'host'+i,label:'Host '+i,online:true,sampled_at:Date.now()/1000,version:'0.9.1',trend:[],metrics:null,agents:Array.from({length:4},(_,j)=>({id:i+':'+j,host:'host'+i,project:'Project '+i+':'+j,title:'Task',category:'work',harness:'codex',status:'working',since:1}))}))};received=Date.now();render();`);
 assert.equal(h.get('working').textContent,32);
 assert.equal((h.get('agents').innerHTML.match(/class="agent-card"/g)||[]).length,6);
 assert.equal((h.get('network').innerHTML.match(/class="machine-node/g)||[]).length,3);
 assert.equal((h.get('timeline').innerHTML.match(/class="event /g)||[]).length,6);
 const first=h.get('agents').innerHTML;
 h.run('page=1;render();');
 assert.notEqual(h.get('agents').innerHTML,first);
 assert.ok(h.get('page-counter').textContent.includes('2/6'));
});
test('Layout retains fixed viewport and bounded two-row execution cells',()=>{
 const css=fs.readFileSync('web/style.css','utf8');
 assert.ok(css.includes('html,body{width:100%;height:100%;overflow:hidden}'));
 assert.ok(css.includes('grid-template-rows:repeat(2,minmax(0,1fr))'));
});

test('Card FX resume elapsed phase across renders and stop with working status',()=>{
 const h=harness();
 h.run(`state={profile:'work',interval:5,theme:{name:'Test',colours:{}},history:[],hosts:[{id:'desktop',label:'Desktop',online:true,sampled_at:Date.now()/1000,version:'test',trend:[],metrics:null,agents:[{id:'a',host:'desktop',project:'Project',title:'Task',category:'work',harness:'codex',status:'working',since:1}]}]};received=Date.now();`);
 for(const elapsed of [0,1000,2000,2999,3000,4100]) {
  h.run(`performance.now=()=>${elapsed};render();`);
  assert.ok(h.get('network').innerHTML.includes(h.run("activityPhase('machine:desktop')")));
  assert.ok(h.get('agents').innerHTML.includes(h.run("activityPhase('thread:desktop:a')")));
  assert.ok(h.get('agents').innerHTML.includes('class="card-fx" aria-hidden="true"'));
 }
 for(const status of ['idle','blocked','done','unknown']) {
  h.run(`state.hosts[0].agents[0].status='${status}';render();`);
  assert.ok(!h.get('agents').innerHTML.includes('card-fx'));
 }
 h.run("state.hosts[0].agents[0].status='working';failed=true;render();");
 assert.ok(!h.get('agents').innerHTML.includes('card-fx'));
 assert.ok(!h.get('network').innerHTML.includes('machine-node active'));
 const css=fs.readFileSync('web/style.css','utf8');
 assert.ok(css.includes('@keyframes circuit{to{stroke-dashoffset:-100}}'));
 assert.ok(css.includes('animation-delay:var(--sweep-delay,0ms)'));
 assert.ok(css.includes('animation:none!important'));
});

test('Entity rhythms differ and remain stable across render order',()=>{
 const h=harness();
 h.run('performance.now=()=>1200');
 const ids=['machine:desktop','machine:laptop','thread:desktop:a','thread:desktop:b','thread:laptop:a'];
 const phases=ids.map(id=>h.run(`activityPhase('${id}')`));
 assert.equal(new Set(phases).size,ids.length);
 ids.slice().reverse().forEach(id=>assert.equal(h.run(`activityPhase('${id}')`),phases[ids.indexOf(id)]));
 const parse=s=>Object.fromEntries([...s.matchAll(/--([a-z-]+):(-?[0-9.]+)ms/g)].map(m=>[m[1],Number(m[2])]));
 const before=parse(phases[0]);
 h.run('performance.now=()=>2200');
 const after=parse(h.run("activityPhase('machine:desktop')"));
 assert.equal(after['sweep-duration'],before['sweep-duration']);
 assert.equal(-after['sweep-delay'],(-before['sweep-delay']+1000)%before['sweep-duration']);
});
test('Technical strip shows genuine counters and explicit missing flags',()=>{
 const h=harness();
 const strip=h.run("technicalStrip({technical:{revision:42,state_change_seq:17,focused:true}})");
 assert.ok(strip.includes('REV <b>42</b>'));
 assert.ok(strip.includes('SEQ <b>17</b>'));
 assert.ok(strip.includes('FOCUS'));
 assert.ok(strip.includes('>—</span>'));
 assert.ok(!h.run("technicalStrip({technical:{revision:'<script>secret</script>'}})").includes('<script>'));
});

test('Snapshot console is bounded, escaped and uses source timestamps',()=>{
 const h=harness();
 h.run(`state={interval:5};received=Date.now();`);
 const result=h.run(`snapshotConsole([{id:'host',label:'<script>private</script>',sampled_at:123,protocol:22,online:true,agents:[{status:'working'}]}],Array.from({length:30},(_,i)=>({at:100+i,host:'node',project:'<img onerror=x>',status:'working',technical:{revision:4}})))`);
 assert.ok(!result.includes('<script>'));
 assert.ok(result.includes('&lt;script&gt;'));
 assert.ok(result.includes('status=unavailable'));
 assert.equal((result.match(/class="console-record"/g)||[]).length,9);
 assert.ok(result.includes(new Date(123000).toLocaleTimeString('en-GB')));
});

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
function harness() {
  const elements = new Map();
  const get = id => { if (!elements.has(id)) elements.set(id, {value:'all', innerHTML:'', style:{}, addEventListener(){}}); return elements.get(id); };
  const context = vm.createContext({document:{getElementById:get,documentElement:{style:{setProperty(){}}}},Date,AbortSignal,fetch:()=>new Promise(()=>{}),setTimeout(){},setInterval(){}});
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

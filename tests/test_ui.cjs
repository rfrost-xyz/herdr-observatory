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
test('Viewport row budget leaves two readable agent rows at target sizes',()=>{
 for(const height of [720,1080]) {
  const clamp=(min,value,max)=>Math.max(min,Math.min(value,max));
  const header=clamp(44,.06*height,64),padding=clamp(8,.013*height,16),gap=clamp(6,.009*height,12);
  const fixed=clamp(40,.06*height,64)+clamp(54,.08*height,84)+clamp(86,.15*height,160)+clamp(108,.18*height,184)+16;
  const workspace=height-header-2*padding-5*gap-fixed;
  assert.ok(workspace>=250);
 }
 const css=fs.readFileSync('web/style.css','utf8');
 assert.ok(css.includes('html,body{width:100%;height:100%;overflow:hidden}'));
 assert.ok(css.includes('grid-template-rows:repeat(2,minmax(0,1fr))'));
});

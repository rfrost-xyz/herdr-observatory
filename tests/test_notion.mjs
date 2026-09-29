import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, rmSync, symlinkSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join, resolve} from 'node:path';
import {spawn, spawnSync} from 'node:child_process';
import {binding, verifyIdentity, observation, refresh, request} from '../omarchy/notion-extension/source.mjs';
const config={version:1,user_id:'11111111-1111-1111-1111-111111111111',workspace_id:'22222222-2222-2222-2222-222222222222'};
const now=1800000000;
const usage=()=>({status:'within_limit',billingPeriodWindow:{creditType:'basic_ai_credits',scope:'per_user',cadence:'billing_period',used:10,limit:200,periodEndMs:(now+86400)*1000}});
const spaces=()=>({[config.user_id]:{notion_user:{[config.user_id]:{value:{value:{id:config.user_id,email:'private@example.invalid'}}}},space:{[config.workspace_id]:{value:{value:{id:config.workspace_id,name:'Private workspace'}}}}}});
test('source binds exact user and workspace and forwards only monthly fields',()=>{
  verifyIdentity(spaces(),config);
  assert.throws(()=>verifyIdentity({},config));
  assert.throws(()=>binding({...config,user_id:'other'}));
  const out=observation(usage(),config,now);
  assert.equal(out.used,10); assert.equal(out.limit,200);
  assert.equal(JSON.stringify(out).includes('private'),false);
  for(const [k,v] of [['used',null],['limit',0],['used',-1],['scope','workspace'],['periodEndMs',now*1000],['periodEndMs',(now+33*86400)*1000]]){
    const raw=usage();raw.billingPeriodWindow[k]=v;assert.throws(()=>observation(raw,config,now),k);
  }
  const raw=usage();raw.billingPeriodWindow.used=0;assert.equal(observation(raw,config,now).used,0);
  raw.billingPeriodWindow.used=300;assert.equal(observation(raw,config,now).used,300);
  raw.status='not_applicable';assert.throws(()=>observation(raw,config,now));
});
test('refresh uses fixed credential-owning browser transport and fails closed',async()=>{
  let messages=[],calls=[];
  const native=async value=>{messages.push(value);return value.operation?config:{ok:true};};
  const fetcher=async(url,options)=>{calls.push([url,options]);return new Response(JSON.stringify(url.endsWith('getSpaces')?spaces():usage()));};
  assert.equal(await refresh(native,fetcher,()=>now),true);
  assert.equal(calls.length,2);assert.equal(calls[0][1].credentials,'include');assert.equal(calls[0][1].redirect,'error');
  assert.equal(calls[1][1].headers['x-notion-active-user-header'],config.user_id);
  assert.deepEqual(JSON.parse(calls[1][1].body),{spaceId:config.workspace_id});
  assert.equal(await refresh(native,async()=>new Response('',{status:401}),()=>now),false);
  assert.equal(messages.at(-1).used,null);assert.equal(messages.at(-1).available,false);
  await assert.rejects(()=>request('other',{},config.user_id,fetcher));
  await assert.rejects(()=>request('getSpaces',{},config.user_id,async()=>new Response(' '.repeat(1024*1024+1))));
});
const binary=resolve('omarchy/anton-runtime/target/debug/anton-runtime');
async function fixture(fn){
  const base=mkdtempSync(join(tmpdir(),'anton-notion-'));
  const root=join(base,'plugin'),state=join(base,'state'),home=join(base,'config');
  for(const path of [root,state,home,join(home,'chromium')])mkdirSync(path,{mode:0o700});
  const privateConfig={hosts:[{id:'test'}],notion:{user_id:config.user_id,workspace_id:config.workspace_id,extension_id:'a'.repeat(32),label:'Work'}};
  writeFileSync(join(root,'.config.json'),JSON.stringify(privateConfig),{mode:0o600});
  writeFileSync(join(root,'.herdr-observatory-install'),'herdr.observatory\n',{mode:0o600});
  const run=(args,input)=>spawnSync(binary,['--root',root,'--state',state,...args],{input,env:{...process.env,XDG_CONFIG_HOME:home},timeout:6000});
  try{await fn({base,root,state,home,run});}finally{rmSync(base,{recursive:true,force:true});}
}
function frame(value){const data=Buffer.from(JSON.stringify(value));const size=Buffer.alloc(4);size.writeUInt32LE(data.length);return Buffer.concat([size,data]);}
const origin='chrome-extension://'+'a'.repeat(32)+'/';
test('native messaging accepts only configured origin and bounded fresh observations',()=>fixture(({state,run,root})=>{
  let result=run(['--notion-bridge',origin],frame({version:1,operation:'configuration'}));
  assert.equal(result.status,0,result.stderr.toString());assert.deepEqual(JSON.parse(result.stdout.subarray(4)),config);
  const time=Date.now()/1000,raw=usage();raw.billingPeriodWindow.periodEndMs=Math.floor((time+86400)*1000);
  const row=observation(raw,config,time);
  result=run(['--notion-bridge',origin],frame(row));assert.equal(result.status,0,result.stderr.toString());
  const saved=readFileSync(join(state,'notion.json'),'utf8');assert.equal(saved.includes(config.user_id),false);assert.equal(JSON.parse(saved).used_percent,5);
  assert.notEqual(run(['--notion-bridge',origin],frame(row)).status,0);
  assert.notEqual(run(['--notion-bridge','chrome-extension://'+'b'.repeat(32)+'/'],frame(row)).status,0);
  const huge=Buffer.alloc(4);huge.writeUInt32LE(4097);assert.notEqual(run(['--notion-bridge',origin],huge).status,0);
  assert.notEqual(run(['--notion-bridge',origin],Buffer.from([1,2])).status,0);
  writeFileSync(join(root,'.herdr-observatory-install'),'herdr.observatory:retired');
  assert.notEqual(run(['--notion-bridge',origin],frame({version:1,operation:'configuration'})).status,0);
}));
test('bridge registration is repeatable, preserves conflicts and removes only its receipt',()=>fixture(({home,root,run})=>{
  const manifest=join(home,'chromium/NativeMessagingHosts/world.herdr.notion_allowance.json');
  assert.equal(run(['--register-notion-bridge']).status,0);
  assert.equal(run(['--register-notion-bridge']).status,0);
  const saved=readFileSync(manifest);writeFileSync(manifest,'{}');
  assert.notEqual(run(['--unregister-notion-bridge']).status,0);assert.equal(existsSync(join(root,'.notion-bridge.json')),true);
  writeFileSync(manifest,saved);assert.equal(run(['--unregister-notion-bridge']).status,0);assert.equal(existsSync(manifest),false);
  assert.equal(run(['--unregister-notion-bridge']).status,0);
  symlinkSync(join(root,'.config.json'),manifest);assert.notEqual(run(['--register-notion-bridge']).status,0);
}));

test('partial native input expires even while the browser keeps stdin open',()=>fixture(async({root,state,home})=>{
  const child=spawn(binary,['--root',root,'--state',state,'--notion-bridge',origin],{env:{...process.env,XDG_CONFIG_HOME:home}});
  let errors='';child.stderr.on('data',value=>errors+=value);
  const started=Date.now();
  const timeout=setTimeout(()=>child.kill('SIGKILL'),5500);
  try {
    child.stdin.write(Buffer.from([1,2]));
    const [code,signal]=await new Promise((resolve,reject)=>{child.once('exit',(...result)=>resolve(result));child.once('error',reject);});
    assert.equal(signal,null);assert.notEqual(code,0);assert.match(errors,/deadline/);
    assert.ok(Date.now()-started<5000);
  } finally {clearTimeout(timeout);child.stdin.destroy();}
}));

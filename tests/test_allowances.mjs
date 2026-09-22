import test from 'node:test';
import assert from 'node:assert/strict';
import {allowanceView, relativeTime, createAllowancePanel} from '../web/allowances.mjs';
const now=1_800_000_000_000;
const sample={label:'Personal',plan:'pro',available:true,sampled_at:now/1000-60,weekly_remaining:26,weekly_resets_at:now/1000+90000,reset_count:1,reset_expires_at:now/1000+180000};
test('weekly and reset passes retain their separate meaning and relative times',()=>{const v=allowanceView(sample,{now});assert.equal(v.weekly,'26%');assert.equal(v.reset,'in 1d 1h');assert.equal(v.passes,'1');assert.equal(v.expiry,'in 2d 2h');assert.equal(v.status,'Checked 1m ago');});
test('missing and zero are distinct; unavailable source cannot look live',()=>{assert.equal(allowanceView({...sample,weekly_remaining:0,reset_count:0},{now}).weekly,'0%');assert.equal(allowanceView({...sample,reset_count:0},{now}).expiry,'None');for(const s of [{...sample,sampled_at:now/1000-601},{...sample,sampled_at:now/1000+1},{...sample,available:false}]){const v=allowanceView(s,{now});assert.equal(v.weekly,'—');assert.equal(v.passes,'—');}assert.equal(allowanceView(sample,{now,disconnected:true}).status,'Disconnected');});
test('scheduled reset passing never fabricates a refilled weekly allowance',()=>{assert.equal(allowanceView({...sample,weekly_resets_at:now/1000-1},{now}).weekly,'—');assert.equal(relativeTime(null,now),'Unknown');assert.equal(relativeTime(now/1000-600,now),'10m ago');});
test('invalid values cannot populate numeric instruments',()=>{const v=allowanceView({...sample,weekly_remaining:101,reset_count:-1,reset_expires_at:null},{now});assert.equal(v.weekly,'—');assert.equal(v.passes,'—');assert.equal(v.expiry,'Unknown');});
test('two persistent account panels update only when visible values change',()=>{let renders=0;const make=()=>({children:[],style:{},append(...xs){this.children.push(...xs);},setAttribute(){}});const root={ownerDocument:{createElement:make},replaceChildren(...xs){this.children=xs;renders++;}};const panel=createAllowancePanel({root});panel.update([sample],{now});assert.equal(root.children.length,2);panel.update([sample],{now});assert.equal(renders,1);panel.update([sample],{now:now+60000});assert.equal(renders,2);});

test('elapsed reset-pass expiry invalidates the old count without inventing zero',()=>{const v=allowanceView({...sample,reset_expires_at:now/1000-60},{now});assert.equal(v.passes,'—');assert.equal(v.expiry,'1m ago');});

test('native business plan names are readable without losing their tier',()=>{assert.equal(allowanceView({...sample,plan:'self_serve_business_prolite'},{now}).plan,'Business Pro Lite');});

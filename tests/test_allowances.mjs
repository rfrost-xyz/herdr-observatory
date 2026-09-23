import test from 'node:test';
import assert from 'node:assert/strict';
import {allowanceView, relativeTime, createAllowancePanel} from '../web/allowances.mjs';

const now=1_800_000_000_000;
const sample={label:'Personal',plan:'pro',available:true,sampled_at:now/1000-60,
  weekly_remaining:26,weekly_resets_at:now/1000+90000,reset_count:1,reset_expires_at:now/1000+180000};
const nodes=root=>{const walk=node=>[node,...node.children.flatMap(walk)];return walk(root);};
function fixture(){
  let renders=0;
  const make=tag=>({tag,children:[],style:{},attrs:{},textContent:'',append(...xs){this.children.push(...xs);},setAttribute(key,value){this.attrs[key]=value;}});
  const root={ownerDocument:{createElement:make},replaceChildren(...xs){this.children=xs;renders++;}};
  return {root,panel:createAllowancePanel({root}),renders:()=>renders};
}

test('weekly balance, reset and passes are distinct facts',()=>{
  const v=allowanceView(sample,{now});
  assert.equal(v.weekly,'26%');assert.equal(v.reset,'in 1d 1h');assert.equal(v.passes,'1');
  assert.equal(v.expiry,'in 2d 2h');assert.equal(v.status,'Checked 1m ago');
  assert.equal(relativeTime(now/1000-600,now),'10m ago');
});

test('weekly pace compares remaining allowance with time remaining on one scale',()=>{
  const aligned=allowanceView({...sample,weekly_remaining:73,weekly_resets_at:now/1000+5.1*86400},{now});
  assert.equal(aligned.pace,'on');assert.equal(aligned.paceLabel,'On pace');assert.equal(aligned.displayDifference,0);
  assert.ok(Math.abs(aligned.timeRemaining-5.1/7*100)<1e-9);
  const deficit=allowanceView({...sample,weekly_remaining:10,weekly_resets_at:now/1000+3.25*86400},{now});
  assert.equal(deficit.pace,'deficit');assert.equal(deficit.paceLabel,'36 percentage points behind even pace');
  const reserve=allowanceView(sample,{now});assert.equal(reserve.pace,'reserve');
  assert.equal(reserve.paceLabel,'11 percentage points ahead of even pace');
  assert.ok(reserve.paceDifference>0);assert.ok(deficit.paceDifference<0);
  const slight=allowanceView({...sample,weekly_remaining:46,weekly_resets_at:now/1000+3.29*86400},{now});
  assert.equal(slight.displayDifference,-1);assert.equal(slight.paceLabel,'1 percentage point behind even pace');
  const nearZero=allowanceView({...sample,weekly_remaining:46.8,weekly_resets_at:now/1000+3.29*86400},{now});
  assert.equal(nearZero.displayDifference,0);assert.equal(nearZero.paceLabel,'On pace');
  assert.equal('runway' in deficit,false);assert.equal('burnPerDay' in deficit,false);
});

test('missing, expired and inconsistent windows never imply a pace or refill',()=>{
  for(const changed of [{...sample,weekly_resets_at:null},{...sample,weekly_resets_at:now/1000+8*86400}]){
    const v=allowanceView(changed,{now});assert.equal(v.weekly,'26%');assert.equal(v.pace,'unknown');assert.equal(v.timeRemaining,null);
  }
  for(const changed of [{...sample,sampled_at:now/1000-601},{...sample,sampled_at:now/1000+1},
    {...sample,available:false},{...sample,weekly_resets_at:now/1000-1}]){
    const v=allowanceView(changed,{now});assert.equal(v.weekly,'—');assert.equal(v.pace,'unknown');
  }
  assert.equal(allowanceView({...sample,weekly_resets_at:now/1000-1},{now}).passes,'1');
  assert.equal(allowanceView({...sample,sampled_at:now/1000-601},{now}).passes,'—');
  assert.equal(allowanceView(sample,{now,disconnected:true}).status,'Disconnected');
  assert.equal(allowanceView({...sample,weekly_remaining:0,reset_count:0},{now}).weekly,'0%');
  assert.equal(allowanceView({...sample,reset_count:0},{now}).expiry,'None');
  assert.equal(allowanceView({...sample,reset_expires_at:now/1000-60},{now}).expiry,'1m ago');
});

test('daily token chart uses only reported dates and never defines quota pace',()=>{
  const daily_usage=[{date:'2026-09-20',tokens:0},{date:'2026-09-22',tokens:9000}];
  const v=allowanceView({...sample,daily_usage},{now});
  assert.equal(v.activity.count,2);assert.equal(v.activity.total,'9K');assert.equal(v.activity.exact,'9,000');
  assert.deepEqual(v.activity.daily.map(row=>row.date),['2026-09-20','2026-09-22']);
  assert.equal(v.activity.daily[0].height,0);assert.equal(v.activity.daily[1].height,100);
  assert.equal(v.pace,allowanceView(sample,{now}).pace);
  for(const invalid of [[{date:'bad',tokens:1}],[{date:'2026-09-22',tokens:-1}],
    [{date:'2026-09-22',tokens:1},{date:'2026-09-22',tokens:2}]]){
    assert.equal(allowanceView({...sample,daily_usage:invalid},{now}).activity,null);
  }
  assert.equal(allowanceView({...sample,sampled_at:now/1000-601,daily_usage},{now}).activity,null);
});

test('fallback cards render pace marker, observed activity and honest pass age',()=>{
  const {root,panel,renders}=fixture();const daily_usage=[{date:'2026-09-20',tokens:0},{date:'2026-09-22',tokens:9000}];
  panel.update([{...sample,weekly_remaining:10,weekly_resets_at:now/1000+3.25*86400,daily_usage}],{now});
  const card=root.children[0],all=nodes(card),gauge=all.find(node=>node.className==='allowance-gauge');
  assert.equal(root.children.length,2);assert.equal(card.attrs['data-pace'],'deficit');
  const variance=all.find(node=>node.className==='allowance-variance');
  assert.equal(variance.attrs['data-direction'],'behind');assert.ok(variance.textContent.startsWith('-'));
  assert.equal(all.some(node=>node.textContent==='weekly left'),false);
  assert.equal(gauge.children[0].style.width,'10%');assert.equal(gauge.children[1].style.left,'10%');assert.ok(gauge.children[1].style.width.startsWith('36.'));assert.ok(gauge.children[2].style.left.startsWith('46.'));
  assert.equal(all.some(node=>node.className==='allowance-pace-label'),false);
  assert.ok(card.attrs['aria-label'].includes('36 percentage points behind even pace'));
  assert.ok(all.some(node=>node.className==='activity-summary'&&node.textContent==='9K tokens · 2 reported days'));
  assert.equal(all.filter(node=>node.tag==='i'&&node.style.height!==undefined).length,2);
  assert.ok(card.attrs['aria-label'].includes('9,000 tokens across 2 reported dates'));
  assert.equal(all.some(node=>node.className==='allowance-outlook'||node.className==='pace-row'),false);
  panel.update([{...sample,weekly_remaining:10,weekly_resets_at:now/1000+3.25*86400,daily_usage}],{now});
  assert.equal(renders(),1);
  panel.update([{...sample,reset_expires_at:now/1000-60}],{now});
  assert.ok(root.children[0].attrs['aria-label'].includes('pass expiry 1m ago'));
  panel.update([{...sample,reset_count:0}],{now});
  assert.ok(root.children[0].attrs['aria-label'].includes('pass expiry None'));
});

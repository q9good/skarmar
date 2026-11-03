import assert from 'node:assert/strict';
import {test} from 'node:test';
import {palettes,neutral} from '../apps/client/src/ui-palette.ts';
import {planRelatedKnowledge} from '../apps/client/src/plan-related-knowledge.ts';
import {emptySnapshot,newGoal} from '../apps/client/src/foundation.ts';

const luminance=hex=>{const rgb=hex.slice(1).match(/../g).map(n=>parseInt(n,16)/255).map(n=>n<=.04045?n/12.92:((n+.055)/1.055)**2.4);return rgb[0]*.2126+rgb[1]*.7152+rgb[2]*.0722;};
const contrast=(a,b)=>(Math.max(luminance(a),luminance(b))+.05)/(Math.min(luminance(a),luminance(b))+.05);
test('all functional themes have readable solid actions and readable secondary labels',()=>{
 for(const [name,c] of Object.entries(palettes)){
  assert(contrast(c.ink,'#FFFFFF')>=4.5,name+' primary');assert(contrast(c.hover,'#FFFFFF')>=4.5,name+' pressed');
  assert(contrast(c.ink,c.soft)>=4.5,name+' soft surface');
 }
 assert(contrast(neutral.muted,neutral.surface)>=4.5);
 assert.notEqual(palettes.family.ink,palettes.classroom.ink);assert.notEqual(palettes.goal.ink,palettes.method.ink);
});
const goal=(id,parent_id=null)=>({id,version:1,input:{...newGoal(),title:id,parent_id}});
const kb=(id,kind,goal_ids=[],method_ids=[],extra={})=>({input:{id,kind,content:id,goal_ids,method_ids},details:{state:kind==='difficulty'?'pending':'active'},...extra});
test('plan knowledge is empty until an actual goal or method is chosen',()=>{
 const data={...emptySnapshot,goals:[goal('g')],knowledge:[kb('k','experience',['g'])]};
 assert.deepEqual(planRelatedKnowledge(data,{targets:[],methods:[],focus_goal_ids:[]}),[]);
});
test('selected goals carry their own and ancestor principles, and selected methods carry details exactly once',()=>{
 const data={...emptySnapshot,goals:[goal('long'),goal('g','long'),goal('other')],methods:[{id:'m',input:{title:'chosen'}},{id:'unselected',input:{title:'not chosen'}}],knowledge:[
  kb('parent','experience',['long']),kb('goal','difficulty',['g']),kb('micro','experience',[],['m']),kb('both','experience',['g'],['m']),
  kb('unselected-only','experience',[],['unselected']),kb('unrelated','difficulty',['other']),kb('deleted','difficulty',['g'],[],{deleted:true}),
  kb('resolved','difficulty',['g'],[],{details:{state:'resolved'}}),kb('retired','experience',['g'],[],{details:{state:'retired'}}),
 ]};
 const rows=planRelatedKnowledge(data,{targets:[{goal_id:'g'}],methods:[{id:'m',version:1}],focus_goal_ids:[]});
 assert.deepEqual(rows.map(r=>r.knowledge.input.id).sort(),['both','goal','micro','parent']);
 const both=rows.find(r=>r.knowledge.input.id==='both');assert.equal(both.goals.length,1);assert.equal(both.methods.length,1);
 assert.equal(planRelatedKnowledge(data,{targets:[{goal_id:'g'}],methods:[],focus_goal_ids:[]}).some(r=>r.knowledge.input.id==='micro'),false);
});
test('explicit long focus is allowed, malformed parent cycles terminate and duplicate snapshots do not duplicate cards',()=>{
 const k=kb('one','experience',['long']);
 const data={...emptySnapshot,goals:[goal('long','child'),goal('child','long')],knowledge:[k,k]};
 const rows=planRelatedKnowledge(data,{targets:[],methods:[],focus_goal_ids:['long']});assert.equal(rows.length,1);
});

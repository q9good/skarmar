import assert from 'node:assert/strict';
import {test} from 'node:test';
import {emptySnapshot,newGoal,newMethod} from '../apps/client/src/foundation.ts';
import {goalReferenceKnowledge,referencesForGoals,referencePathLabel} from '../apps/client/src/plan-related-knowledge.ts';
import {activityPlanReferences} from '../apps/client/src/activity-plan-references.ts';
import {goalOverviewRows,goalStatusGroups} from '../apps/client/src/goal-overview.ts';
import {emptySession,startSummary} from '../apps/client/src/prd-sessions.ts';
const goal=(id,extra={})=>({id,version:1,input:{...newGoal(),title:id,...extra}});
const method=(id,goals,extra={})=>({id,version:1,input:{...newMethod(),title:id,goal_ids:goals,...extra}});
const kb=(id,kind,goals=[],methods=[],details={})=>({input:{id,kind,content:id,goal_ids:goals,method_ids:methods},details:{state:kind==='experience'?'active':'pending',...details}});
const row=(id,ratio=0,extra={})=>({id,ratio,last_date:null,attention_anchor:'2026-10-01',counts:{family:0,lesson:0,selected:0},focus:{family:0,lesson:0,selected:0},...extra});
const report=rows=>({source:'family',days:30,start:'2026-10-06',end:'2026-11-04',goals:rows});
test('reference knowledge traverses only direct goal, active methods and direct ancestors, deduplicating all paths',()=>{
 const g=goal('g',{parent_id:'mid'}),data={...emptySnapshot,goals:[goal('long',{level:'long'}),goal('mid',{level:'medium',parent_id:'long'}),g,goal('other')],methods:[method('m',['g','other']),method('retired',['g'],{state:'retired'})],knowledge:[
  kb('both','experience',['g'],['m']),kb('micro','experience',[],['m']),kb('macro','experience',['long']),kb('support','difficulty',['g']),
  kb('other-only','experience',['other']),kb('retired-only','experience',[],['retired']),kb('solved','difficulty',['g'],[],{state:'resolved'}),kb('paused','difficulty',['g'],[],{state:'paused'}),{...kb('gone','experience',['g']),deleted:true},
 ]};
 const rows=goalReferenceKnowledge(data,'g');assert.deepEqual(rows.map(r=>r.knowledge.input.id).sort(),['both','macro','micro','support']);
 assert.deepEqual(rows.find(r=>r.knowledge.input.id==='both').paths.map(p=>p.kind),['direct','method']);
 assert(referencePathLabel(data,rows.find(r=>r.knowledge.input.id==='macro').paths[0]).includes('上级方向「long」'));
 assert(referencePathLabel(data,rows.find(r=>r.knowledge.input.id==='micro').paths[0]).includes('经方法「m」'));
 const merged=referencesForGoals(data,['g','other','g']);assert.equal(merged.filter(r=>r.knowledge.input.id==='micro').length,1);assert.equal(merged.find(r=>r.knowledge.input.id==='both').paths.filter(p=>p.kind==='method').length,2);
});
test('malformed parent cycles stop and archived parent principles are not pushed',()=>{
 const data={...emptySnapshot,goals:[goal('a',{parent_id:'b'}),goal('b',{parent_id:'a',archived:true})],knowledge:[kb('own','experience',['a']),kb('old','experience',['b'])]};
 assert.deepEqual(goalReferenceKnowledge(data,'a').map(r=>r.knowledge.input.id),['own']);
});
test('read-only reminders respect family long priorities, explicit support and cadence while excluding inactive/no-rhythm goals',()=>{
 const data={...emptySnapshot,goals:[goal('low',{level:'long',long_priority:0}),goal('high',{level:'long',long_priority:2}),goal('normal',{level:'long'}),goal('support'),goal('overdue'),goal('later'),goal('fourth'),goal('paused',{state:'paused'}),goal('achieved',{state:'achieved'}),goal('not-started',{state:'not_started'}),goal('none',{frequency:'none'}),goal('archived',{archived:true})],knowledge:[kb('needs-support','difficulty',['support'],[],{priority:true})]};
 const r=report([row('support',0),row('overdue',4),row('later',2),row('fourth',1),row('paused',99),row('none',99)]),before=structuredClone(data);
 const rows=activityPlanReferences(data,r);assert.deepEqual(rows.map(r=>r.goal.id),['high','normal','support','overdue','later']);assert(rows[2].reason.includes('重点困难'));assert.deepEqual(data,before,'reference generation must never mutate snapshot or select anything');
});
test('unread coverage uses a state-based fallback without inventing zero counts or a last date',()=>{
 const data={...emptySnapshot,goals:[goal('one')],knowledge:[]};const result=activityPlanReferences(data,null);assert.equal(result.length,1);assert(!result[0].reason.includes('0次'));assert.deepEqual(goalOverviewRows(data.goals,null),[{goal:data.goals[0],counts:null}]);
});
test('old plan targets do not become performed automatically; a fresh activity plan yields an empty actual summary',()=>{
 const input={...emptySession('2026-11-04'),plan:'跟随孩子玩他喜欢的活动',targets:[{goal_id:'old',role:'main',note:'原笔记'}],methods:[{id:'m',version:1}]};
 const before=structuredClone(input),summary=startSummary(input);assert.equal(summary.targets[0].performed,false);assert.deepEqual(summary.methods,[]);assert.equal(summary.confirmation,'unconfirmed');assert.deepEqual(input,before);
 assert.deepEqual(startSummary(emptySession('2026-11-04')).targets,[]);
});
test('all goal states are counted once; archived inclusion is controlled by the caller scope',()=>{
 const goals=[goal('active'),goal('legacy',{state:null}),goal('paused',{state:'paused'}),goal('achieved',{state:'achieved'}),goal('maintaining',{state:'maintaining'}),goal('new',{state:'not_started'})];
 const stats=goalStatusGroups(goals);assert.equal(stats.reduce((n,s)=>n+s.goals.length,0),goals.length);assert.equal(stats.find(s=>s.id==='active').goals.length,2);assert.equal(stats.find(s=>s.id==='achieved').goals.length,1);
});
test('coverage ring preserves explicit long focus, real zeros, unknown rows and separate family/lesson counts',()=>{
 const goals=[goal('long',{level:'long'}),goal('short'),goal('unread')],r=report([row('long',0,{focus:{family:1,lesson:2,selected:1},counts:{family:9,lesson:9,selected:9}}),row('short')]);
 const rows=goalOverviewRows(goals,r);assert.equal(rows[0].counts.selected,1);assert.equal(rows[0].counts.lesson,2);assert.equal(rows[1].counts.selected,0);assert.equal(rows[2].counts,null);
});

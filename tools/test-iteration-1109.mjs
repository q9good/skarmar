import assert from 'node:assert/strict';
import {test} from 'node:test';
import {emptySnapshot,newGoal,newMethod,mergeSnapshot} from '../apps/client/src/foundation.ts';
import {priorityValue,todayFocus,twoWeekPriority} from '../apps/client/src/object-priority.ts';
import {observationItems,withObservationItems} from '../apps/client/src/observation-items.ts';
const goal=(id,score=50,parent=null,level='long')=>({id,version:1,input:{...newGoal(),title:id,level,parent_id:parent,priority_score:score}});
const method=(id,ids,score=50)=>({id,version:1,input:{...newMethod(),title:id,goal_ids:ids,priority_score:score}});
test('numeric priority counts unique active relations, obeys per-goal bonus/cap and leaves base unchanged',()=>{
 const a=goal('a'),b=goal('b'),done=goal('done');done.input.state='achieved';const m=method('m',['a','b','b','done']);
 const data={...emptySnapshot,goals:[a,b,done],methods:[m],priority_policy:{version:1,minimum:10,maximum:200,method_per_goal:7,method_bonus_cap:10}};
 assert.deepEqual(priorityValue(data,m,'method','2026-11-09'),{base:50,bonus:10,score:60,temporary:false,count:2,until:undefined});assert.equal(m.input.priority_score,50);
 assert.equal(priorityValue({...data,priority_policy:{...data.priority_policy,maximum:40}},m,'method','2026-11-09').base,40);
});
test('Today combines actual long goal/method relations, temporary priority expires after 14 business days and ties use creation',()=>{
 const a=goal('a',99),b=goal('b',1),m=method('new method',['a'],1),unrelated=method('unrelated',['missing'],200);b.input.priority_until=twoWeekPriority('2026-11-09');
 const data={...emptySnapshot,goals:[a,b],methods:[m,unrelated],created_at:{a:'2026-01-01',b:'2026-11-09',m:'2026-11-09'}};
 assert.equal(b.input.priority_until,'2026-11-22');assert.equal(todayFocus(data,'2026-11-22')[0].value.id,'b');assert.equal(todayFocus(data,'2026-11-23')[0].value.id,'a');assert(!todayFocus(data,'2026-11-09').some(r=>r.value.id==='unrelated'));
 m.input.priority_score=99;const tied={...data,priority_policy:{version:1,minimum:0,maximum:100,method_per_goal:0,method_bonus_cap:0},created_at:{...data.created_at,[m.id]:'2026-11-09T10:00:00+08:00',a:'2026-11-08T22:00:00Z'}};assert.equal(todayFocus(tied,'2026-11-23')[0].kind,'method');
});
test('legacy paragraph remains one item even with newlines; structured items preserve identity and category order',()=>{
 const old={teacher_notes:'one\ntwo',notes:''};assert.equal(observationItems(old).length,1);
 const items=[{id:'a',kind:'teacher_notes',content:'first'},{id:'b',kind:'teacher_notes',content:'second'},{id:'c',kind:'interest_notes',content:'third'}],next=withObservationItems(old,items);
 assert.equal(next.teacher_notes,'first\nsecond');assert.deepEqual(observationItems(next),items);assert.equal(old.teacher_notes,'one\ntwo');
});
test('partial accepted snapshots preserve policy and original creation timestamps',()=>{
 const original={...emptySnapshot,priority_policy:{version:2,minimum:0,maximum:200,method_per_goal:3,method_bonus_cap:30},created_at:{a:'2026-01-01'}};
 const result=mergeSnapshot(original,{...emptySnapshot,goals:[goal('a')]});assert.deepEqual(result.priority_policy,original.priority_policy);assert.deepEqual(result.created_at,original.created_at);
});

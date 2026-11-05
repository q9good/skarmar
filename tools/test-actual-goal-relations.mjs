import assert from 'node:assert/strict';
import {test} from 'node:test';
import {newGoal} from '../apps/client/src/foundation.ts';
import {visibleGoals,actualGoalRelations} from '../apps/client/src/actual-goal-relations.ts';
import {goalOverviewRows} from '../apps/client/src/goal-overview.ts';
const goal=(id,level='long',parent_id=null,archived=false)=>({id,version:1,input:{...newGoal(),title:id,level,parent_id,archived}});
const goals=[goal('long'),goal('mid','medium','long'),goal('short','short','mid'),goal('sibling','short','mid'),goal('archived','short',null,true)];
test('Goals and picker share the saved catalogue, including all horizons and no invented example choices',()=>{
 const before=structuredClone(goals);assert.deepEqual(visibleGoals(goals).map(g=>g.id),['long','mid','short','sibling']);assert.deepEqual(goals,before);
 assert.equal(visibleGoals(goals,true).length,5);assert(visibleGoals(goals,false,['archived']).some(g=>g.id==='archived'));
});
test('selecting only a long goal is explicit and never selects its children',()=>{
 const r=actualGoalRelations(goals,['long']);assert.deepEqual([...r.explicit],['long']);assert.equal(r.inherited.size,0);assert.deepEqual([...r.associated],['long']);
});
test('a short goal associates its full parent chain without creating separate observations',()=>{
 const r=actualGoalRelations(goals,['short']);assert.deepEqual([...r.explicit],['short']);assert.deepEqual([...r.inherited.keys()],['mid','long']);assert.deepEqual(r.inherited.get('long'),['short']);
});
test('siblings and explicit ancestors deduplicate; removing one leaf retains the other leaf parent associations',()=>{
 const r=actualGoalRelations(goals,['short','sibling','long','short']);assert.equal(r.explicit.size,3);assert.equal(r.associated.size,4);assert.deepEqual(r.inherited.get('long'),['short','sibling']);
 assert.equal(actualGoalRelations(goals,['sibling']).associated.size,3);assert.equal(actualGoalRelations(goals,[]).associated.size,0);
});
test('unknown, missing, cyclic and wrong-level parent links never invent unrelated associations',()=>{
 assert.deepEqual([...actualGoalRelations(goals,['unknown']).associated],['unknown']);
 const malformed=[goal('a','short','b'),goal('b','short','a'),goal('other')];assert.equal(actualGoalRelations(malformed,['a']).inherited.size,0);
 assert.equal(actualGoalRelations([goal('orphan','short','missing')],['orphan']).inherited.size,0);
});
test('overview uses the server union rather than summing overlapping direct, descendant and legacy focus counts',()=>{
 const g=goals[0],report={goals:[{id:g.id,linked:{family:2,lesson:1,selected:3},focus:{family:1,lesson:1,selected:2},counts:{family:2,lesson:1,selected:3},descendants:{family:2,lesson:1,selected:3}}]};
 assert.deepEqual(goalOverviewRows([g],report)[0].counts,{family:2,lesson:1,selected:3});assert.equal(goalOverviewRows([g],null)[0].counts,null);
});

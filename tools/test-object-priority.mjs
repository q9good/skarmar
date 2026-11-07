import assert from 'node:assert/strict';
import {test} from 'node:test';
import {newGoal,emptySnapshot} from '../apps/client/src/foundation.ts';
import {goalAttention,methodAttention,prioritizedLongDirections,dueReviews} from '../apps/client/src/object-priority.ts';
const goal=(id,priority=1,level='long')=>({id,version:1,input:{...newGoal(),title:id,level,long_priority:priority}});
const schedule=(id,date,extra={})=>({object_kind:'goal',object_id:id,reminders:true,eligible:true,mode:'regular',due_date:date,...extra});
const difficulty=id=>({input:{id:'k-'+id,kind:'difficulty',goal_ids:[id],method_ids:[],content:'合成困难'},details:{state:'following',priority:true},deleted:false});
test('manual high/normal/low remains the base and same-level difficulties and due reviews adjust attention',()=>{
 const high=goal('high',2),normal=goal('normal'),due=goal('due'),critical=goal('critical'),low=goal('low',0),paused=goal('paused',2);paused.input.state='paused';
 const data={...emptySnapshot,goals:[low,paused,normal,due,critical,high],knowledge:[difficulty('critical'),difficulty('low')]},s=[schedule('due','2026-11-07')];
 assert.deepEqual(prioritizedLongDirections(data,'2026-11-07',s).map(g=>g.id),['high','critical','due','normal','low']);
 assert.equal(goalAttention(data,low,'2026-11-07',s).level,'low');assert.equal(goalAttention(data,critical,'2026-11-07',s).level,'watch');
 assert.deepEqual(prioritizedLongDirections({...data,goals:[...data.goals].reverse()},'2026-11-07',s).map(g=>g.id),['high','critical','due','normal','low']);
 assert.equal(goalAttention(data,goal('short',2,'short'),'2026-11-07').level,'high');
});
test('a directly selected long goal counts as recent family evidence; plans and unknown anchors do not invent recency',()=>{
 const g=goal('long');g.input.frequency='weekly';g.input.attention_start='2020-01-01';
 const session={id:'s',status:'completed',created_at:'2026-11-07T01:00:00Z',input:{business_date:'2026-11-07'},actual_targets:[{goal:g,ancestors:[]}],actual_methods:[],focus_goals:[]};
 const data={...emptySnapshot,goals:[g],sessions:[session]};assert(!goalAttention(data,g,'2026-11-07').reasons.some(r=>r.startsWith('超过')));
 assert(goalAttention({...data,sessions:[{...session,status:'planned'}]},g,'2026-11-07').reasons.some(r=>r.startsWith('超过')));
 const fresh=goal('fresh');assert(!goalAttention(emptySnapshot,fresh,'2026-11-07').reasons.some(r=>r.startsWith('超过')));
});
test('method priority keeps legacy pin intent, gives high choices emphasis and leaves inactive methods behind',()=>{
 const g=goal('g'),data={...emptySnapshot,goals:[g]},m={id:'m',version:1,input:{title:'合成方法',goal_ids:['g'],state:'active',pinned:true}};
 const legacy=methodAttention(data,m,'2026-11-07'),high=methodAttention(data,{...m,input:{...m.input,pinned:false,priority:2}},'2026-11-07');assert.equal(legacy.rank,high.rank);assert.equal(legacy.level,'high');
 assert(methodAttention(data,{...m,input:{...m.input,state:'paused'}},'2026-11-07').rank>high.rank);
 assert.equal(methodAttention(data,{...m,input:{...m.input,pinned:false,priority:0}},'2026-11-07').level,'low');
});
test('due reminders appear on the business day, persist overdue and honor paused/disabled/future conditions',()=>{
 const rows=[schedule('future','2026-11-08'),schedule('today','2026-11-07'),schedule('overdue','2026-11-06'),schedule('disabled','2026-11-07',{reminders:false}),schedule('paused','2026-11-07',{eligible:false}),schedule('resume','2026-11-07',{eligible:false,mode:'resume'}),schedule('unknown',null)];
 assert.deepEqual(dueReviews(rows,'2026-11-06').map(s=>s.object_id),['overdue']);
 assert.deepEqual(dueReviews(rows,'2026-11-07').map(s=>s.object_id),['overdue','resume','today']);
 assert.deepEqual(dueReviews(rows,'2026-11-08').map(s=>s.object_id),['overdue','resume','today','future']);
 assert.equal(rows[0].due_date,'2026-11-08');
});

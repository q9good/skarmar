import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
export async function sessionContract(label,send,admin){
 async function call(method,path,body,status=200){const r=await send('/api'+path,{method,headers:{'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1',Origin:'https://prd-contract.local',Cookie:admin},body:body===undefined?undefined:JSON.stringify(body)});const data=await r.json();assert.equal(r.status,status,`${label} ${path}: ${JSON.stringify(data)}`);return data;}
 let state=await call('GET','/prd/state');const g=state.goals[0],m=state.methods[0],lesson=state.lessons.find(l=>l.status==='recorded'),action=state.actions.find(a=>a.status==='pending');
 const input={business_date:'2020-01-01',time_slot:'晚上',plan:'测试共享计划',targets:[{goal_id:g.id,role:'main',note:'计划笔记'}],methods:[{id:m.id,version:m.version}],focus_goal_ids:[],tags:['游戏'],context:'',lesson_sources:[{id:lesson.id,version:lesson.version}],action_ids:[action.input.id],summary:null};
 const id=randomUUID(),cmd=(id,version,action,input=null)=>({id,operation_id:randomUUID(),expected_version:version,action,input,change_note:'测试修订'});
 const plan=cmd(id,null,'save_draft',input);
 const beforeBlank=await call('GET','/prd/state');
 await call('PUT','/prd/sessions/'+id,{...plan,input:{...input,plan:' \t\n'}},422);
 assert.deepEqual(await call('GET','/prd/state'),beforeBlank,'blank plan rejection must not reserve an action or create a record');
 // The failed operation is not reserved: corrected content can use its identity.
 await call('PUT','/prd/sessions/'+id,plan);await call('PUT','/prd/sessions/'+id,plan);
 assert.equal((await call('GET','/prd/coverage?days=30&source=family')).records.selected,0);
 const competing=randomUUID();await call('PUT','/prd/sessions/'+competing,cmd(competing,null,'save_draft',input),409);
 const summary={targets:[{goal_id:g.id,performed:true,observation:'测试：实际进行了练习'}],confirmation:'unconfirmed',methods:[],tags:['游戏','游戏'],context:'',signals:[{kind:'主动开始',goal_ids:[]}],new_knowledge:[],new_actions:[],action_outcomes:[{action_id:action.input.id,decision:'continue'}]};
 await call('PUT','/prd/sessions/'+id,cmd(id,1,'complete',{...input,summary}),422);
 const kid=randomUUID();summary.confirmation='unknown';summary.new_knowledge=[{id:kid,kind:'difficulty',content:'测试干预困难',goal_ids:[g.id],method_ids:[],stage:null,next_review_date:null}];
 const complete=cmd(id,1,'complete',{...input,summary});let saved=await call('PUT','/prd/sessions/'+id,complete);await call('PUT','/prd/sessions/'+id,complete);assert.equal(saved.status,'completed');assert.equal(saved.actual_methods.length,0);assert.equal(saved.planned_methods.length,1);assert.deepEqual(saved.input.summary.tags,['游戏']);
 state=await call('GET','/prd/state');assert.equal(state.knowledge.filter(k=>k.source_session_id===id).length,1);assert.equal(state.actions.find(a=>a.input.id===action.input.id).status,'pending');
 const next={...saved.input,summary:{...saved.input.summary,knowledge_versions:{[kid]:1},new_knowledge:[{...saved.input.summary.new_knowledge[0],content:'测试干预困难修订'}]}};
 saved=await call('PUT','/prd/sessions/'+id,cmd(id,2,'complete',next));state=await call('GET','/prd/state');assert.equal(state.knowledge.find(k=>k.input.id===kid).version,2);
 await call('PUT','/prd/sessions/'+id,cmd(id,2,'complete',next),409);assert.equal((await call('GET','/prd/sessions/'+id+'/history')).length,3);
 const future=randomUUID();await call('PUT','/prd/sessions/'+future,cmd(future,null,'complete',{...next,business_date:'2099-01-01',action_ids:[],summary:{...summary,new_knowledge:[],action_outcomes:[]}}),422);
 await call('PUT','/prd/sessions/'+id,cmd(id,3,'delete'));state=await call('GET','/prd/state');assert.equal(state.sessions.find(s=>s.id===id).status,'deleted');assert.equal(state.knowledge.find(k=>k.input.id===kid).source_session_id,id);
 const resumed=randomUUID();await call('PUT','/prd/sessions/'+resumed,cmd(resumed,null,'save_draft',{...input,lesson_sources:[]}));await call('PUT','/prd/sessions/'+resumed,cmd(resumed,1,'cancel'));assert.equal((await call('GET','/prd/state')).actions.find(a=>a.input.id===action.input.id).status,'pending');
 // Activity-only plans acquire actual targets later; those must not become planned snapshots.
 const activityId=randomUUID(),activity={...input,targets:[],methods:[],action_ids:[],lesson_sources:[]};
 const activityPlan=await call('PUT','/prd/sessions/'+activityId,cmd(activityId,null,'save_draft',activity));
 assert.deepEqual(activityPlan.planned_targets,[]);assert.deepEqual(activityPlan.planned_methods,[]);
 const actual={...activity,targets:input.targets,summary:{...summary,confirmation:'used',methods:input.methods,new_knowledge:[],action_outcomes:[]}};
 const activityComplete=cmd(activityId,1,'complete',actual),actualSaved=await call('PUT','/prd/sessions/'+activityId,activityComplete);
 assert.equal(actualSaved.actual_targets.length,1);assert.equal(actualSaved.actual_methods.length,1);
 assert.deepEqual(actualSaved.planned_targets,[]);assert.deepEqual(actualSaved.planned_methods,[]);
 assert.deepEqual(await call('PUT','/prd/sessions/'+activityId,activityComplete),actualSaved);
 // Delete only the synthetic addition, keeping the following coverage contracts' original scope.
 await call('PUT','/prd/sessions/'+activityId,cmd(activityId,2,'delete'));
 console.log(`PASS ${label}: activity-only plan and explicit actual targets/methods never backfill planned snapshots; exact receipt retained`);
 console.log(`PASS ${label}: required plan and atomic rejection, shared plan/summary/completion, explicit actual methods, atomic revised outputs, exclusive actions, future guard, history and source-preserving deletion`);
}

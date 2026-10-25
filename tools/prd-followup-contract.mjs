import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
export async function followupContract(label,send,admin){
 async function call(method,path,body,status=200,cookie=admin){const r=await send('/api'+path,{method,headers:{'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1',Origin:'https://prd-contract.local',Cookie:cookie},body:body===undefined?undefined:JSON.stringify(body)});const data=await r.json();assert.equal(r.status,status,`${label} ${path}: ${JSON.stringify(data)}`);return {data,cookie:r.headers.get('set-cookie')?.split(';')[0]};}
 const process=async(kind,value,status=200,cookie=admin)=>(await call('POST','/prd/process',{kind,value},status,cookie)).data;
 const read=async(view,id='',kind='')=>(await call('GET',`/prd/process?view=${view}&id=${id}&object_kind=${kind}`)).data;
 const state=(await call('GET','/prd/state')).data,g=state.goals[0],m=state.methods[0];
 const create=async(kind)=>{const id=randomUUID();return process('knowledge',{id,operation_id:randomUUID(),expected_version:null,action:'create',input:{id,kind,content:'测试明确做法与观察',goal_ids:[g.id],method_ids:[m.id],stage:kind==='experience'?'try':null},details:{title:`测试${kind}`,basis:'每次5次机会',support:'示范一次',comparison_start:'2020-01-01'}})};
 const k=await create('experience'),kid=k.input.id,eid=randomUUID();
 const command={id:eid,operation_id:randomUUID(),expected_version:null,action:'record',input:{knowledge_id:kid,source:{kind:'independent',id:eid,version:1},business_date:'2020-01-01',result:'helpful',observation:'测试观察到有帮助'}};
 await process('feedback',command);await process('feedback',command);
 const member=(await call('POST','/auth/login',{username:'lesson-family',password:'s'})).cookie;
 await process('feedback',{id:eid,operation_id:randomUUID(),expected_version:1,action:'annotate',note:'测试另一位家人的相反观察'},200,member);
 assert.equal((await read('knowledge',kid)).summary.total,1);assert.equal((await read('events',kid)).annotations.length,1);
 await process('feedback',{...command,id:randomUUID(),operation_id:randomUUID()},409);
 const revised=await process('feedback',{...command,operation_id:randomUUID(),expected_version:1,action:'revise',note:'测试更正主结果',input:{...command.input,result:'uncertain'}},200,member);assert.equal(revised.version,2);assert.equal((await read('knowledge',kid)).summary.counts.uncertain,1);
 await process('feedback',{id:eid,operation_id:randomUUID(),expected_version:2,action:'delete'},403,member);
 const difficulty=await create('difficulty');for(let i=0;i<3;i++){const id=randomUUID();await process('feedback',{id,operation_id:randomUUID(),expected_version:null,action:'record',input:{knowledge_id:difficulty.input.id,source:{kind:'independent',id,version:1},business_date:'2020-01-01',result:'same',observation:'测试同条件下仍需支持',comparable:true,basis:'每次5次机会',support:'示范一次'}});}
 const proposed=await read('knowledge',difficulty.input.id);assert.equal(proposed.summary.offer_weekly,true);assert.equal(proposed.knowledge.details.priority,false);
 const c=await read('review_context',g.id,'goal'),rid=randomUUID();
 const review={id:rid,operation_id:randomUUID(),expected_version:null,object_kind:'goal',object_id:g.id,object_version:c.object_version,schedule_version:c.schedule.version,generation:c.schedule.generation,due_date:c.schedule.due_date,action:'complete',decision:'maintain',note:'测试已有依据，转为维持',next_actions:[{id:randomUUID(),content:'测试复盘下一步',goal_ids:[g.id],assignee_id:null}]};
 const receipt=await process('review',review);await process('review',review);assert(receipt.schedule.due_date>receipt.review.completed_on);
 await process('review',{...review,id:randomUUID(),operation_id:randomUUID()},409);
 const memberIdentity=(await call('GET','/auth/me',undefined,200,member)).data.user;
 const taskId=receipt.review.action_ids[0];let task=(await call('GET','/prd/state')).data.actions.find(a=>a.input.id===taskId);
 task=await process('action',{id:taskId,operation_id:randomUUID(),expected_version:task.version,input:{...task.input,assignee_id:memberIdentity.id},state:'pending',note:'测试家人指派'});
 await process('action',{id:taskId,operation_id:randomUUID(),expected_version:task.version,input:task.input,state:'cancelled',note:'测试家人明确取消'},200,member);
 assert.equal((await read('action',taskId)).history.length,3);
 const sid=randomUUID(),fid=randomUUID(),session={id:sid,operation_id:randomUUID(),expected_version:null,action:'complete',input:{business_date:'2020-01-01',time_slot:'晚上',summary:{targets:[{goal_id:g.id,performed:true,observation:'测试独立来源的实际观察'}],confirmation:'none',feedback:[{id:fid,knowledge_id:kid,result:'helpful'}]}}};
 await call('PUT','/prd/sessions/'+sid,session);await call('PUT','/prd/sessions/'+sid,session);assert.equal((await read('knowledge',kid)).summary.total,2);
 const now=await read('review_context',g.id,'goal');await process('review',{...review,id:randomUUID(),operation_id:randomUUID(),object_version:now.object_version,schedule_version:now.schedule.version,generation:now.schedule.generation,due_date:now.schedule.due_date,action:'defer',next_date:'2099-01-01',next_actions:[]});
 assert.equal((await read('reviews')).reviews.filter(r=>r.object_id===g.id).length,1);
 const cfg=(await read('board')).settings;await process('settings',{operation_id:randomUUID(),expected_version:cfg.version,weekday:cfg.weekday,fortnight_anchor:cfg.fortnight_anchor,monthly_ordinal:-1,reorder_overdue:false});assert.equal((await read('board')).schedules.find(s=>s.object_id===g.id).due_date,'2099-01-01');
 const empty1=await create('difficulty'),empty2=await create('difficulty');const cs=await Promise.all([empty1,empty2].map(k=>read('review_context',k.input.id,'knowledge')));
 const group={id:randomUUID(),operation_id:randomUUID(),expected_version:null,object_kind:'knowledge',object_id:empty1.input.id,object_version:cs[0].object_version,schedule_version:cs[0].schedule.version,generation:cs[0].schedule.generation,due_date:cs[0].schedule.due_date,action:'defer_group',next_date:'2099-01-01',group_tokens:cs.map(c=>({object_kind:c.schedule.object_kind,object_id:c.schedule.object_id,object_version:c.object_version,schedule_version:c.schedule.version,generation:c.schedule.generation,due_date:c.schedule.due_date}))};
 assert.equal((await process('review',group)).deferred_group,2);await process('review',group);assert.equal((await read('reviews')).reviews.filter(r=>r.object_id===g.id).length,1);
 console.log(`PASS ${label}: knowledge CRUD, source-unique feedback, annotations, explicit revision, comparison gate, atomic review/retry, calendar settings, action assignments/history, embedded feedback and atomic group deferral`);
}

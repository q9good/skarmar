// Real SQLite/DO HTTP operations on a disposable synthetic space only.
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
export async function workflowUpdatesContract(label,send,cookie){
 const origin='https://prd-contract.local';
 async function call(method,path,body,status=200){const response=await send('/api'+path,{method,headers:{Origin:origin,Cookie:cookie,'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'},body:body===undefined?undefined:JSON.stringify(body)});const data=await response.json();assert.equal(response.status,status,`${label} workflow ${method} ${path}: ${JSON.stringify(data)}`);return data;}
 async function put(kind,value){const id=randomUUID();return(await call('PUT','/prd/records/'+id,{id,operation_id:randomUUID(),expected_version:null,change_note:'',mutation:{kind,value},...(kind==='method'?{method_create_mode:'full'}:{})})).value;}
 const goal=await put('goal',{title:'合成循环目标',level:'short',long_priority:2}),method=await put('method',{title:'合成循环方法',steps:'合成先示范再等待',goal_ids:[goal.id],priority:2});assert.equal(method.input.priority,2);assert.equal(goal.input.long_priority,2);
 const course=await put('catalog',{kind:'course',name:'合成循环课程',active:true}),teacher=await put('catalog',{kind:'teacher',name:'合成循环老师',active:true});
 const id=randomUUID(),base={business_date:'2020-01-01',course_id:course.id,teacher_id:teacher.id},request=(expected_version,text)=>({id,operation_id:randomUUID(),expected_version,action:{kind:'observation',value:{...base,teacher_notes:text}}});
 await call('PUT','/prd/lessons/'+id,request(null,'合成最初观察'));const second=request(1,'合成中间老师建议');const receipt=await call('PUT','/prd/lessons/'+id,second);await call('PUT','/prd/lessons/'+id,request(2,'合成较新建议'));
 assert.equal((await call('GET','/prd/lessons/'+id+'/history')).length,1);
 const kid=randomUUID();await call('PUT','/prd/lessons/'+id,{id,operation_id:randomUUID(),expected_version:1,action:{kind:'organization',value:{seen_observation_version:2,new_knowledge:[{id:kid,kind:'experience',content:'合成待尝试的做法',stage:'idea',goal_ids:[goal.id]}]}}});
 const sid=randomUUID();await call('PUT','/prd/sessions/'+sid,{id:sid,operation_id:randomUUID(),expected_version:null,action:'complete',change_note:'',input:{business_date:'2020-01-01',time_slot:'上午',lesson_sources:[{id,version:2}],summary:{targets:[{goal_id:goal.id,performed:true,observation:'合成明确家庭观察'}],confirmation:'none'}}});
 await call('PUT','/prd/lessons/'+id,{id,operation_id:randomUUID(),expected_version:2,action:{kind:'organization',value:{seen_observation_version:3,request_generation:3,processing:'complete'}}});
 const final=await call('PUT','/prd/lessons/'+id,request(3,'合成最终建议'));assert.equal(final.request_generation,4);
 const state=await call('GET','/prd/state');assert.equal(state.sessions.find(s=>s.id===sid).lesson_sources[0].observation.teacher_notes,'合成中间老师建议');assert.equal(state.knowledge.find(k=>k.input.id===kid).source_observation.teacher_notes,'合成中间老师建议');assert.deepEqual(state.lessons.find(l=>l.id===id).organization.targets,[]);
 const pending=()=>call('GET','/prd/process?view=board');assert((await pending()).inbox.requests.some(r=>r.id===id));assert.equal((await pending()).inbox.requests.find(r=>r.id===id).claimed_by,null);
 const complete=(generation,seen)=>({id,operation_id:randomUUID(),expected_version:3,action:{kind:'organization',value:{seen_observation_version:seen,request_generation:generation,processing:'complete'}}});
 await call('PUT','/prd/lessons/'+id,complete(3,2),409);await call('PUT','/prd/lessons/'+id,complete(4,2),409);await call('PUT','/prd/lessons/'+id,complete(4,4));assert(!(await pending()).inbox.requests.some(r=>r.id===id));
 assert.deepEqual(await call('PUT','/prd/lessons/'+id,second),receipt);assert.equal((await call('GET','/prd/state')).lessons.find(l=>l.id===id).observation.teacher_notes,'合成最终建议');assert(!(await pending()).inbox.requests.some(r=>r.id===id));
 console.log(`PASS ${label}: classroom updates omit revision feed, frozen intermediate HTTP references and replay survive, automatic review requires latest material, proposals do not count as course coverage, priorities persist`);
}

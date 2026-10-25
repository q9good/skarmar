import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
export async function observationContract(label,send,admin){
 const headers={'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1',Origin:'https://prd-contract.local',Cookie:admin};
 async function call(body,status=200){const r=await send('/api/prd/process',{method:'POST',headers,body:JSON.stringify(body)});const data=await r.json();assert.equal(r.status,status,`${label} observations ${JSON.stringify(data)}`);return data;}
 const state=await(await send('/api/prd/state',{headers})).json(),goal=state.goals[0];
 const write=(kind,id,input,version=null)=>({kind:'observe',value:{id,operation_id:randomUUID(),expected_version:version,kind,action:'save',input,note:version?'测试修订':''}});
 const today=new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date());
 const metricId=randomUUID(),metric={goal_id:goal.id,goal_version:goal.version,name:'合成机会记录',template:'count',criterion:'5次机会',unit:'',support:'示范一次'};
 const cmd=write('metric',metricId,metric),saved=await call(cmd);assert.deepEqual(await call(cmd),saved);
 const momentId=randomUUID(),moment=write('moment',momentId,{business_date:today,text:'测试主动邀请',goal_ids:[goal.id]});await call(moment);
 const measurement={metric_id:metricId,metric_version:1,business_date:today,support:'示范一次',opportunities:0,achieved:0,source:{kind:'moment',id:momentId,version:1}};
 const measurementId=randomUUID(),actual=write('measurement',measurementId,measurement),observed=await call(actual);assert.deepEqual(await call(actual),observed);assert.equal(observed.metric.version,1);
 await call(write('measurement',randomUUID(),{...measurement,opportunities:0,achieved:1}),422);await call(write('measurement',randomUUID(),measurement),409);
 await call(write('metric',metricId,{...metric,support:'不提醒'},1));const result=await(await send('/api/prd/process?view=observations&object_kind=measurement',{headers})).json();const row=result.items.find(m=>m.id===measurementId);assert.equal(row.metric.input.support,'示范一次');
 const needs=write('needs','00000000-0000-4000-8000-000000000001',{communication:'测试用手势表示休息',sensory:'安静环境'});await call(needs);await call({...needs,value:{...needs.value,operation_id:randomUUID()}},409);
 const context=await(await send('/api/prd/process?view=review_context&object_kind=goal&id='+goal.id,{headers})).json();assert(context.evidence.some(e=>e.source.kind==='moment'));assert(context.evidence.some(e=>e.source.kind==='measurement'));
 const before=await(await send('/api/prd/coverage?days=30&source=family',{headers})).json();await call(write('moment',randomUUID(),{business_date:today,text:'测试第二个瞬间',goal_ids:[goal.id]}));const after=await(await send('/api/prd/coverage?days=30&source=family',{headers})).json();assert.deepEqual(after,before);
 console.log(`PASS ${label}: metric snapshots, validated actual values, source deduplication, shared needs, review evidence and separate moment counts`);
}

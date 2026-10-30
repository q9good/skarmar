import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';

// Real native/DO writes, revisions and concurrency; no production targets.
export async function reviewFeedbackContract(label,send,admin) {
 const headers={'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1',Origin:'https://prd-contract.local',Cookie:admin};
 const call=async(method,path,body,status=200)=>{const r=await send('/api'+path,{method,headers,body:body===undefined?undefined:JSON.stringify(body)});const value=await r.json();assert.equal(r.status,status,JSON.stringify(value));return value;};
 const id=randomUUID(),command=(version,value)=>({id,operation_id:randomUUID(),expected_version:version,change_note:'家人调整关注顺序',mutation:{kind:'goal',value}});
 const first=await call('PUT','/prd/records/'+id,command(null,{title:'合成长期关注优先级',level:'long'}));
 assert.equal(first.value.input.long_priority??null,null);
 const high=command(1,{title:'合成长期关注优先级',level:'long',long_priority:2});
 const saved=await call('PUT','/prd/records/'+id,high);assert.equal(saved.value.input.long_priority,2);
 await call('PUT','/prd/records/'+id,command(1,{title:'并发旧填写',level:'long',long_priority:0}),409);
 await call('PUT','/prd/records/'+id,command(2,{title:'非法优先级',level:'long',long_priority:3}),422);
 // A partial title update must keep the family's priority, and replay must not revert it.
 await call('PUT','/prd/records/'+id,command(2,{title:'合成已保留优先级',level:'long'}));
 assert.deepEqual(await call('PUT','/prd/records/'+id,high),saved);
 const current=(await call('GET','/prd/state')).goals.find(g=>g.id===id);
 assert.equal(current.version,3);assert.equal(current.input.long_priority,2);
 const history=await call('GET','/prd/records/'+id+'/history');assert.deepEqual(history.map(r=>r.entity.value.input.long_priority??null),[null,2,2]);
 console.log(`PASS ${label}: shared long-focus priority, partial update, invalid/stale rejection and immutable retry/history`);
}

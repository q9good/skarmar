import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
export async function outboxContract(label,send,admin){
 const headers={'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1',Origin:'https://prd-contract.local',Cookie:admin};
 const identity=await(await send('/api/auth/me',{headers})).json();
 async function queue(write,status=200,extra={},user=identity.user.id){
  const r=await send('/api/prd/queue',{method:'POST',headers:{...headers,'X-Skarma-Space':identity.space,...extra},body:JSON.stringify({user_id:user,write})});const data=await r.json();assert.equal(r.status,status,`${label} queue ${JSON.stringify(data)}`);return data;
 }
 const catalog=(id,kind)=>({kind:'catalog',value:{id,operation_id:randomUUID(),expected_version:null,change_note:'',mutation:{kind:'catalog',value:{kind,name:'测试离线'+kind,active:true}}}});
 const course=randomUUID(),teacher=randomUUID(),c=catalog(course,'course'),t=catalog(teacher,'teacher');
 await queue(c,403,{'X-Skarma-Space':'wrong-space'});await queue(c,409,{},randomUUID());await queue(c,409,{'X-Skarma-Contract':'old-contract'});
 await queue(c,401,{Cookie:''});const saved=await queue(c);assert.deepEqual(await queue(c),saved);await queue(t);
 const id=randomUUID(),day=new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date());
 const write={kind:'lesson',value:{id,operation_id:randomUUID(),expected_version:null,action:{kind:'observation',value:{business_date:day,course_id:course,teacher_id:teacher,notes:'测试断网后补传'}}}};
 const receipt=await queue(write);assert.equal(receipt.creator_id,identity.user.id);assert.deepEqual(await queue(write),receipt);
 const disable={...t.value,expected_version:1,operation_id:randomUUID(),mutation:{kind:'catalog',value:{...t.value.mutation.value,active:false}}};
 assert.equal((await send('/api/prd/records/'+teacher,{method:'PUT',headers,body:JSON.stringify(disable)})).status,200);
 assert.deepEqual(await queue(write),receipt,'already committed receipt must remain retrievable after catalog was disabled');
 await queue({...write,value:{...write.value,id:randomUUID(),operation_id:randomUUID()}},422);
 await queue({...write,value:{...write.value,expected_version:1}},422);
 await queue({...c,value:{...c.value,id:randomUUID(),mutation:{kind:'goal',value:{title:'禁止混入目标',level:'short'}}}},422);
 const revisions=await(await send('/api/prd/lessons/'+id+'/history',{headers})).json();assert.equal(revisions.length,1);
 console.log(`PASS ${label}: queued writes check actor/space/contract, catalog dependencies, inactive option rejection and exact receipt replay`);
}

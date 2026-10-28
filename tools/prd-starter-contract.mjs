import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {randomUUID} from 'node:crypto';
export async function starterContract(label,send,admin) {
 const pack=JSON.parse(readFileSync(new URL('../data/seeds/family-starter-v1.json',import.meta.url)));
 const catalog=JSON.parse(readFileSync(new URL('../data/seeds/initial-catalog.json',import.meta.url)));
 const keys=[...pack.goals,...pack.methods].map(r=>r.key).concat(catalog.catalog.map(r=>'catalog.'+r.key));
 const headers={Cookie:admin,Origin:'https://prd-contract.local','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Transfer':'skarmar.transfer.v1','Content-Type':'application/json'};
 const state=async()=>{const r=await send('/api/prd/state',{headers});assert.equal(r.status,200);return r.json();};
 async function post(kind,value,status=200){const r=await send('/api/data-transfer',{method:'POST',headers,body:JSON.stringify({kind,value})});const text=await r.text();assert.equal(r.status,status,`${label} starter ${kind}: ${text}`);return JSON.parse(text);}
 const before=await state();
 const p=await post('starter_preview',{keys});assert.equal(p.rows.length,26);assert.deepEqual(await state(),before);
 const receipt=await post('commit',{preview_id:p.preview_id,digest:p.digest});assert.equal(receipt.saved,26);assert.equal(receipt.reused,0);
 assert.deepEqual(await post('commit',{preview_id:p.preview_id,digest:p.digest}),receipt);
 const s=await state();assert.equal(s.goals.length,before.goals.length+6);assert.equal(s.methods.length,before.methods.length+10);assert.equal(s.catalog.length,before.catalog.length+10);
 for(const key of ['lessons','sessions','knowledge','actions'])assert.deepEqual(s[key],before[key]);
 const attention=s.goals.find(g=>g.input.title==='注意力与学习效能');const method=s.methods.find(m=>m.input.title==='先建立注意，再发出指令');assert.deepEqual(method.input.goal_ids,[attention.id]);
 const intrinsic=s.goals.find(g=>g.input.title==='内在动机与主动探索');assert.equal(intrinsic.input.level,'long');assert(intrinsic.input.criteria.includes('阶段观察'));
 const edited={...intrinsic.input,title:'合成家庭修改后的长期方向'};
 const r=await send('/api/prd/records/'+intrinsic.id,{method:'PUT',headers,body:JSON.stringify({id:intrinsic.id,operation_id:randomUUID(),expected_version:1,change_note:'合成个体化修改',mutation:{kind:'goal',value:edited}})});assert.equal(r.status,200);await r.arrayBuffer();
 const after=await state();const repeat=await post('starter_preview',{keys});assert(repeat.rows.every(row=>row.skipped));
 const retried=await post('commit',{preview_id:repeat.preview_id,digest:repeat.digest});assert.equal(retried.saved,0);assert.equal(retried.skipped,26);assert.deepEqual(await state(),after);
 console.log(`PASS ${label}: starter preview/read-only, atomic six goals/ten methods/ten options, explicit relations, no practices, receipt replay and renamed-content preservation`);
}

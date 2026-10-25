import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {resolve} from 'node:path';
export async function exportContract(label,send,admin){
 const python=process.env.PYTHON3||'python3';
 const headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1',Cookie:admin};
 assert.equal((await send('/api/prd/export.xlsx',{headers:{'X-Skarma-Contract':'skarmar.prd.v1'}})).status,401);
 assert.equal((await send('/api/prd/export.xlsx',{headers:{...headers,'X-Skarma-Contract':'old'}})).status,409);
 const state=await(await send('/api/prd/state',{headers})).json();
 const response=await send('/api/prd/export.xlsx',{headers});assert.equal(response.status,200);assert.equal(response.headers.get('content-type'),'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet');assert(response.headers.get('content-disposition').includes('skarmar-prd-export.xlsx'));
 const expected={'目标':state.goals.length,'方法':state.methods.length,'课程老师':state.catalog.length,'机构课记':state.lessons.length,'家庭干预':state.sessions.length,'经验困难':state.knowledge.length,'下一步':state.actions.length};
 for(const [kind,name] of [['metric','观察指标'],['measurement','实际测量'],['moment','瞬间观察'],['needs','共享需要']])expected[name]=(await(await send('/api/prd/process?view=observations&object_kind='+kind,{headers})).json()).total;
 const bytes=Buffer.from(await response.arrayBuffer());assert(bytes.byteLength>1000);execFileSync(python,[resolve(import.meta.dirname,'verify-prd-export.py'),JSON.stringify(expected)],{input:bytes,stdio:['pipe','inherit','pipe']});
 assert.deepEqual(await(await send('/api/prd/state',{headers})).json(),state,'Export must not alter saved business data');
 console.log(`PASS ${label}: authenticated Excel export and unchanged source state`);
}

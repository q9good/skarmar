// Fresh synthetic workbook through the actual authenticated multipart parser.
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {execFileSync} from 'node:child_process';
export async function transferContract(label,send,admin){
 const python=process.env.PYTHON3||'python3';
 const headers={Origin:'https://prd-contract.local',Cookie:admin,'X-Skarma-Request':'1','X-Skarma-Transfer':'skarmar.transfer.v1','X-Skarma-Contract':'skarmar.prd.v1'};
 async function post(kind,value,status=200,cookie=admin){const r=await send('/api/data-transfer',{method:'POST',headers:{...headers,Cookie:cookie,'Content-Type':'application/json'},body:JSON.stringify({kind,value})});const text=await r.text();assert.equal(r.status,status,`${label} transfer ${kind}: ${text}`);return JSON.parse(text);}
 const ns='synthetic-'+randomUUID();
 const workbook=execFileSync(python,['-c',`from openpyxl import Workbook\nfrom datetime import datetime\nimport sys,io\nw=Workbook();s=w.active;s.title='目标';s.append(['编号','名称','额外字段']);s.append(['G1','合成导入轮替','=SUM(A1:A3)']);s=w.create_sheet('课程');s.append(['编号','日期','课程','老师','目标','内容']);s.append(['L1',datetime(2020,1,2),'合成导入课','合成导入老师','目标:G1','轮替观察']);s=w.create_sheet('经验');s.append(['编号','内容','关联','阶段','状态']);s.append(['K1','合成既有经验','目标:G1','已验证','停用']);s=w.create_sheet('说明');s.append(['原文']);s.append(['保留🙂未映射字段']);b=io.BytesIO();w.save(b);sys.stdout.buffer.write(b.getvalue())`]);
 async function upload(bytes=workbook,status=200,cookie=admin){const boundary='synthetic-'+randomUUID();const r=await send('/api/prd/import/preview',{method:'POST',headers:{...headers,Cookie:cookie,'Content-Type':'multipart/form-data; boundary='+boundary},body:Buffer.concat([Buffer.from(`--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="synthetic.xlsx"\r\nContent-Type: application/octet-stream\r\n\r\n`),bytes,Buffer.from(`\r\n--${boundary}--\r\n`)])});const text=await r.text();assert.equal(r.status,status,`${label} transfer upload: ${text}`);return JSON.parse(text);}
 await upload(Buffer.from('bad'),401,'');
 await post('workbook',{},422);
 const before=await(await send('/api/prd/state',{headers})).json();
 const preview=await upload();assert.equal(preview.sheets.length,4);assert.deepEqual(await(await send('/api/prd/state',{headers})).json(),before);
 const rules=[{sheet:'目标',kind:'goal',level:'short',fields:{key:'编号',title:'名称'}},{sheet:'课程',kind:'lesson',fields:{key:'编号',date:'日期',course:'课程',teacher:'老师',goals:'目标',description:'内容'}},{sheet:'经验',kind:'experience',fields:{key:'编号',title:'内容',goals:'关联',stage:'阶段',state:'状态'}},{sheet:'说明',kind:'archive',fields:{}}];
 const plan=await post('map',{preview_id:preview.preview_id,namespace:ns,rules});
 const confirm={preview_id:plan.preview_id,digest:plan.digest};
 const result=await post('commit',confirm);assert.equal(result.saved,3);assert.equal(result.archived,1);
 assert.deepEqual(await post('commit',confirm),result);
 const raw=await post('source',{key:result.sources.find(r=>r.kind==='goal').key});assert.equal(raw.raw.cells[2],'=SUM(A1:A3)');
 const state=await(await send('/api/prd/state',{headers})).json();assert.equal(state.goals.length,before.goals.length+1);assert.equal(state.lessons.length,before.lessons.length+1);const l=state.lessons.find(r=>r.id===result.sources.find(r=>r.kind==='lesson').target_id);assert.equal(l.observation.business_date,'2020-01-02');assert.equal(l.organization.targets.length,1);const k=state.knowledge.find(k=>k.input.id===result.sources.find(r=>r.kind==='experience').target_id);assert.equal(k.input.stage,'helpful');assert.equal(k.details.state,'retired');
 const p2=await upload();const map2=await post('map',{preview_id:p2.preview_id,namespace:ns,rules});const repeat=await post('commit',{preview_id:map2.preview_id,digest:map2.digest});assert.equal(repeat.skipped,4);
 const missing=await send('/api/data-transfer',{method:'POST',headers:{...headers,'X-Skarma-Transfer':'old','Content-Type':'application/json'},body:JSON.stringify({kind:'history',value:{page:0}})});assert.equal(missing.status,409);await missing.arrayBuffer();
 console.log(`PASS ${label}: actual XLSX multipart import, stable source dedupe, explicit lesson-goal relation, retained raw fields and exact commit replay`);
 return raw;
}

// Offline CLI rehearsal against a disposable synthetic DB, never the configured application DB.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdtemp,mkdir,readFile,rm,stat} from 'node:fs/promises';
import {resolve} from 'node:path';
const root=resolve(import.meta.dirname,'..'),python=process.env.PYTHON3||'python3';await mkdir(resolve(root,'.local'),{recursive:true});const scratch=await mkdtemp(resolve(root,'.local/maintenance-contract-')),db=resolve(scratch,'source.sqlite'),binary=resolve(root,'target/debug/skarma-api');
const base={...process.env,SKARMA_STORAGE:'sqlite',SKARMA_DATABASE:db,SKARMA_MODEL:'prd_v1',SKARMA_DEMO:'0',SKARMA_AUTH:'required'};
const run=(args,input,env=base)=>execFileSync(binary,args,{cwd:root,env,input,stdio:['pipe','pipe','pipe']});
const sql=(query,path=db)=>execFileSync(python,['-c','import sqlite3,sys\nc=sqlite3.connect(sys.argv[1]);r=c.execute(sys.argv[2]);print(r.fetchall());c.commit()',path,query],{encoding:'utf8'}).trim();
try{
 assert.throws(()=>run(['data-transfer-stdin','--offline','--username','owner'],'{"kind":"history","value":{"page":0}}',{...base,SKARMA_DATABASE:resolve(scratch,'must-not-exist.sqlite')}));
 await assert.rejects(stat(resolve(scratch,'must-not-exist.sqlite')));
 run(['admin-init','--username','owner','--name','合成管理员','--password-stdin'],'synthetic-maintenance-password\n');
 const transfer=(kind,value)=>JSON.parse(run(['data-transfer-stdin','--offline','--username','owner'],JSON.stringify({kind,value})).toString());
 const p=transfer('workbook',{filename:'合成.csv',sha256:'synthetic-only',sheets:[{name:'目标',headers:['编号','名称'],rows:[{row:2,cells:['G1','离线合成目标']}]}]});const plan=transfer('map',{preview_id:p.preview_id,namespace:'offline-synthetic',rules:[{sheet:'目标',kind:'goal',level:'short',fields:{key:'编号',title:'名称'}}]});assert.equal(transfer('commit',{preview_id:plan.preview_id,digest:plan.digest}).saved,1);
 sql("INSERT INTO auth_sessions(token_hash,user_id,created_at,expires_at) SELECT 'synthetic-session-hash',id,1,2 FROM auth_users LIMIT 1");
 const backup=resolve(scratch,'before.sqlite');run(['backup',backup]);const bytes=await readFile(backup);
 assert.throws(()=>run(['restore-stdin','--offline'],bytes,{...base,SKARMA_MODEL:'legacy'}));assert.equal(sql('SELECT COUNT(*) FROM auth_sessions'),'[(1,)]');
 run(['restore-stdin','--offline'],bytes);assert.equal(sql('SELECT COUNT(*) FROM auth_sessions'),'[(0,)]');assert.equal(sql('SELECT COUNT(*) FROM prd_entities'),'[(1,)]');assert.equal(sql('SELECT COUNT(*) FROM auth_users'),'[(1,)]');
 assert.equal(transfer('history',{page:0}).batches.length,1);
 console.log('PASS offline CLI: no accidental empty DB, explicit admin import, consistent backup, wrong-model restore refusal, valid new-model restore and session revocation');
}finally{await rm(scratch,{recursive:true,force:true});}

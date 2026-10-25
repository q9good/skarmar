// Fresh synthetic databases; never targets a remote service or shares app data.
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, mkdtemp, rm } from 'node:fs/promises';
import { resolve } from 'node:path';
const root=resolve(import.meta.dirname,'..'),python=process.env.PYTHON3||'python3';
await mkdir(resolve(root,'.local'),{recursive:true});
const scratch=await mkdtemp(resolve(root,'.local/prd-web-'));
try {
  for (const script of (process.argv.length>2?process.argv.slice(2):['smoke_prd_model_web.py','smoke_prd_r15_web.py','smoke_prd_lessons_web.py','smoke_prd_sessions_web.py','smoke_prd_followups_web.py','smoke_prd_offline_web.py','smoke_prd_history_web.py','smoke_prd_auth_sessions_web.py','smoke_prd_observations_web.py','smoke_prd_export_web.py','smoke_prd_import_web.py','smoke_prd_review_pages_web.py','smoke_prd_polish_web.py','smoke_prd_completion_web.py','smoke_prd_balance_web.py','smoke_prd_gap_fixes_web.py','smoke_prd_refinement_web.py','smoke_prd_draft_windows_web.py','smoke_prd_actor_guard_web.py'])) {
    const env={...process.env,SKARMA_STORAGE:'sqlite',SKARMA_MODEL:script==='smoke_prd_foundation_web.py'?'legacy':'prd_v1',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_DATABASE:resolve(scratch,script+'.sqlite'),SKARMA_PORT:'0',SKARMA_PUBLIC_URL:undefined};
    delete env.SKARMA_PUBLIC_URL;
    execFileSync(resolve(root,'target/debug/skarma-api'),['admin-init','--username','owner','--name','测试管理员','--password-stdin'],{cwd:root,env,input:script==='smoke_prd_foundation_web.py'?'synthetic-owner-contract-password\n':'synthetic-prd-ui-password\n',stdio:['pipe','ignore','pipe']});
    const child=spawn(resolve(root,'target/debug/skarma-api'),[],{cwd:root,env,stdio:['ignore','ignore','pipe']});let log='';
    child.stderr.on('data',data=>log+=data);child.on('error',e=>log+=e.message);
    try {
      let port;
      for(let i=0;i<100;i++){port=log.match(/listening on port (\d+)/)?.[1];if(port)break;if(child.exitCode!==null)throw new Error(log);await new Promise(r=>setTimeout(r,50));}
      if(!port)throw new Error('Test server did not start');
      const browser=spawn(python,[resolve(root,'tools',script)],{cwd:root,env:{...process.env,SKARMA_TEST_URL:'http://127.0.0.1:'+port},stdio:'inherit'});
      const [code]=await once(browser,'exit');if(code!==0)throw new Error(`${script} failed: ${code}`);
    } finally {if(child.exitCode===null&&child.signalCode===null){child.kill('SIGTERM');await once(child,'exit');}}
  }
} finally {await rm(scratch,{recursive:true,force:true});}

import assert from 'node:assert/strict';
import { spawn, spawnSync, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, mkdtemp, rm } from 'node:fs/promises';
import { resolve } from 'node:path';
import { Miniflare } from 'miniflare';
import workerdPath,{version as workerdVersion} from 'workerd';
import { prdContract, prdTestPassword } from './prd-contract.mjs';
const root=resolve(import.meta.dirname,'..');
await mkdir(resolve(root,'.local'),{recursive:true});
const scratch=await mkdtemp(resolve(root,'.local/prd-contract-'));
let child,mf;
const database=resolve(scratch,'native.sqlite');
const nativeEnv={...process.env,SKARMA_STORAGE:'sqlite',SKARMA_MODEL:'prd_v1',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_DATABASE:database,SKARMA_PORT:'0',SKARMA_PUBLIC_URL:'https://prd-contract.local',SKARMA_PRODUCTION:'1'};
async function native() {
  let log='';child=spawn(resolve(root,'target/debug/skarma-api'),[],{cwd:root,env:nativeEnv,stdio:['ignore','ignore','pipe']});
  child.stderr.on('data',data=>log+=data.toString());
  child.on('error',e=>log+=e.message);
  for(let i=0;i<100;i++) { const port=log.match(/listening on port (\d+)/)?.[1];if(port)return 'http://127.0.0.1:'+port;if(child.exitCode!==null)throw new Error(log);await new Promise(r=>setTimeout(r,50)); }
  throw new Error('Local server did not start');
}
function workerdEnvironmentBlock() {
  if(process.platform!=='linux')return null;
  const probe=spawnSync(workerdPath.default,['--version'],{encoding:'utf8'});
  if(!probe.error&&probe.status===0)return null;
  const detail=[probe.error?.message,probe.stdout,probe.stderr].filter(Boolean).join('\n');
  if(!/GLIBC_\d+\.\d+/.test(detail))throw new Error(`workerd preflight failed before the Durable Object contract:\n${detail}`);
  const detected=process.report?.getReport?.().header?.glibcVersionRuntime??'unknown';
  return {detected,required:[...new Set([...detail.matchAll(/GLIBC_(\d+\.\d+)/g)].map(match=>match[1]))].sort()};
}
try {
  execFileSync(resolve(root,'target/debug/skarma-api'),['admin-init','--username','owner','--name','测试管理员','--password-stdin'],{cwd:root,env:nativeEnv,input:prdTestPassword+'\n',stdio:['pipe','ignore','pipe']});
  let url=await native();
  const receipt=await prdContract('native SQLite',(path,init)=>fetch(url+path,init));
  child.kill('SIGTERM');await once(child,'exit');url=await native();
  assert.deepEqual(await(await fetch(url+'/api/prd/state',{headers:{Cookie:receipt.admin}})).json(),receipt.snapshot);
  for(const [kind,expected] of Object.entries(receipt.observations))assert.deepEqual(await(await fetch(url+'/api/prd/process?view=observations&object_kind='+kind,{headers:{Cookie:receipt.admin}})).json(),expected);
  for(const [path,expected] of Object.entries(receipt.completion))assert.deepEqual(await(await fetch(url+'/api'+path,{headers:{Cookie:receipt.admin}})).json(),expected);
  assert.deepEqual(await(await fetch(url+'/api/data-transfer',{method:'POST',headers:{Cookie:receipt.admin,'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Transfer':'skarmar.transfer.v1',Origin:'https://prd-contract.local'},body:JSON.stringify({kind:'source',value:{key:receipt.importSource.key}})})).json(),receipt.importSource);
  child.kill('SIGTERM');await once(child,'exit');
  console.log('PASS native SQLite runtime: full PRD contract, restart, authentication, histories and export/import-source checks');
  const blocked=workerdEnvironmentBlock();
  if(blocked){
    const requirements=blocked.required.length?`；workerd 二进制需要 GLIBC_${blocked.required.join('/GLIBC_')}`:'';
    console.log(`SKIP Rust Worker / Durable Object: 环境受限 — workerd ${workerdVersion} 无法在本机运行（glibc ${blocked.detected} < 2.32${requirements}）。native SQLite 半段已完整运行并通过；请在 glibc ≥2.32 的机器（如 Mac 或较新 Linux）上运行 DO 半段。`);
  }else{
    const options={modules:true,modulesRoot:root,scriptPath:resolve(root,'crates/worker/build/index.js'),
      modulesRules:[{type:'ESModule',include:['**/*.js','**/*.mjs']},{type:'CompiledWasm',include:['**/*.wasm']}],compatibilityDate:'2026-07-30',compatibilityFlags:['nodejs_compat'],
      durableObjects:{SKARMA_DB:{className:'SkarmaSpace',useSQLite:true}},durableObjectsPersist:resolve(scratch,'do'),
      bindings:{SKARMA_STORAGE:'durable_object',SKARMA_MODEL:'prd_v1',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_PRODUCTION:'1',SKARMA_SPACE:'prd-contract',SKARMA_PUBLIC_URL:'https://prd-contract.local',SKARMA_INITIAL_ADMIN_USER:'owner',SKARMA_INITIAL_ADMIN_PASSWORD:prdTestPassword}};
    mf=new Miniflare(options);
    const cf=await prdContract('Rust Worker / Durable Object',(path,init)=>mf.dispatchFetch('https://prd-contract.local'+path,init));
    await mf.dispose();mf=new Miniflare(options);
    assert.deepEqual(await(await mf.dispatchFetch('https://prd-contract.local/api/prd/state',{headers:{Cookie:cf.admin}})).json(),cf.snapshot);
    for(const [kind,expected] of Object.entries(cf.observations))assert.deepEqual(await(await mf.dispatchFetch('https://prd-contract.local/api/prd/process?view=observations&object_kind='+kind,{headers:{Cookie:cf.admin}})).json(),expected);
    for(const [path,expected] of Object.entries(cf.completion))assert.deepEqual(await(await mf.dispatchFetch('https://prd-contract.local/api'+path,{headers:{Cookie:cf.admin}})).json(),expected);
    assert.deepEqual(await(await mf.dispatchFetch('https://prd-contract.local/api/data-transfer',{method:'POST',headers:{Cookie:cf.admin,'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Transfer':'skarmar.transfer.v1',Origin:'https://prd-contract.local'},body:JSON.stringify({kind:'source',value:{key:cf.importSource.key}})})).json(),cf.importSource);
    console.log('PASS both runtimes: restart preserves new model, authentication and immutable histories');
  }
} finally {
  if(mf)await mf.dispose();
  if(child?.pid&&child.exitCode===null&&child.signalCode===null){child.kill('SIGTERM');await once(child,'exit');}
  await rm(scratch,{recursive:true,force:true});
}

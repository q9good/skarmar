// Real local SQLite + Rust Wasm/DO rehearsal of the explicitly selected empty
// baseline. Synthetic accounts/records only; never connects to a public app.
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {once} from 'node:events';
import {randomUUID} from 'node:crypto';
import {mkdtemp,mkdir,rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {Miniflare} from 'miniflare';
const root=resolve(import.meta.dirname,'..');
await mkdir(resolve(root,'.local'),{recursive:true});
const scratch=await mkdtemp(resolve(root,'.local/empty-baseline-'));
const origin='https://empty-baseline.local',password='synthetic-cutover-password';
let child,mf;
const env={...process.env,SKARMA_DATABASE:resolve(scratch,'native.sqlite'),SKARMA_STORAGE:'sqlite',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_PRODUCTION:'1',SKARMA_PUBLIC_URL:origin,SKARMA_PORT:'0'};
async function stop(){if(child&&child.exitCode===null&&child.signalCode===null){child.kill('SIGTERM');await once(child,'exit');}}
async function native(model,baseline){
  await stop();let log='';
  const runtime={...env,SKARMA_MODEL:model};
  delete runtime.SKARMA_PRD_BASELINE;
  if(baseline)runtime.SKARMA_PRD_BASELINE='empty_preserve_legacy_v1';
  child=spawn(resolve(root,'target/debug/skarma-api'),[],{cwd:root,env:runtime,stdio:['ignore','ignore','pipe']});
  child.stderr.on('data',v=>log+=v);child.on('error',e=>log+=e.message);
  for(let i=0;i<150;i++){
    const port=log.match(/listening on port (\d+)/)?.[1];
    if(port)return(path,init)=>fetch('http://127.0.0.1:'+port+path,init);
    if(child.exitCode!==null)throw Error(log);
    await new Promise(r=>setTimeout(r,40));
  }
  throw Error('Local synthetic server startup failed');
}
async function worker(model,baseline){
  if(mf)await mf.dispose();
  mf=new Miniflare({modules:true,modulesRoot:root,scriptPath:resolve(root,'crates/worker/build/index.js'),modulesRules:[{type:'ESModule',include:['**/*.js','**/*.mjs']},{type:'CompiledWasm',include:['**/*.wasm']}],compatibilityDate:'2026-07-30',compatibilityFlags:['nodejs_compat'],durableObjects:{SKARMA_DB:{className:'SkarmaSpace',useSQLite:true}},durableObjectsPersist:resolve(scratch,'do'),bindings:{SKARMA_STORAGE:'durable_object',SKARMA_MODEL:model,SKARMA_SPACE:'same-original-space',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_PRODUCTION:'1',SKARMA_PUBLIC_URL:origin,SKARMA_INITIAL_ADMIN_USER:'owner',SKARMA_INITIAL_ADMIN_PASSWORD:password,...(baseline?{SKARMA_PRD_BASELINE:'empty_preserve_legacy_v1'}:{})}});
  return(path,init)=>mf.dispatchFetch(origin+path,init);
}
async function contract(label,restart){
  let send=await restart('legacy',false),cookie;
  async function api(method,path,body,expected=200,overrideCookie=cookie){
    const response=await send('/api'+path,{method,headers:{Origin:origin,'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','Content-Type':'application/json',...(overrideCookie?{Cookie:overrideCookie}:{})},body:body===undefined?undefined:JSON.stringify(body)});
    const text=await response.text();assert.equal(response.status,expected,`${label} ${path}: ${text}`);
    if(path==='/auth/login'&&response.ok)cookie=response.headers.get('set-cookie').split(';')[0];
    return JSON.parse(text);
  }
  const login=()=>api('POST','/auth/login',{username:'owner',password});
  const identity=await login();
  const oldGoal=randomUUID();
  await api('POST','/goals',{id:oldGoal,title:'合成旧目标保留',level:'short',parent_id:null,area:'',criteria:''});
  await api('PUT','/lessons/'+randomUUID(),{expected_version:null,lesson:{business_date:'2020-01-02',course:'合成旧课程',teacher:'合成旧老师',content:'合成旧内容',goals:'',problems:'',home_cooperation:''}});
  send=await restart('prd_v1',true);
  // Exactly the same unissued-session probe as the production release checks.
  await api('POST','/auth/logout',{},200,'__Host-skarma_session='+randomUUID().replaceAll('-','').repeat(2));
  assert.equal((await api('GET','/auth/me')).user.id,identity.user.id,'original session/account remains usable');
  assert.equal((await api('GET','/runtime')).model,'prd_v1');
  let state=await api('GET','/prd/state');
  for(const key of ['goals','methods','lessons','sessions','knowledge','actions'])assert.equal(state[key].length,0,key+' starts empty');
  await api('POST','/goals',{id:randomUUID(),title:'stale page',level:'short',parent_id:null,area:'',criteria:''},409);
  const newGoal=randomUUID();
  await api('PUT','/prd/records/'+newGoal,{id:newGoal,operation_id:randomUUID(),expected_version:null,change_note:'',mutation:{kind:'goal',value:{title:'合成新目标',level:'short'}}});
  state=await api('GET','/prd/state');assert.equal(state.goals[0].id,newGoal);
  send=await restart('prd_v1',true);
  assert.deepEqual(await api('GET','/prd/state'),state,'configured flag never resets new records');
  send=await restart('prd_v1',false);
  assert.deepEqual(await api('GET','/prd/state'),state,'persistent marker supports normal startup without baseline flag');
  assert.equal((await login()).user.id,identity.user.id,'original password still works');
  console.log(`PASS ${label}: original account/session, empty new domain, stale legacy write refused, new data survives restart and flag removal`);
}
try{
  execFileSync(resolve(root,'target/debug/skarma-api'),['admin-init','--username','owner','--name','合成管理员','--password-stdin'],{cwd:root,env:{...env,SKARMA_MODEL:'legacy'},input:password+'\n',stdio:['pipe','ignore','pipe']});
  await contract('native SQLite',native);await stop();
  await contract('Rust Wasm / Durable Object',worker);
}finally{
  await stop();if(mf)await mf.dispose();await rm(scratch,{recursive:true,force:true});
}

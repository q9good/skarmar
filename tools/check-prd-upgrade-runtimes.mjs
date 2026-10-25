// Local-only model cutover rehearsals: original SQLite file and original DO namespace.
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {spawn,execFileSync} from 'node:child_process';
import {once} from 'node:events';
import {mkdtemp,mkdir,rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {Miniflare} from 'miniflare';
const root=resolve(import.meta.dirname,'..');await mkdir(resolve(root,'.local'),{recursive:true});const scratch=await mkdtemp(resolve(root,'.local/upgrade-contract-'));let child,mf;
const password='synthetic-upgrade-only-password',origin='https://upgrade-contract.local';
async function stopNative(){if(child?.exitCode===null&&child.signalCode===null){child.kill('SIGTERM');await once(child,'exit');}}
const nativeEnv={...process.env,SKARMA_STORAGE:'sqlite',SKARMA_DATABASE:resolve(scratch,'native.sqlite'),SKARMA_PORT:'0',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_PRODUCTION:'1',SKARMA_PUBLIC_URL:origin};
async function startNative(model){await stopNative();let log='';child=spawn(resolve(root,'target/debug/skarma-api'),[],{cwd:root,env:{...nativeEnv,SKARMA_MODEL:model},stdio:['ignore','ignore','pipe']});child.stderr.on('data',v=>log+=v);child.on('error',e=>log+=e.message);for(let n=0;n<100;n++){const port=log.match(/listening on port (\d+)/)?.[1];if(port){const base='http://127.0.0.1:'+port;return(path,init)=>fetch(base+path,init);}if(child.exitCode!==null)throw new Error(log);await new Promise(r=>setTimeout(r,50));}throw new Error('Synthetic native startup failed');}
async function startWorker(model){if(mf)await mf.dispose();mf=new Miniflare({modules:true,modulesRoot:root,scriptPath:resolve(root,'crates/worker/build/index.js'),modulesRules:[{type:'ESModule',include:['**/*.js','**/*.mjs']},{type:'CompiledWasm',include:['**/*.wasm']}],compatibilityDate:'2026-07-30',compatibilityFlags:['nodejs_compat'],durableObjects:{SKARMA_DB:{className:'SkarmaSpace',useSQLite:true}},durableObjectsPersist:resolve(scratch,'do'),bindings:{SKARMA_STORAGE:'durable_object',SKARMA_MODEL:model,SKARMA_SPACE:'same-upgrade-space',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_PRODUCTION:'1',SKARMA_PUBLIC_URL:origin,SKARMA_INITIAL_ADMIN_USER:'owner',SKARMA_INITIAL_ADMIN_PASSWORD:password}});return(path,init)=>mf.dispatchFetch(origin+path,init);}
async function contract(label,restart){
 let send=await restart('legacy'),cookie;
 async function api(method,path,body,status=200){const r=await send('/api'+path,{method,headers:{Origin:origin,'X-Skarma-Request':'1','X-Skarma-Transfer':'skarmar.transfer.v1','X-Skarma-Contract':'skarmar.prd.v1',...(cookie?{Cookie:cookie}:{}),...(body===undefined?{}:{'Content-Type':'application/json'})},body:body===undefined?undefined:JSON.stringify(body)});const text=await r.text();assert.equal(r.status,status,`${label} ${path}: ${text}`);if(path==='/auth/login')cookie=r.headers.get('set-cookie').split(';')[0];return JSON.parse(text);}
 const transfer=(kind,value,status=200)=>api('POST','/data-transfer',{kind,value},status);
 await api('POST','/auth/login',{username:'owner',password});
 const goal=await api('POST','/goals',{id:randomUUID(),title:'合成旧本领',level:'short',parent_id:null,area:'原领域保留',criteria:'合成可观察标准'});
 const session={business_date:'2020-01-02',time_slot:'晚上',activities:['轮流游戏'],organization:'居家',plan:'一起滚球',observation:'合成观察',targets:[{goal_id:goal.id,role:'primary',plan:'轮流',progress:'轮替三次后休息',outcome:'continue',next_step:''}],has_difficulty:true,difficulty:'等待时需要帮助',has_experience:false,experience:'',experience_kind:'task'};
 const completed=await api('PUT','/sessions/'+randomUUID(),{operation_id:randomUUID(),expected_version:null,action:'complete',session});
 await api('PUT','/sessions/'+randomUUID(),{operation_id:randomUUID(),expected_version:null,action:'save_draft',session:{...session,observation:'',has_difficulty:false,difficulty:''}});
 await api('PUT','/lessons/'+randomUUID(),{expected_version:null,lesson:{business_date:'2020-01-02',course:'旧课程',teacher:'旧老师',content:'课堂滚球',goals:'原文不自动提取',problems:'',home_cooperation:'',links:{targets:[{goal_id:goal.id,role:'primary',plan:'',progress:'轮替观察'}],pathway_ids:[]}}});
 const experience=await api('POST','/experiences',{id:randomUUID(),content:'合成既有有效实践',goal_ids:[goal.id],pathway_ids:[]});await api('PUT','/experiences/'+experience.id,{expected_version:experience.version,status:'verified',next_review_date:null,conclusion:'旧家人已有观察依据'});
 const original=await api('GET','/state');let p=await transfer('upgrade_preview');
 const confirm=()=>({source_digest:p.source_digest,note:'合成库维护演练，已核对原库与恢复路径',sessions:p.sessions.map(s=>({id:s.id,disposition:s.id===completed.session.id?'completed':'archive',actual_goal_ids:s.id===completed.session.id?[goal.id]:[],observations:{}}))});
 let result=await transfer('upgrade_commit',confirm());
 send=await restart('prd_v1');assert.equal((await api('GET','/runtime')).model,'prd_v1');let state=await api('GET','/prd/state');assert.equal(state.sessions.length,1);assert.equal(state.lessons.length,1);assert.equal(state.knowledge.length,2);assert(state.knowledge.some(k=>k.source_session_id===completed.session.id));const existing=state.knowledge.find(k=>k.input.id===experience.id);assert.equal(existing.input.stage,'helpful');assert.equal(existing.details.evidence,'旧家人已有观察依据');assert.equal(state.sessions[0].input.summary.confirmation,'unknown');assert.equal(state.sessions[0].actual_targets[0].goal.input.title,'合成旧本领');assert.equal(state.lessons[0].teacher.name,'旧老师');
 const sources=await transfer('sources',{batch_id:result.receipt.batch_id,page:0});assert(sources.sources.length>0);assert((await transfer('source',{key:sources.sources[0].key})).raw);
 const replay=await transfer('upgrade_commit',confirm());assert.equal(replay.baseline,result.baseline);
 await transfer('rollback',{baseline:result.baseline,note:'合成未写入回滚'});
 send=await restart('legacy');assert.deepEqual(await api('GET','/state'),original,'rollback preserves exact original business records and original account login');
 p=await transfer('upgrade_preview');result=await transfer('upgrade_commit',confirm());send=await restart('prd_v1');
 const moment=randomUUID();await api('POST','/prd/process',{kind:'observe',value:{id:moment,operation_id:randomUUID(),expected_version:null,kind:'moment',action:'save',input:{business_date:'2020-01-02',text:'合成转换后新观察',goal_ids:[goal.id]},note:''}});
 await transfer('rollback',{baseline:result.baseline,note:'必须阻止丢失新增观察'},409);
 send=await restart('prd_v1');state=await api('GET','/prd/state');assert.equal(state.sessions.length,1);assert.equal((await api('GET','/prd/process?view=observations&object_kind=moment')).items[0].id,moment);
 console.log(`PASS ${label}: same-store upgrade, original login/lesson/family/knowledge/source retention, restart, safe rollback, and post-write rollback refusal`);
}
try{execFileSync(resolve(root,'target/debug/skarma-api'),['admin-init','--username','owner','--name','合成管理员','--password-stdin'],{cwd:root,env:{...nativeEnv,SKARMA_MODEL:'legacy'},input:password+'\n',stdio:['pipe','ignore','pipe']});await contract('native SQLite',startNative);await stopNative();await contract('Rust Wasm / Durable Object',startWorker);}finally{await stopNative();if(mf)await mf.dispose();await rm(scratch,{recursive:true,force:true});}

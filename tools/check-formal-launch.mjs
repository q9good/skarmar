// Local synthetic DO rehearsal: ordinary -> one-time launch -> ordinary upgrade.
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {cp,mkdir,mkdtemp,rm,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {Miniflare} from 'miniflare';

const root=resolve(import.meta.dirname,'..'),ack='retain-goals-courses-teachers-accounts-20261101',epoch='formal-20261101';
await mkdir(resolve(root,'.local'),{recursive:true});
const scratch=await mkdtemp(resolve(root,'.local/formal-launch-'));
const origin='https://formal-launch.local',password='synthetic-formal-launch-password';
const env={...process.env};delete env.SKARMA_FORMAL_LAUNCH;
let mf,admin;
const options=scriptPath=>({name:'formal-launch-contract',modules:true,modulesRoot:root,scriptPath,
 modulesRules:[{type:'ESModule',include:['**/*.js','**/*.mjs']},{type:'CompiledWasm',include:['**/*.wasm']}],
 compatibilityDate:'2026-07-30',compatibilityFlags:['nodejs_compat'],
 durableObjects:{SKARMA_DB:{className:'SkarmaSpace',useSQLite:true}},durableObjectsPersist:resolve(scratch,'do'),
 bindings:{SKARMA_STORAGE:'durable_object',SKARMA_MODEL:'prd_v1',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_PRODUCTION:'1',SKARMA_SPACE:'default',SKARMA_PUBLIC_URL:origin}});
async function call(method,path,body,status=200,generation=epoch,cookie=admin){
 const r=await mf.dispatchFetch(origin+'/api'+path,{method,headers:{'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1',Origin:origin,
  ...(cookie?{Cookie:cookie}:{}),...(generation?{'X-Skarma-Data-Epoch':generation}:{})},body:body===undefined?undefined:JSON.stringify(body)});
 const data=await r.json();assert.equal(r.status,status,`${method} ${path}: ${JSON.stringify(data)}`);return {data,cookie:r.headers.get('set-cookie')?.split(';')[0]};
}
const write=async(kind,value)=>{
 const id=randomUUID();return (await call('PUT','/prd/records/'+id,{id,operation_id:randomUUID(),expected_version:null,change_note:'',mutation:{kind,value},...(kind==='method'?{method_create_mode:'quick'}:{})})).data.value;
};
const state=async()=>(await call('GET','/prd/state')).data;
const newLesson=async(course,teacher)=>{
 const id=randomUUID();return (await call('PUT','/prd/lessons/'+id,{id,operation_id:randomUUID(),expected_version:null,action:{kind:'observation',value:{business_date:'2020-01-01',course_id:course.id,teacher_id:teacher.id,notes:'合成课堂记录'}}})).data;
};
try {
 console.log('Building ordinary synthetic baseline');
 execFileSync('worker-build',['--release'],{cwd:resolve(root,'crates/worker'),env,stdio:['ignore','pipe','pipe']});
 await cp(resolve(root,'crates/worker/build'),resolve(scratch,'normal'),{recursive:true});
 const initial=options(resolve(scratch,'normal/index.js'));
 initial.bindings={...initial.bindings,SKARMA_INITIAL_ADMIN_USER:'owner',SKARMA_INITIAL_ADMIN_PASSWORD:password};
 mf=new Miniflare(initial);
 admin=(await call('POST','/auth/login',{username:'owner',password})).cookie;assert(admin);
 await call('POST','/auth/users',{username:'family',display_name:'合成家人',password:'s',role:'member'});
 const long=await write('goal',{title:'合成长期关注',level:'long',long_priority:2});
 const goal=await write('goal',{title:'合成短期目标',level:'short',parent_id:long.id,criteria:'保留观察标准'});
 const course=await write('catalog',{kind:'course',name:'合成课程',active:true});
 const teacher=await write('catalog',{kind:'teacher',name:'合成老师',active:true});
 const method=await write('method',{title:'试用方法',goal_ids:[goal.id]});
 const knowledge=randomUUID();
 await call('POST','/prd/process',{kind:'knowledge',value:{id:knowledge,operation_id:randomUUID(),expected_version:null,action:'create',input:{id:knowledge,kind:'difficulty',content:'试用困难',goal_ids:[goal.id],method_ids:[method.id]},details:{title:'试用困难'},note:''}});
 await newLesson(course,teacher);
 const session=randomUUID();
 await call('PUT','/prd/sessions/'+session,{id:session,operation_id:randomUUID(),expected_version:null,action:'save_draft',input:{business_date:'2020-01-01',time_slot:'晚上',plan:'合成：示范轮流，等待孩子回应',targets:[{goal_id:goal.id,role:'main'}],methods:[{id:method.id,version:method.version}]},change_note:''});
 const before=await state(),users=(await call('GET','/auth/users')).data;
 const history=(await call('GET','/prd/records/'+goal.id+'/history')).data;
 assert(before.methods.length&&before.lessons.length&&before.sessions.length&&before.knowledge.length);
 assert.deepEqual((await call('GET','/release/formal-launch',undefined,200,null,null)).data,{epoch,completed:false});
 await mf.dispose();mf=null;
 console.log('Synthetic trial fixture saved; building explicit one-time cleanup binary');
 execFileSync('worker-build',['--release'],{cwd:resolve(root,'crates/worker'),env:{...env,SKARMA_FORMAL_LAUNCH:ack},stdio:['ignore','pipe','pipe']});
 await cp(resolve(root,'crates/worker/build'),resolve(scratch,'formal'),{recursive:true});
 // No bootstrap secret exists in the formal/future deployments; accounts must survive.
 mf=new Miniflare(options(resolve(scratch,'formal/index.js')));
 assert.deepEqual((await call('GET','/release/formal-launch',undefined,200,null,null)).data,{epoch,completed:true});
 console.log('Cleanup transaction committed; checking retained data, stale pages and later writes');
 const after=await state();assert.deepEqual(after.goals,before.goals);assert.deepEqual(after.catalog,before.catalog);
 for(const k of ['methods','lessons','sessions','knowledge','actions'])assert.deepEqual(after[k],[]);
 assert.deepEqual((await call('GET','/auth/users')).data,users);
 assert.deepEqual((await call('GET','/prd/records/'+goal.id+'/history')).data,history);
 await call('GET','/prd/state',undefined,409,null);await call('GET','/prd/state',undefined,409,'old-epoch');
 const late=randomUUID();await call('PUT','/prd/records/'+late,{id:late,operation_id:randomUUID(),expected_version:null,mutation:{kind:'method',value:{title:'旧页面重传',goal_ids:[goal.id]}}},409,null);
 // A real post-launch write must survive repeated hook and a later normal binary.
 const fresh=await write('method',{title:'正式新增方法',goal_ids:[goal.id]});assert(fresh.id);
 await newLesson(course,teacher);const formalState=await state();
 await mf.dispose();mf=new Miniflare(options(resolve(scratch,'formal/index.js')));
 assert.deepEqual(await state(),formalState);
 await mf.dispose();mf=new Miniflare(options(resolve(scratch,'normal/index.js')));
 assert.deepEqual(await state(),formalState);
 const family=(await call('POST','/auth/login',{username:'family',password:'s'})).cookie;assert(family);
 assert.deepEqual((await call('GET','/prd/state',undefined,200,epoch,family)).data,formalState);
 assert.deepEqual((await call('GET','/release/formal-launch',undefined,200,null,null)).data,{epoch,completed:true});
 const result={synthetic:true,checks:['preserved_goals_hierarchy_catalog_accounts_histories','trial_records_cleared','old_browser_and_outbox_rejected','new_formal_records_survive_restart','ordinary_binary_upgrade_preserves_formal_data','member_password_still_works'],errors:[]};
 await writeFile(resolve(root,'.local/formal-launch-checks.json'),JSON.stringify(result,null,2)+'\n');
 console.log('PASS Rust Wasm / Durable Object: formal cleanup, retained data, stale-client isolation, restart and ordinary upgrade; no SKIP');
} finally {
 if(mf)await mf.dispose();
 await rm(scratch,{recursive:true,force:true});
 // Leave a normal (non-cleanup) local build for ordinary runtime/browser checks.
 execFileSync('worker-build',['--release'],{cwd:resolve(root,'crates/worker'),env,stdio:['ignore','pipe','pipe']});
}

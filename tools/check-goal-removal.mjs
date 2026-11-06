// Ordinary -> exact synthetic catalogue removal -> ordinary, same local DO/credentials.
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {cp,mkdir,mkdtemp,rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {Miniflare} from 'miniflare';
const root=resolve(import.meta.dirname,'..');await mkdir(resolve(root,'.local'),{recursive:true});
const scratch=await mkdtemp(resolve(root,'.local/goal-removal-')),build=resolve(root,'crates/worker/build'),normal=resolve(scratch,'normal');
const env={...process.env};delete env.SKARMA_FORMAL_LAUNCH;delete env.SKARMA_GOAL_REMOVAL;
const origin='https://goal-removal.local',password='synthetic-goal-removal-password';let mf,cookie,haveNormal=false;
const options=scriptPath=>({modules:true,modulesRoot:root,scriptPath,modulesRules:[{type:'ESModule',include:['**/*.js','**/*.mjs']},{type:'CompiledWasm',include:['**/*.wasm']}],compatibilityDate:'2026-07-30',compatibilityFlags:['nodejs_compat'],durableObjects:{SKARMA_DB:{className:'SkarmaSpace',useSQLite:true}},durableObjectsPersist:resolve(scratch,'do'),bindings:{SKARMA_STORAGE:'durable_object',SKARMA_MODEL:'prd_v1',SKARMA_DEMO:'0',SKARMA_AUTH:'required',SKARMA_PRODUCTION:'1',SKARMA_SPACE:'default',SKARMA_PUBLIC_URL:origin}});
async function call(method,path,body,status=200,session=cookie){const r=await mf.dispatchFetch(origin+'/api'+path,{method,headers:{Origin:origin,'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101',...(session?{Cookie:session}:{})},body:body===undefined?undefined:JSON.stringify(body)});const data=await r.json();assert.equal(r.status,status,`${method} ${path}: ${JSON.stringify(data)}`);return {data,cookie:r.headers.get('set-cookie')?.split(';')[0]};}
const state=async()=>(await call('GET','/prd/state')).data;
const put=async(kind,value)=>{const id=randomUUID(),request={id,operation_id:randomUUID(),expected_version:null,change_note:'',mutation:{kind,value},...(kind==='method'?{method_create_mode:'quick'}:{})};return {saved:(await call('PUT','/prd/records/'+id,request)).data.value,request};};
try{
 execFileSync('worker-build',['--release'],{cwd:resolve(root,'crates/worker'),env,stdio:['ignore','pipe','pipe']});await cp(build,normal,{recursive:true});haveNormal=true;
 const first=options(resolve(normal,'index.js'));first.bindings={...first.bindings,SKARMA_INITIAL_ADMIN_USER:'owner',SKARMA_INITIAL_ADMIN_PASSWORD:password};mf=new Miniflare(first);
 cookie=(await call('POST','/auth/login',{username:'owner',password})).cookie;assert(cookie);
 await call('POST','/auth/users',{username:'family',display_name:'合成家人',password:'s',role:'member'});
 const a=await put('goal',{title:'合成待移除甲',level:'medium'}),b=await put('goal',{title:'合成待移除乙',level:'short'}),keep=await put('goal',{title:'合成保留长期',level:'long'});
 const course=await put('catalog',{kind:'course',name:'合成保留课程',active:true}),teacher=await put('catalog',{kind:'teacher',name:'合成保留老师',active:true});
 await put('method',{title:'合成保留方法',goal_ids:[keep.saved.id]});
 const sid=randomUUID();await call('PUT','/prd/sessions/'+sid,{id:sid,operation_id:randomUUID(),expected_version:null,action:'complete',change_note:'',input:{business_date:'2020-01-01',time_slot:'晚上',summary:{targets:[{goal_id:keep.saved.id,performed:true,observation:'合成保留家庭记录'}],confirmation:'none'}}});
 const lid=randomUUID();await call('PUT','/prd/lessons/'+lid,{id:lid,operation_id:randomUUID(),expected_version:null,action:{kind:'observation',value:{business_date:'2020-01-01',course_id:course.saved.id,teacher_id:teacher.saved.id,notes:'合成保留课堂观察'}}});
 const retiredSessionId=randomUUID(),retiredInput={business_date:'2020-01-01',time_slot:'下午',summary:{targets:[{goal_id:a.saved.id,performed:true,observation:'合成已保存旧目标观察'}],confirmation:'none',new_knowledge:[{id:randomUUID(),kind:'experience',content:'合成保留旧关联经验',stage:'try',goal_ids:[a.saved.id]}],new_actions:[{id:randomUUID(),content:'合成保留旧关联行动',goal_ids:[a.saved.id]}]}};
 const retiredBefore=(await call('PUT','/prd/sessions/'+retiredSessionId,{id:retiredSessionId,operation_id:randomUUID(),expected_version:null,action:'complete',change_note:'',input:retiredInput})).data;
 const before=await state(),identity=(await call('GET','/auth/me')).data;await mf.dispose();mf=null;
 execFileSync('worker-build',['--release'],{cwd:resolve(root,'crates/worker'),env:{...env,SKARMA_GOAL_REMOVAL:'remove-synthetic-two-goals-20261105'},stdio:['ignore','pipe','pipe']});mf=new Miniflare(options(resolve(build,'index.js')));
 assert.deepEqual((await call('GET','/release/goal-removal',undefined,200,null)).data,{batch:'synthetic-goals-20261105',completed:true});
 const after=await state();assert.deepEqual(after.goals,before.goals.filter(g=>![a.saved.id,b.saved.id].includes(g.id)));assert.deepEqual(new Set(after.removed_goal_ids),new Set([a.saved.id,b.saved.id]));
 for(const key of ['catalog','sessions','lessons','methods','knowledge','actions'])assert.deepEqual(after[key],before[key],key+' must remain unchanged');
 assert.deepEqual((await call('GET','/auth/me')).data,identity);
 assert.deepEqual((await call('PUT','/prd/records/'+a.saved.id,a.request)).data.value,a.saved,'old successful receipts remain immutable');assert.deepEqual(await state(),after);
 await call('PUT','/prd/records/'+a.saved.id,{...a.request,operation_id:randomUUID(),expected_version:1},409);
 assert.equal((await call('GET','/prd/records/'+a.saved.id+'/history')).data.length,1);
 await call('GET','/release/goal-removal',undefined,200,null);assert.deepEqual(await state(),after);
 await mf.dispose();mf=null;mf=new Miniflare(options(resolve(normal,'index.js')));assert.deepEqual(await state(),after);assert.deepEqual((await call('GET','/auth/me')).data,identity);
 const revised=(await call('PUT','/prd/sessions/'+retiredSessionId,{id:retiredSessionId,operation_id:randomUUID(),expected_version:1,action:'complete',change_note:'合成修订旧观察',input:{...retiredInput,summary:{...retiredInput.summary,targets:[{...retiredInput.summary.targets[0],observation:'合成修订旧目标观察'}]}}})).data;
 assert.deepEqual(revised.actual_targets[0].goal,retiredBefore.actual_targets[0].goal);assert.equal(revised.actual_targets[0].input.note,'合成修订旧目标观察');
 const forbiddenId=randomUUID();await call('PUT','/prd/sessions/'+forbiddenId,{id:forbiddenId,operation_id:randomUUID(),expected_version:null,action:'complete',change_note:'',input:retiredInput},422);
 assert.equal((await call('GET','/prd/sessions/'+retiredSessionId+'/history')).data.length,2);
 const again=await put('goal',{title:'合成待移除甲',level:'short'});assert((await state()).goals.some(g=>g.id===again.saved.id),'a new explicit record must not be repeatedly removed by title');
 console.log('PASS synthetic DO removal: exact two catalogue entries, original accounts/sessions/courses/teachers/methods/records retained; anonymous read-only status, immutable receipts/history, no resurrection, idempotence and future ordinary upgrade');
}finally{if(mf)await mf.dispose();if(haveNormal){await rm(build,{recursive:true,force:true});await cp(normal,build,{recursive:true});}await rm(scratch,{recursive:true,force:true});}

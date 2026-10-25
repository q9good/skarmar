import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';

// The same public contract runs against native SQLite and the actual Rust Wasm
// worker. All objects are synthetic and each runtime gets a disposable database.
export async function completionContract(label,send,admin){
 const headers={'Content-Type':'application/json','X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1',Origin:'https://prd-contract.local',Cookie:admin};
 async function call(method,path,body,status=200){const r=await send('/api'+path,{method,headers,body:body===undefined?undefined:JSON.stringify(body)});const data=await r.json();assert.equal(r.status,status,`${label} completion ${path}: ${JSON.stringify(data)}`);return data;}
 const entity=async(kind,value)=>{const id=randomUUID();return(await call('PUT','/prd/records/'+id,{id,operation_id:randomUUID(),expected_version:null,change_note:'',mutation:{kind,value}})).value;};
 const g=await entity('goal',{title:'合成补全轮替目标',level:'short'}),m=await entity('method',{title:'合成轮流示范',goal_ids:[g.id]});
 const c=await entity('catalog',{kind:'course',name:'合成补全课程',active:true}),t=await entity('catalog',{kind:'teacher',name:'合成补全老师',active:true});
 const kid=randomUUID();await call('POST','/prd/process',{kind:'knowledge',value:{id:kid,operation_id:randomUUID(),expected_version:null,action:'create',input:{id:kid,kind:'experience',content:'合成先跟随再示范',goal_ids:[g.id],method_ids:[m.id],stage:'helpful'},details:{title:'合成已有经验'}}});
 const lid=randomUUID(),obs={business_date:'2020-01-01',course_id:c.id,teacher_id:t.id,notes:'合成旧观察',course_started:true};
 const lesson=(version,kind,value)=>({id:lid,operation_id:randomUUID(),expected_version:version,action:{kind,value}});
 await call('PUT','/prd/lessons/'+lid,lesson(null,'observation',obs));
 await call('PUT','/prd/lessons/'+lid,lesson(1,'observation',{...obs,business_date:'2020-01-02',notes:'合成新观察'}));
 const fid=randomUUID(),input={seen_observation_version:1,targets:[{goal_id:g.id,note:'合成课堂表现'}],method_ids:[m.id],feedback:[{id:fid,knowledge_id:kid,result:'helpful',observation:'合成愿意再试',expected_version:null}],feedback_knowledge_versions:{[kid]:1}};
 await call('PUT','/prd/lessons/'+lid,lesson(1,'organization',{...input,feedback_knowledge_versions:{[kid]:2}}),409);
 const command=lesson(1,'organization',input),saved=await call('PUT','/prd/lessons/'+lid,command);assert.deepEqual(await call('PUT','/prd/lessons/'+lid,command),saved);
 assert.equal(saved.organization.feedback_ids.length,1);assert.equal(saved.observation_version,2);assert.equal(saved.organization.seen_observation_version,1);
 const eventPath='/prd/process?view=lesson_feedback&id='+lid,events=await call('GET',eventPath);assert.equal(events.length,1);assert.equal(events[0].input.business_date,'2020-01-01');assert.equal(events[0].source_snapshot.referenced_observation.value.notes,'合成旧观察');assert.equal(events[0].source_snapshot.observation.notes,'合成新观察');
 await call('PUT','/prd/lessons/'+lid,lesson(2,'organization',{...input,feedback:[{...input.feedback[0],id:randomUUID()}]}),409);
 const sid=randomUUID();await call('PUT','/prd/sessions/'+sid,{id:sid,operation_id:randomUUID(),expected_version:null,action:'complete',input:{business_date:'2020-01-03',time_slot:'晚上',targets:[{goal_id:g.id,role:'main'}],summary:{targets:[{goal_id:g.id,performed:true,observation:'合成实际表现'}],confirmation:'used',methods:[{id:m.id,version:1}],first_try_ids:[m.id],context:'合成家人原文：当天换了材料'}}});
 const contextPath='/prd/process?view=review_context&object_kind=goal&id='+g.id,context=await call('GET',contextPath);const changes=context.evidence.flatMap(e=>e.changes??[]);assert(changes.some(c=>c.kind==='method_first_try'));assert(changes.some(c=>c.kind==='course_started'));assert(changes.some(c=>c.text==='合成家人原文：当天换了材料'));
 const review={id:randomUUID(),operation_id:randomUUID(),object_kind:'goal',object_id:g.id,action:'complete',schedule_version:context.schedule.version,generation:context.schedule.generation,due_date:context.schedule.due_date,object_version:context.object_version,decision:'continue',note:'合成继续观察',evidence:[],next_actions:[],seen_versions:context.seen_versions,caregiver_state:'stretched'};
 const reviewed=await call('POST','/prd/process',{kind:'review',value:review});assert.equal(reviewed.review.caregiver_state,'stretched');assert.deepEqual(await call('POST','/prd/process',{kind:'review',value:review}),reviewed);
 const observe=async(kind,id,value,version=null)=>call('POST','/prd/process',{kind:'observe',value:{id,operation_id:randomUUID(),expected_version:version,kind,action:'save',input:value,note:version?'合成修订':''}});
 const metricId=randomUUID();await observe('metric',metricId,{goal_id:g.id,goal_version:1,name:'合成机会测量',template:'count',criterion:'5次机会',support:'提醒一次'});
 for(const [date,n,k] of [['2020-01-01',0,0],['2020-01-02',5,3]])await observe('measurement',randomUUID(),{metric_id:metricId,metric_version:1,business_date:date,support:'提醒一次',opportunities:n,achieved:k});
 const progressPath='/prd/process?view=goal_progress',progress=await call('GET',progressPath),entry=progress.goals.find(v=>v.id===g.id);assert.equal(entry.measurement_count,2);assert.equal(entry.segment_count,1);assert.equal(entry.latest[0].input.achieved,3);assert.equal(entry.latest[0].metric.version,1);
 const queuePath='/prd/process?view=review_queue',oldQueue=await call('GET',queuePath);
 await call('PUT','/prd/records/'+g.id,{id:g.id,operation_id:randomUUID(),expected_version:1,change_note:'合成只更正名称',mutation:{kind:'goal',value:{...g.input,title:'合成当前队列目标名'}}});
 const freshQueue=await call('GET',queuePath);assert.equal(freshQueue.state.goals.find(v=>v.id===g.id).input.title,'合成当前队列目标名');assert(freshQueue.board.data_version>oldQueue.board.data_version);assert.equal(freshQueue.board.schedules.filter(s=>s.object_kind==='method'&&s.object_id===m.id).length,1);assert(!('feedback' in freshQueue.state));
 console.log(`PASS ${label}: explicit queue refresh returns current names, relations and one schedule per object in one snapshot`);

 // Final gap regressions cross the public JSON and both storage runtimes.
 const difficulty=randomUUID(),today=new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date());
 await call('POST','/prd/process',{kind:'knowledge',value:{id:difficulty,operation_id:randomUUID(),action:'create',input:{id:difficulty,kind:'difficulty',content:'合成持续离开',goal_ids:[g.id],method_ids:[m.id]},details:{title:'合成困难比较期',state:'following',basis:'相同机会',support:'一次提示'}}});
 const feedback=async(version)=>{const id=randomUUID();return call('POST','/prd/process',{kind:'feedback',value:{id,operation_id:randomUUID(),action:'record',input:{knowledge_id:difficulty,source:{kind:'independent',id,version:1},business_date:today,result:'same',observation:'合成同样条件下的观察',comparable:true,basis:'相同机会',support:'一次提示',method_refs:[{id:m.id,version}]}}});};
 await feedback(1);await feedback(1);
 const adjust={id:m.id,operation_id:randomUUID(),expected_version:1,method_change:'adjustment',change_note:'合成实际做法调整',mutation:{kind:'method',value:{...m.input,description:'合成先跟随再等待'}}};
 const adjusted=await call('PUT','/prd/records/'+m.id,adjust);assert.deepEqual(await call('PUT','/prd/records/'+m.id,adjust),adjusted);
 await feedback(2);await feedback(1);
 const knowledgePath='/prd/process?view=knowledge&id='+difficulty,comparison=await call('GET',knowledgePath);assert.equal(comparison.summary.total,4);assert.equal(comparison.summary.comparable_count,1);assert.equal(comparison.summary.offer_weekly,false);assert.equal(comparison.summary.comparison_boundary.method_version,2);
 const dc=await call('GET','/prd/process?view=review_context&object_kind=knowledge&id='+difficulty);
 const difficultyReview={id:randomUUID(),operation_id:randomUUID(),object_kind:'knowledge',object_id:difficulty,action:'complete',schedule_version:dc.schedule.version,generation:dc.schedule.generation,due_date:dc.schedule.due_date,object_version:dc.object_version,decision:'adjust',note:'合成同日再次调整',seen_versions:dc.seen_versions};
 const dr=await call('POST','/prd/process',{kind:'review',value:difficultyReview});await feedback(2);assert.equal((await call('GET',knowledgePath)).summary.comparable_count,1);
 await call('POST','/prd/process',{kind:'review',value:{...difficultyReview,operation_id:randomUUID(),expected_version:1,action:'revise',note:'合成仅更正文字'}});assert.equal((await call('GET',knowledgePath)).summary.comparable_count,1);
 const preserved=await entity('goal',{title:'合成省略字段原名',level:'short',meaning:'合成必须保留的意义',criteria:'合成必须保留的标准',state:'paused',resume_on:'2099-01-01',frequency:'monthly',archived:true});
 const partialRename={id:preserved.id,operation_id:randomUUID(),expected_version:1,change_note:'合成只改名字',mutation:{kind:'goal',value:{title:'合成只更名',level:'short'}}};
 const renamed=await call('PUT','/prd/records/'+preserved.id,partialRename);assert.equal(renamed.value.input.meaning,preserved.input.meaning);assert.equal(renamed.value.input.criteria,preserved.input.criteria);assert.equal(renamed.value.input.resume_on,'2099-01-01');assert.equal(renamed.value.input.archived,true);assert.deepEqual(await call('PUT','/prd/records/'+preserved.id,partialRename),renamed);
 await call('PUT','/prd/records/'+preserved.id,{...partialRename,mutation:{kind:'goal',value:{...partialRename.mutation.value,meaning:''}}},409);
 const cleared=await call('PUT','/prd/records/'+preserved.id,{...partialRename,operation_id:randomUUID(),expected_version:2,mutation:{kind:'goal',value:{...partialRename.mutation.value,meaning:'',resume_on:null,archived:false}}});assert.equal(cleared.value.input.meaning,'');assert.equal(cleared.value.input.resume_on,null);assert.equal(cleared.value.input.criteria,preserved.input.criteria);
 const pm=await entity('method',{title:'合成保留方法',goal_ids:[g.id],description:'合成保留做法',steps:'合成保留步骤',guidance:'合成老师示范',state:'paused',resume_on:'2099-01-01',pinned:true});
 const renamedMethod=await call('PUT','/prd/records/'+pm.id,{id:pm.id,operation_id:randomUUID(),expected_version:1,change_note:'合成只改方法名',mutation:{kind:'method',value:{title:'合成方法新名',goal_ids:[g.id]}}});assert.equal(renamedMethod.value.input.description,pm.input.description);assert.equal(renamedMethod.value.input.steps,pm.input.steps);assert.equal(renamedMethod.value.input.resume_on,'2099-01-01');assert.equal(renamedMethod.value.input.pinned,true);
 console.log(`PASS ${label}: omitted fields survive HTTP/DO transport and retries; explicit empties/null clear only selected fields`);
 const partialGoal=await entity('goal',{title:'合成未展开证据',level:'short'});
 for(let i=0;i<5;i++)await observe('moment',randomUUID(),{business_date:'2020-01-01',text:'合成较早记录'+i,goal_ids:[partialGoal.id]});
 const partialPath='/prd/process?view=review_context&object_kind=goal&id='+partialGoal.id,pc=await call('GET',partialPath);assert.equal(pc.total,5);
 const shown=Object.assign({},...pc.evidence.slice(0,3).map(e=>e.seen_versions));
 await call('POST','/prd/process',{kind:'review',value:{id:randomUUID(),operation_id:randomUUID(),object_kind:'goal',object_id:partialGoal.id,action:'complete',schedule_version:pc.schedule.version,generation:pc.schedule.generation,due_date:pc.schedule.due_date,object_version:pc.object_version,decision:'continue',note:'合成只回看三条',seen_versions:shown}});
 const remaining=await call('GET',partialPath);assert.equal(remaining.total,2);assert(remaining.evidence.every(e=>e.carried_over));
 for(const [view,id] of [['entity_history',m.id],['lesson_history',lid],['session_history',sid]]){const history=await call('GET',`/prd/process?view=${view}&id=${id}&page=0`);assert(history.items.length>0&&history.items.length<=30);assert.equal(history.page,0);}
 console.log(`PASS ${label}: method adjustment splits old evidence, retries exactly once, bounded histories and partial reviews retain older unseen records`);
 const reviewPath='/prd/process?view=review_context&object_kind=goal&id='+g.id;
 console.log(`PASS ${label}: atomic lesson feedback, exact old observation references, retry/conflict, explicit concurrent changes, caregiver state and measured progress`);
 const persisted={};for(const path of [eventPath,progressPath,reviewPath,queuePath,knowledgePath,partialPath])persisted[path]=await call('GET',path);return persisted;
}

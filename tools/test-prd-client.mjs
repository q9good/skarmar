import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';
import { acceptSaved, decodeDrafts, emptySnapshot, mergeSnapshot, newGoal, newMethod } from '../apps/client/src/foundation.ts';
import {sessionExpiresSoon,TRUSTED_RENEWAL_NOTICE_SECONDS} from '../apps/client/src/session-renewal.ts';
import {businessDateLabel,todayRecords} from '../apps/client/src/prd-today.ts';
test('today records use the Shanghai business date, authors, time slots and sync recency',()=>{
 const now=Date.parse('2026-10-25T12:05:00Z'),today='2026-10-25';
 const lesson={id:'lesson',status:'recorded',created_at:'2026-10-25T12:04:00Z',creator_name:'奶奶',observation:{business_date:today},course:{name:'语言课'},teacher:{name:'王老师'}};
 const session={id:'session',status:'completed',created_at:'2026-10-25T12:00:00Z',creator_name:'爸爸',input:{business_date:today,time_slot:'晚上',plan:'',summary:{targets:[{performed:true,observation:'主动表达两次'}]}}};
 const moment={id:'moment',kind:'moment',version:1,created_at:'2026-10-25T11:00:00Z',creator_name:'妈妈',deleted:false,input:{business_date:today,text:'拿书给家人看'},audit:{actor_name:'妈妈',at:'2026-10-25T11:00:00Z'}};
 const rows=todayRecords({...emptySnapshot,lessons:[lesson],sessions:[session]},[moment],today,now);
 assert.equal(businessDateLabel(today),'2026-10-25 周日');assert.deepEqual(rows.map(row=>row.kind),['lesson','session','moment']);
 assert.deepEqual(rows.map(row=>row.author),['奶奶','爸爸','妈妈']);assert.deepEqual(rows.map(row=>row.timing),['20:04','晚上','19:00']);
 assert.deepEqual(rows.map(row=>row.sync),['刚保存 · 已同步','刚保存 · 已同步','已同步']);
});
test('trusted renewal prompt starts at fourteen days while standard sessions retain the one-day window',()=>{
 const now=1000,day=86400;
 assert.equal(sessionExpiresSoon({expires_at:now+TRUSTED_RENEWAL_NOTICE_SECONDS+1,trusted_device:true},now),false);
 assert.equal(sessionExpiresSoon({expires_at:now+TRUSTED_RENEWAL_NOTICE_SECONDS,trusted_device:true},now),true);
 assert.equal(sessionExpiresSoon({expires_at:now+day+1,trusted_device:false},now),false);
 assert.equal(sessionExpiresSoon({expires_at:now+day,trusted_device:false},now),true);
});
test('late reads and retried older responses do not roll an observed version back',()=>{
  const initial={...emptySnapshot,goals:[{id:'g',version:1,input:{...newGoal(),title:'before'}}]};
  const latest=acceptSaved(initial,{kind:'goal',value:{id:'g',version:3,input:{...newGoal(),title:'latest'}}});
  const late=mergeSnapshot(latest,initial);
  const replay=acceptSaved(late,{kind:'goal',value:initial.goals[0]});
  assert.equal(replay.goals[0].version,3);assert.equal(replay.goals[0].input.title,'latest');
  assert.equal(replay.goals.length,1);
});
test('multiple drafts and exact pending operations survive serialization; malformed queues fail closed',()=>{
  const goal={id:'g',operation_id:'op',expected_version:null,change_note:'',mutation:{kind:'goal',value:newGoal()}};
  const method={id:'m',operation_id:'op2',expected_version:1,change_note:'revision',mutation:{kind:'method',value:newMethod()}};
  const drafts={g:{...goal,pending:goal},m:method};
  assert.deepEqual(decodeDrafts(JSON.stringify(drafts)),drafts);
  const legacyValue={...newGoal()};delete legacyValue.domain;delete legacyValue.due;delete legacyValue.support;
  const legacy={...goal,mutation:{kind:'goal',value:legacyValue}};
  assert.deepEqual(decodeDrafts(JSON.stringify({g:legacy})),{g:legacy});
  for(const raw of ['[]','null','invalid',JSON.stringify({g:{...goal,mutation:{kind:'goal'}}}),JSON.stringify({wrong:goal}),JSON.stringify({g:{...goal,pending:{...goal,id:'other'}}})]) assert.throws(()=>decodeDrafts(raw));
  assert.deepEqual(decodeDrafts(null),{});
});

import { commonChoices, decodeLessonDrafts, emptyObservation, lessonSearchSources, monthRange, shiftMonth, teacherTotals } from '../apps/client/src/prd-lessons.ts';
const lesson=(id,day,teacher='t',course='c',status='recorded')=>({id,version:1,observation_version:1,organization_version:1,created_at:day+'T00:00:00Z',creator_id:'u',creator_name:'测试',acknowledgements:[],request_generation:0,requested_by:null,request_name:null,request_message:'',status,marked_error:false,observation:{...emptyObservation(day),course_id:course,teacher_id:teacher},teacher:{id:teacher,name:'旧名称',kind:'teacher',version:1},course:{id:course,name:'课程',kind:'course',version:1},organization:{seen_observation_version:0,targets:[],methods:[],focus_goals:[],processed_request_generation:0,state:'pending',claimed_by:null,claimed_name:null,outcome:''}});
test('lesson totals use stable teacher identity, include marked errors, exclude tombstones and respect calendar bounds',()=>{
  const rows=[lesson('one','2024-02-01'),{...lesson('two','2024-02-29'),marked_error:true,teacher:{id:'t',name:'新名称'}},lesson('three','2024-02-20','t','c','withdrawn'),lesson('four','2024-02-20','t','c','deleted'),lesson('five','2024-03-01')];
  const {start,end}=monthRange('2024-02');assert.equal(end,'2024-02-29');assert.equal(monthRange('2025-02').end,'2025-02-28');
  const groups=teacherTotals(rows,start,end);assert.equal(groups.length,1);assert.equal(groups[0].count,2);assert.equal(groups[0].marked,1);
  assert.equal(shiftMonth('2024-01',-1),'2023-12');assert.equal(shiftMonth('2023-12',1),'2024-01');
});
test('Appendix D lesson settlement uses stable teacher IDs, historical names and every countability boundary',()=>{
  const fixture=JSON.parse(readFileSync(new URL('./fixtures/prd-coverage.json',import.meta.url),'utf8'));
  const rows=fixture.records.filter(row=>row.source==='lesson').map(row=>{
    const value=lesson(row.id,row.day,row.teacher,row.course,row.status);
    return {...value,marked_error:Boolean(row.marked_error),teacher:{...value.teacher,name:row.teacher_name},organization:{...value.organization,targets:row.targets.map(([goal_id])=>({target:{goal_id,note:''}}))}};
  });
  const summary=groups=>groups.sort((a,b)=>a.id.localeCompare(b.id)).map(({id,name,count,marked})=>({id,name,count,marked}));
  assert.deepEqual(summary(teacherTotals(rows,'2026-10-01','2026-10-31','2026-10-24')),[
    {id:'T1',name:'测试甲新名',count:2,marked:0},
    {id:'T2',name:'测试乙',count:2,marked:1},
  ]);
  assert.deepEqual(summary(teacherTotals(rows,'2026-09-01','2026-09-30','2026-10-24')),[{id:'T1',name:'测试甲',count:1,marked:0}]);
  assert.equal(rows.find(row=>row.id==='L4').organization.targets.length,0);
  assert.equal(rows.find(row=>row.id==='L5').status,'withdrawn');
});
test('lesson search reports each exact source and never guesses teacher advice from free text',()=>{
  const row={...lesson('l','2026-10-24'),course:{id:'c',name:'语言课'},teacher:{id:'t',name:'王老师'},observation:{...emptyObservation('2026-10-24'),notes:'玩轮流游戏',teacher_notes:'先等待再示范',difficulty_notes:'离开座位'}};
  const knowledge=[
    {deleted:false,source_lesson_id:'l',input:{kind:'difficulty',content:'等待时需要支持'}},
    {deleted:false,source_lesson_id:'l',input:{kind:'experience',content:'视觉提示有帮助'}},
    {deleted:false,source_lesson_id:'other',input:{kind:'difficulty',content:'不应匹配'}},
  ];
  assert.deepEqual(lessonSearchSources(row,knowledge,'语言'),['课程']);
  assert.deepEqual(lessonSearchSources(row,knowledge,'王老师'),['老师']);
  assert.deepEqual(lessonSearchSources(row,knowledge,'等待'),['老师建议','关联困难']);
  assert.deepEqual(lessonSearchSources(row,knowledge,'轮流'),['课堂观察']);
  assert.deepEqual(lessonSearchSources(row,knowledge,'视觉'),['关联经验']);
  assert(!lessonSearchSources(row,knowledge,'轮流').includes('老师建议'));
});
test('course shortcuts favor recent same weekday, teacher shortcuts rank independently and inactive or old records do not suggest',()=>{
  const catalog=[['c','course'],['x','course'],['t','teacher'],['u','teacher'],['off','teacher']].map(([id,kind])=>({id,version:1,input:{kind,name:id,active:id!=='off'}}));
  const rows=[lesson('a','2024-02-26','u','x'),lesson('b','2024-02-29'),lesson('d','2024-02-28'),lesson('old','2023-01-01','u','x'),lesson('off','2024-03-03','off','c')];
  assert.equal(commonChoices(rows,catalog,'course','2024-03-04')[0].id,'x');
  assert.equal(commonChoices(rows,catalog,'teacher','2024-03-04')[0].id,'t');
  assert.deepEqual(commonChoices([],catalog,'teacher','2024-03-04'),[]);
});
test('lesson drafts validate nested payloads, zones and exact pending requests before rendering without losing original raw data',()=>{
  const request={id:'l',operation_id:'op',expected_version:null,action:{kind:'observation',value:emptyObservation('2024-03-04')}};
  const draft={request,phase:'basic',pending:request};const valid={'l:observation':draft};
  assert.deepEqual(decodeLessonDrafts(JSON.stringify(valid)),valid);assert.deepEqual(decodeLessonDrafts(null),{});
  const invalid=[null,[],{'l:observation':{...draft,phase:'organization'}},{wrong:draft},{'l:observation':{...draft,request:{...request,action:{kind:'observation',value:{}}}}},{'l:observation':{...draft,pending:{...request,operation_id:'different'}}},{'l:observation':{...draft,backup:{...request,id:'other'}}}];
  for(const value of invalid)assert.throws(()=>decodeLessonDrafts(JSON.stringify(value)));
  const saved={...lesson('l','2024-03-04'),version:4,status:'withdrawn'};
  const combined=mergeSnapshot({...emptySnapshot,lessons:[saved]},{...emptySnapshot,lessons:[{...saved,version:3,status:'recorded'}]});
  assert.equal(combined.lessons[0].status,'withdrawn');
});

import {chooseMain,decodeSessionDrafts,emptySession,monthWeeks,startSummary} from '../apps/client/src/prd-sessions.ts';
test('swapping the main target retains each target note and summary never assumes planned methods were actually used',()=>{
 const targets=[{goal_id:'a',role:'main',note:'note a'},{goal_id:'b',role:'secondary',note:'note b'}];
 const swapped=chooseMain(targets,'b');assert.equal(swapped[0].note,'note a');assert.equal(swapped[1].note,'note b');assert.deepEqual(swapped.map(t=>t.role),['secondary','main']);
 const input={...emptySession('2026-10-24'),targets,methods:[{id:'m',version:2}],tags:['游戏']};const actual=startSummary(input);assert.equal(actual.confirmation,'unconfirmed');assert.deepEqual(actual.methods,[]);assert.deepEqual(actual.tags,['游戏']);assert.equal(actual.targets[0].observation,'');assert.deepEqual(actual.knowledge_details,{});
 const request={id:'s',operation_id:'op',expected_version:null,action:'save_draft',input,change_note:''};const d={request,stage:'plan',pending:request};assert.deepEqual(decodeSessionDrafts(JSON.stringify({s:d})),{s:d});
 const legacySummary={...actual};delete legacySummary.knowledge_details;const legacyRequest={...request,id:'legacy',input:{...input,summary:legacySummary}};
 assert.deepEqual(decodeSessionDrafts(JSON.stringify({legacy:{request:legacyRequest,stage:'summary'}})).legacy.request.input.summary,legacySummary);
 const orphan={...actual,knowledge_details:{orphan:emptyDetails()}},orphanRequest={...request,input:{...input,summary:orphan}};
 assert.throws(()=>decodeSessionDrafts(JSON.stringify({s:{request:orphanRequest,stage:'summary'}})));
 assert.throws(()=>decodeSessionDrafts(JSON.stringify({s:{...d,pending:{...request,operation_id:'other'}}})));
 assert.throws(()=>decodeSessionDrafts(JSON.stringify({s:{...d,request:{...request,input:{...input,summary:{targets:[]}}}}})));
 assert.deepEqual(decodeSessionDrafts(null),{});
 const quick={request:{id:'new-method',operation_id:'new-op',expected_version:null,change_note:'',method_create_mode:'quick',mutation:{kind:'method',value:{...newMethod(),title:'quick',goal_ids:['a']}}},stage:'plan',kind:'method'};
 assert.deepEqual(decodeSessionDrafts(JSON.stringify({s:{...d,quick}})).s.quick,quick);
 assert.throws(()=>decodeSessionDrafts(JSON.stringify({s:{...d,quick:{...quick,stage:'summary'}}})));

});
test('monthly calendar covers leap days with Monday alignment and one entry per real day',()=>{
 const weeks=monthWeeks('2024-02');assert.equal(weeks[0][0],'');assert.equal(weeks[0][3],'2024-02-01');assert.equal(weeks.flat().filter(Boolean).length,29);assert.equal(weeks.flat().at(-1),'2024-02-29');
});

import {decodeProcessDrafts,emptyDetails,practiceOf} from '../apps/client/src/prd-followups.ts';
test('followup drafts preserve exact pending operations and reject malformed nested outputs',()=>{
 const command={kind:'knowledge',value:{id:'k',operation_id:'op',expected_version:null,action:'create',input:{id:'k',kind:'difficulty',content:'',goal_ids:[],method_ids:[],stage:null,next_review_date:null},details:emptyDetails(),derived_from:[],note:''}};
 const queue={'knowledge:k':{command,pending:command}};assert.deepEqual(decodeProcessDrafts(JSON.stringify(queue)),queue);assert.deepEqual(decodeProcessDrafts(null),{});
 assert.throws(()=>decodeProcessDrafts(JSON.stringify({'knowledge:k':{command:{...command,value:{...command.value,derived_from:[null]}}}})));
 assert.throws(()=>decodeProcessDrafts(JSON.stringify({'knowledge:k':{...queue['knowledge:k'],pending:{...command,value:{...command.value,operation_id:'different'}}}})));
 const event={id:'e',version:2,input:{knowledge_id:'k',source:{kind:'session',id:'s',version:1},business_date:'2026-10-24',result:'helpful',observation:'actual',attempt:'',adult_response:'',conditions:'',reason:'',comparable:false,basis:'',support:'',method_refs:[]}};
 const embedded=practiceOf(event);assert.equal(embedded.expected_version,2);assert.equal(embedded.observation,'actual');assert(!('source' in embedded));assert(!('business_date' in embedded));assert(!('method_refs' in embedded));
 const action={kind:'action',value:{id:'a',operation_id:'op',expected_version:2,input:{id:'a',content:'next',goal_ids:['g'],method_ids:[],assignee_id:null},state:'pending',note:''}};
 assert.deepEqual(decodeProcessDrafts(JSON.stringify({'action:a':{command:action}}))['action:a'].command,action);
});

import {decodeOutbox,LessonOutbox} from '../apps/client/src/prd-outbox.ts';
function outboxFixture(overrides={}) {
 let raw=null,chain=Promise.resolve(),online=true,active=true;const sent=[],receipts=[];
 const snapshot={...emptySnapshot,catalog:['c','t'].map((id,i)=>({id,version:1,input:{kind:i?'teacher':'course',name:id,active:true}}))};
 const io={read:async()=>raw,write:async v=>{raw=v;},lock:fn=>{const task=chain.catch(()=>{}).then(fn);chain=task;return task;},
  identity:async()=>({user:{id:'u',active:true},space:'home',demo:false}),runtime:async()=>({model:'prd_v1',contract:'skarmar.prd.v1'}),state:async()=>snapshot,
  send:async w=>{sent.push(structuredClone(w));return w.kind==='catalog'?{kind:'catalog',value:{id:w.value.id,version:1,input:w.value.mutation.value}}:lesson(w.value.id,'2026-10-24');},
  online:()=>online,active:()=>active,accepted:r=>receipts.push(r),changed:()=>{},...overrides};
 const queue=new LessonOutbox({user:'u',space:'home'},io);
 return {queue,io,snapshot,sent,receipts,raw:()=>raw,offline:()=>{online=false;},online:()=>{online=true;},stop:()=>{active=false;}};
}
const queuedLesson=(id='q',course='c',teacher='t')=>({kind:'lesson',value:{id,operation_id:'op-'+id,expected_version:null,action:{kind:'observation',value:{...emptyObservation('2026-10-24'),course_id:course,teacher_id:teacher}}}});
const queuedCatalog=(id,kind)=>({kind:'catalog',value:{id,operation_id:'op-'+id,expected_version:null,change_note:'',mutation:{kind:'catalog',value:{kind,name:'新选项'+id,active:true}}}});
test('offline lesson is durable but not a server receipt; recovery/reload replays one immutable operation',async()=>{
 const f=outboxFixture();await f.queue.load();f.offline();const w=queuedLesson();await f.queue.enqueue(w);await f.queue.flush();assert.equal(f.sent.length,0);assert.equal(decodeOutbox(f.raw()).entries.q.status,'queued');
 const reopened=new LessonOutbox({user:'u',space:'home'},f.io);await reopened.load();f.online();await Promise.all([reopened.flush(),reopened.flush()]);assert.equal(f.sent.length,1);assert.equal(f.sent[0].value.operation_id,w.value.operation_id);assert.equal(reopened.value.entries.q.status,'confirmed');
});
test('catalog dependencies are confirmed before lessons, with stable IDs and no name placeholders',async()=>{
 const f=outboxFixture();await f.queue.load();await f.queue.enqueue(queuedLesson('new','new-course','new-teacher'));await f.queue.enqueue(queuedCatalog('new-teacher','teacher'));await f.queue.enqueue(queuedCatalog('new-course','course'));await f.queue.flush();
 assert.deepEqual(f.sent.map(w=>w.kind),['catalog','catalog','lesson']);assert.equal(f.queue.value.entries.new.status,'confirmed');
});
test('lost response after commit retries the same request and ignores stale catalog preflight until receipt is known',async()=>{
 let committed,attempts=0;const f=outboxFixture({send:async w=>{attempts++;if(!committed){committed=structuredClone(w);throw new Error('network lost');}assert.deepEqual(w,committed);return lesson(w.value.id,'2026-10-24');}});
 await f.queue.load();await f.queue.enqueue(queuedLesson());await f.queue.flush();assert.equal(f.queue.value.entries.q.status,'uncertain');
 f.snapshot.catalog[0].input.active=false;const reopened=new LessonOutbox({user:'u',space:'home'},f.io);await reopened.load();await reopened.flush();assert.equal(attempts,2);assert.equal(reopened.value.entries.q.status,'confirmed');
});
test('inactive catalog blocks only dependent lessons, keeps data and requires explicit resume',async()=>{
 const f=outboxFixture();f.snapshot.catalog[0].input.active=false;await f.queue.load();await f.queue.enqueue(queuedLesson());await f.queue.flush();assert.equal(f.sent.length,0);assert.equal(f.queue.value.entries.q.status,'blocked');
 f.snapshot.catalog[0].input.active=true;await f.queue.flush();assert.equal(f.sent.length,0);await f.queue.resume('q');assert.equal(f.sent.length,1);
});
test('account/space or contract mismatch never uploads cached records',async()=>{
 for(const overrides of [{identity:async()=>({user:{id:'other',active:true},space:'home',demo:false})},{identity:async()=>({user:{id:'u',active:true},space:'other',demo:false})},{runtime:async()=>({model:'legacy',contract:'legacy'})}]){
  const f=outboxFixture(overrides);await f.queue.load();await f.queue.enqueue(queuedLesson());await f.queue.flush();assert.equal(f.sent.length,0);assert.equal(f.queue.value.entries.q.status,'blocked');
 }
});
test('two tabs cannot overwrite queued data or upload a confirmed operation twice',async()=>{
 const f=outboxFixture(),other=new LessonOutbox({user:'u',space:'home'},f.io);await f.queue.load();await other.load();await Promise.all([f.queue.enqueue(queuedLesson('a')),other.enqueue(queuedLesson('b'))]);
 await Promise.all([f.queue.flush(),other.flush()]);assert.equal(f.sent.length,2);assert.equal(Object.keys(decodeOutbox(f.raw()).entries).length,2);
 await assert.rejects(()=>other.enqueue({...queuedLesson('a'),value:{...queuedLesson('a').value,operation_id:'changed'}}));
});
test('storage failure never sends, unknown queue versions fail closed without overwriting originals',async()=>{
 let sends=0,writes=0;const f=outboxFixture({write:async()=>{throw new Error('quota full');},send:async()=>{sends++;}});await f.queue.load();await assert.rejects(()=>f.queue.enqueue(queuedLesson()));await f.queue.flush();assert.equal(sends,0);
 const bad=JSON.stringify({schema:'future',entries:{}}),q=new LessonOutbox({user:'u',space:'home'},{...f.io,read:async()=>bad,write:async()=>{writes++;}});await assert.rejects(()=>q.load());assert.equal(q.ready,false);assert.equal(writes,0);
 for(const raw of ['[]','null','{}',JSON.stringify({schema:'skarmar.lesson-queue.v1',entries:{q:{write:queuedLesson(),status:'confirmed',reason:''}}})])assert.throws(()=>decodeOutbox(raw));
});

import {historyEvidence,timeDistribution,matchesEvidence} from '../apps/client/src/prd-history.ts';
test('history is deduplicated by actual source, follows immutable ancestor snapshots, excludes future and nonformal records',()=>{
 const family=(id,day,status='completed')=>({id,status,created_at:day+'T00:00:00Z',input:{business_date:day},actual_targets:[{goal:{id:'child'},ancestors:[{id:'old-parent'}]},{goal:{id:'other-child'},ancestors:[{id:'old-parent'}]}],actual_methods:[{id:'m'}],focus_goals:[{id:'long'}]});
 const l={...lesson('l','2026-10-20'),created_at:'2026-10-20T00:00:00Z',organization:{targets:[{goal:{id:'child',input:{parent_id:'old-parent'}},ancestors:[{id:'old-parent'}]}],methods:[{id:'m'}],focus_goals:[]}};
 const data={...emptySnapshot,sessions:[family('a','2026-10-24'),family('b','2026-06-01'),family('future','2099-01-01'),family('draft','2026-10-23','summary_draft'),family('gone','2026-10-23','deleted')],lessons:[l,{...l,id:'withdrawn',status:'withdrawn'}]};
 const rows=historyEvidence(data,'2026-10-24','combined');assert.equal(rows.length,3);assert.equal(rows.filter(r=>matchesEvidence(r,'old-parent','goal')).length,3);assert.equal(rows.filter(r=>matchesEvidence(r,'new-parent','goal')).length,0);assert.equal(rows.filter(r=>matchesEvidence(r,'long','goal',true)).length,2);assert.equal(rows.filter(r=>matchesEvidence(r,'m','method')).length,3);
 const d=timeDistribution(rows,'2026-10-24');assert.equal(d.count,2);assert.equal(d.last,'2026-10-24');assert.equal(d.weeks.length,13);assert.equal(d.weeks.reduce((n,w)=>n+w.count,0),2);assert.equal(historyEvidence(data,'2026-10-24').length,2);
});
test('90-day weekly bars cover each included boundary date once while recent date stays visible outside window',()=>{
 const dates=['2026-07-26','2026-07-27','2026-08-02','2026-10-24'];const rows=dates.reverse().map(date=>({date}));const d=timeDistribution(rows,'2026-10-24');assert.equal(d.start,'2026-07-27');assert.equal(d.count,3);assert.equal(d.weeks.reduce((n,w)=>n+w.count,0),3);assert.equal(d.weeks[0].start,d.start);assert.equal(d.weeks.at(-1).end,'2026-10-24');
 const old=timeDistribution([{date:'2026-01-01'}],'2026-10-24');assert.equal(old.count,0);assert.equal(old.last,'2026-01-01');
});

import {prioritizeMethods} from '../apps/client/src/prd-history.ts';
test('method order explains pin/difficulty/overdue/normal/achieved without increasing practice or changing knowledge',()=>{
 const methods=['normal','achieved','delayed','difficulty','pinned'].map(id=>({id,version:1,input:{...newMethod(),title:id,goal_ids:[id==='achieved'?'done':id],pinned:id==='pinned'}}));
 const data={...emptySnapshot,methods,goals:[{id:'done',version:1,input:{...newGoal(),state:'achieved'}}],knowledge:[{input:{id:'k',kind:'difficulty',goal_ids:[],method_ids:['difficulty']},details:{state:'following'},deleted:false}]};
 assert.deepEqual(prioritizeMethods(data,['delayed']).map(r=>r.id),['pinned','difficulty','delayed','normal','achieved']);
 assert.equal(prioritizeMethods(data,['delayed'])[1].reason,'关联未解决困难，先看看是否需要调整');assert.equal(data.knowledge[0].details.state,'following');
});
test('account pause does not erase ambiguous status: original receipt stays retrievable after explicit resume',async()=>{
 let wrong=false,attempts=0;const f=outboxFixture({identity:async()=>({user:{id:wrong?'other':'u',active:true},space:'home',demo:false}),send:async w=>{attempts++;if(attempts===1)throw new Error('lost');return lesson(w.value.id,'2026-10-24');}});
 await f.queue.load();await f.queue.enqueue(queuedLesson());await f.queue.flush();wrong=true;await f.queue.flush();assert.equal(f.queue.value.entries.q.status,'blocked');assert.equal(f.queue.value.entries.q.attempted,true);wrong=false;f.snapshot.catalog[0].input.active=false;await f.queue.resume('q');assert.equal(attempts,2);assert.equal(f.queue.value.entries.q.status,'confirmed');
});
test('confirmed server save with failed local receipt is reported saved and remains safe to retry',async()=>{
 let stored=null,failed=false,saves=0;const f=outboxFixture({read:async()=>stored,write:async raw=>{if(JSON.parse(raw).entries.q?.status==='confirmed'&&!failed){failed=true;throw new Error('disk full');}stored=raw;},send:async w=>{saves++;return lesson(w.value.id,'2026-10-24');}});
 await f.queue.load();await f.queue.enqueue(queuedLesson());await assert.rejects(()=>f.queue.flush(),/服务器已确认保存/);assert.equal(decodeOutbox(stored).entries.q.status,'uncertain');await f.queue.flush();assert.equal(saves,2);assert.equal(f.queue.value.entries.q.status,'confirmed');
});

import {decodeObservationDrafts,normalizedObservation,measurementText,measurementSegment} from '../apps/client/src/prd-observations.ts';
test('measurement inputs preserve unfinished numbers locally, never invent rates for zero opportunities and split changed conditions',()=>{
 const w={id:'m',operation_id:'op',expected_version:null,kind:'measurement',action:'save',note:'',input:{metric_id:'metric',metric_version:1,business_date:'2026-10-24',support:'示范一次',text:'',opportunities:'0',achieved:'0',amount:null,source:null}};
 const pending=normalizedObservation(w);assert.equal(pending.input.opportunities,0);const draft={m:{command:w,pending}};assert.deepEqual(decodeObservationDrafts(JSON.stringify(draft)),draft);
 const record={id:'m',input:pending.input,metric:{id:'metric',version:1,input:{template:'count'}}};assert.equal(measurementText(record),'未计算做到比例（没有机会或未记录机会数）');assert.equal(measurementText({...record,input:{...record.input,opportunities:5,achieved:3}}),'做到 3／5 次，60%（该次机会内的比例）');
 assert.notEqual(measurementSegment(record),measurementSegment({...record,input:{...record.input,support:'不提醒'}}));assert.notEqual(measurementSegment(record),measurementSegment({...record,metric:{...record.metric,version:2}}));
 const unfinished={...w,input:{...w.input,opportunities:'.'}};assert.deepEqual(decodeObservationDrafts(JSON.stringify({m:{command:unfinished}})).m.command,unfinished);assert.throws(()=>normalizedObservation(unfinished));
 for(const malformed of ['[]','null',JSON.stringify({m:{command:{...w,input:{}}}}),JSON.stringify({m:{command:w,pending:{...pending,input:{...pending.input,achieved:4}}}})])assert.throws(()=>decodeObservationDrafts(malformed));
});
test('explicit duplicate-catalog resolution rebinds only provably unsent lessons and keeps linked receipt',async()=>{
 const f=outboxFixture();const original=queuedCatalog('offline-course','course'),record=queuedLesson('linked-lesson','offline-course');
 f.io.send=async w=>{f.sent.push(structuredClone(w));if(w.kind==='catalog')throw Object.assign(new Error('duplicate name'),{status:409});return lesson(w.value.id,'2026-10-24');};await f.queue.load();await f.queue.enqueue(original);await f.queue.enqueue(record);await f.queue.flush();assert.equal(f.queue.value.entries['offline-course'].status,'blocked');assert.equal(f.queue.value.entries['linked-lesson'].attempted,false);
 await f.queue.resolveCatalog('offline-course','c');assert.equal(f.queue.value.entries['offline-course'].status,'linked');assert.equal(f.queue.value.entries['linked-lesson'].status,'confirmed');assert.equal(f.sent.at(-1).value.action.value.course_id,'c');assert.notEqual(f.sent.at(-1).value.operation_id,record.value.operation_id);assert.equal(f.sent.filter(w=>w.kind==='lesson').length,1);assert.doesNotThrow(()=>decodeOutbox(f.raw()));
});
test('ambiguous dependent lessons cannot be rebound, and malformed persisted receipts fail closed',async()=>{
 const f=outboxFixture({send:async()=>{throw Object.assign(new Error('invalid'),{status:422});}});await f.queue.load();await f.queue.enqueue(queuedCatalog('offline-course','course'));await f.queue.enqueue(queuedLesson('uncertain','offline-course'),true);await f.queue.flush();await assert.rejects(()=>f.queue.resolveCatalog('offline-course','c'),/曾尝试上传/);
 const malformed={schema:'skarmar.lesson-queue.v1',entries:{q:{write:queuedLesson(),status:'confirmed',reason:'',receipt:{id:'q',version:1,status:'recorded'}}}};assert.throws(()=>decodeOutbox(JSON.stringify(malformed)));
});
test('cancelling untouched lessons retains shared catalog dependencies then removes only final orphans',async()=>{
 const f=outboxFixture();await f.queue.load();f.offline();
 await f.queue.enqueue(queuedCatalog('offline-course','course'));await f.queue.enqueue(queuedCatalog('offline-teacher','teacher'));
 await f.queue.enqueue(queuedLesson('first','offline-course','offline-teacher'));await f.queue.enqueue(queuedLesson('second','offline-course','offline-teacher'));
 assert.deepEqual(await f.queue.cancel('first'),['first']);let entries=decodeOutbox(f.raw()).entries;
 assert.deepEqual(Object.keys(entries).sort(),['offline-course','offline-teacher','second']);
 const removed=await f.queue.cancel('second');assert.deepEqual(new Set(removed),new Set(['second','offline-course','offline-teacher']));
 entries=decodeOutbox(f.raw()).entries;assert.deepEqual(entries,{});assert.equal(f.sent.length,0);
});
test('catalog dependencies and uncertain or blocked writes cannot be discarded as local cancellations',async()=>{
 const f=outboxFixture();await f.queue.load();f.offline();await f.queue.enqueue(queuedCatalog('offline-course','course'));await f.queue.enqueue(queuedLesson('dependent','offline-course'));
 const before=f.raw();await assert.rejects(()=>f.queue.cancel('offline-course'),/正被待上传课记使用/);assert.equal(f.raw(),before);
 await f.queue.enqueue(queuedLesson('uncertain'),true);const ambiguous=f.raw();await assert.rejects(()=>f.queue.cancel('uncertain'),/必须先核对/);assert.equal(f.raw(),ambiguous);
 const blocked=outboxFixture({identity:async()=>({user:{id:'other',active:true},space:'home',demo:false})});await blocked.queue.load();await blocked.queue.enqueue(queuedLesson());await blocked.queue.flush();assert.equal(blocked.queue.value.entries.q.status,'blocked');await assert.rejects(()=>blocked.queue.cancel('q'),/必须先核对/);
});

import {buildBrief} from '../apps/client/src/prd-brief.ts';
import {coverageItems,coverageLegend} from '../apps/client/src/prd-coverage.ts';
import {knowledgeForGoal,knowledgeRelation,knowledgeSource} from '../apps/client/src/knowledge-relations.ts';
import {planContext} from '../apps/client/src/plan-context.ts';
test('planning reference uses historical scope and only offers available or explicitly retained actions',()=>{
 const goal=id=>({id,version:1,input:{...newGoal(),title:id}}),child=goal('child'),old=goal('old'),other=goal('other');
 const record={id:'session',version:1,status:'completed',created_at:'2026-10-01T00:00:00Z',input:{business_date:'2026-10-01'},focus_goals:[],actual_targets:[{goal:child,ancestors:[old],input:{note:'保留的对应观察'}},{goal:other,ancestors:[],input:{note:'OTHER_TARGET_NOTE'}}]};
 const action=(id,status='pending',plan_id=null)=>({input:{id,content:id,goal_ids:[],method_ids:['m']},status,plan_id});
 const data={...emptySnapshot,goals:[{...child,input:{...child.input,parent_id:'new'}},old],methods:[{id:'m',version:1,input:{...newMethod(),goal_ids:['old']}}],sessions:[record,{...record,id:'future',input:{business_date:'2026-10-25'}},{...record,id:'gone',status:'deleted'}],actions:[action('available'),action('mine','in_plan','this-plan'),action('occupied','in_plan','other-plan'),action('cancelled','cancelled')]};
 const context=planContext(data,'old','2026-10-24','this-plan');assert.equal(context.family.record.id,'session');assert.equal(context.family.note,'保留的对应观察');assert.deepEqual(context.actions.map(a=>a.input.id),['available','mine']);assert.equal(planContext(data,'new','2026-10-24','this-plan').family,undefined);
 assert.deepEqual(planContext(data,'old','2026-10-24','this-plan',['occupied','cancelled']).actions.map(a=>a.input.id),['available','mine','occupied','cancelled']);
 const focusOnly={...record,actual_targets:[record.actual_targets[1]],focus_goals:[old]};assert(!planContext({...data,sessions:[focusOnly]},'old','2026-10-24','this-plan').family.note.includes('OTHER_TARGET_NOTE'));
});
test('coverage charts keep one report, fixed bins and separate long focus from descendant evidence',()=>{
 const goal=(id,level)=>({id,version:1,input:{...newGoal(),title:id,level}}),data={...emptySnapshot,goals:[goal('a','short'),goal('b','short'),goal('m','medium'),goal('l','long')]};
 const counts=(family,lesson)=>({family,lesson,selected:family+lesson}),row=(id,n)=>({id,counts:counts(n,1),direct:counts(n,1),descendants:counts(5,3),focus:counts(1,1),band:2,last_date:null,attention_anchor:'2026-10-01'});
 const report={source:'combined',goals:[row('a',1),{...row('b',0),counts:counts(0,0)},row('m',3),row('l',4)],methods:{method:counts(1,1)},tags:{游戏:2,阅读:2}};
 assert.deepEqual(coverageItems(data,report,'goals','short').map(r=>r.id),['a','b']);assert.deepEqual(coverageItems(data,report,'goals','short',true).map(r=>r.id),['b']);assert.equal(coverageItems(data,report,'goals','long')[0].counts.selected,2);assert.equal(coverageItems(data,report,'goals','long')[0].band,undefined);assert(coverageItems(data,report,'goals','long')[0].detail.includes('家庭5／机构3'));
 assert.equal(coverageItems(data,report,'methods','short')[0].counts.selected,2);assert.equal(coverageItems(data,report,'tags','long').reduce((n,r)=>n+r.counts.selected,0),4);assert.equal(coverageLegend(30),'0／1／2–3／4–7／8+');assert.equal(coverageLegend(7),'0／1／2／3–4／5+');
 const zeroMethods={...data,methods:[{id:'unused',input:{...newMethod(),title:'还未实践'}},{id:'retired',input:{...newMethod(),state:'retired'}}]};assert.equal(coverageItems(zeroMethods,report,'methods','short').find(r=>r.id==='unused').counts.selected,0);assert(!coverageItems(zeroMethods,report,'methods','short').some(r=>r.id==='retired'));
 assert.equal(coverageItems(data,{...report,unclassified:2},'tags','short').find(r=>r.id==='untagged').counts.selected,2);
});
test('knowledge applicability is deduplicated while direct, via-method and source relationships stay explicit',()=>{
 const k={input:{id:'k',kind:'experience',content:'原观察',goal_ids:['g'],method_ids:['m']},deleted:false,source_session_id:'gone',source_session_version:2};
 const data={...emptySnapshot,methods:[{id:'m',version:1,input:{...newMethod(),title:'示范',goal_ids:['g']}}],knowledge:[k,{...k},{...k,input:{...k.input,id:'deleted'},deleted:true}],sessions:[{id:'gone',status:'deleted'}]};
 const rows=knowledgeForGoal(data,'g');assert.equal(rows.length,1);assert(rows[0].direct);assert.equal(rows[0].via[0].id,'m');assert.equal(knowledgeRelation(data,k,'g'),'直接关联本目标；经方法关联：示范');assert(knowledgeSource(data,k).includes('来源记录已删除'));
 const indirect={...k,input:{...k.input,goal_ids:[]}};assert.equal(knowledgeForGoal({...data,knowledge:[indirect]},'g')[0].direct,false);assert.equal(knowledgeRelation(data,indirect,'g'),'经方法关联：示范');assert(!knowledgeSource({...data,sessions:[]},k).includes('已删除'));
});
test('long-focus-only professional brief never includes unselected target observations',()=>{
 const g=(id,level='short')=>({id,version:1,input:{...newGoal(),title:id,level}}),long=g('长期方向','long'),other=g('未选目标');
 const session={id:'s',status:'completed',input:{business_date:'2026-10-24',summary:{signals:[{kind:'主动开始',goal_ids:[long.id]},{kind:'背景_不应导出',goal_ids:[]}]}},actual_targets:[{goal:other,ancestors:[],input:{note:'PRIVATE_UNSELECTED'}}],focus_goals:[long]};
 const data={...emptySnapshot,goals:[long,other],sessions:[session]},options={from:'2026-10-01',to:'2026-10-31',goalIds:[long.id],parts:['family']};
 const text=buildBrief(data,[],options);assert(text.includes('明确长期关注：长期方向'));assert(text.includes('主动开始'));assert(!text.includes('PRIVATE_UNSELECTED'));assert(!text.includes('背景_不应导出'));assert(!text.includes(other.id));
 const child={...session,actual_targets:[{...session.actual_targets[0],ancestors:[long],input:{note:'明确下级表现'}}]};assert(buildBrief({...data,sessions:[child]},[],options).includes('明确下级表现'));
});
test('professional brief uses explicit scope and calendar dates, and shares needs only after selection',()=>{
 const g={id:'g',version:1,input:{...newGoal(),title:'所选目标'}},data={...emptySnapshot,goals:[g]};
 const records=[{id:'moment',kind:'moment',input:{business_date:'2026-10-24',text:'保留的观察',goal_ids:['g']},deleted:false},{id:'other',kind:'moment',input:{business_date:'2026-10-24',text:'OTHER_TARGET',goal_ids:['other']},deleted:false},{id:'needs',kind:'needs',input:{health:'PRIVATE_NEEDS'},deleted:false},{id:'measurement',kind:'measurement',goal:g,metric:{id:'metric',version:1,input:{template:'count',criterion:'五次机会'}},input:{business_date:'2026-10-24',opportunities:0,achieved:0,support:'提醒一次'},deleted:false}];
 const options={from:'2026-10-01',to:'2026-10-31',goalIds:['g'],parts:['moment','measurement']},text=buildBrief(data,records,options);
 assert(text.includes('保留的观察'));assert(text.includes('未计算做到比例'));assert(!text.includes('OTHER_TARGET'));assert(!text.includes('PRIVATE_NEEDS'));assert(buildBrief(data,records,{...options,parts:['needs']}).includes('PRIVATE_NEEDS'));assert.throws(()=>buildBrief(data,records,{...options,from:'2026-02-31'}));
});
test('lesson settlement excludes a future record while retaining marked errors inside the selected range',()=>{
 const records=[{...lesson('actual','2026-10-24'),marked_error:true},lesson('future','2026-10-25')];const totals=teacherTotals(records,'2026-10-01','2026-10-31','2026-10-24');assert.equal(totals[0].count,1);assert.equal(totals[0].marked,1);
});

import {measurementSeries,measureValue} from '../apps/client/src/prd-progress.ts';
import {dailyDistribution} from '../apps/client/src/prd-history.ts';
import {groupReviewQueue} from '../apps/client/src/review-queue.ts';
test('measurement graphs preserve missing denominators, split definitions/conditions/method versions, and never score text',()=>{
 const goal={id:'g',version:1,input:newGoal()},metric={id:'metric',version:1,input:{name:'轮替机会',template:'count',unit:''}};
 const measurement=(id,date,n,k,support='一次提醒',version=1)=>({id,kind:'measurement',deleted:false,goal,metric:{...metric,version},input:{business_date:date,opportunities:n,achieved:k,support,source:null},source_snapshot:null});
 const a=measurement('a','2026-10-20',5,3),b=measurement('b','2026-10-21',0,0),c=measurement('c','2026-10-22',5,4,'不提醒'),d=measurement('d','2026-10-23',5,5,'一次提醒',2);
 assert.equal(measureValue(a),60);assert.equal(measureValue(b),null);assert.equal(measureValue({...a,input:{...a.input,achieved:6}}),null);
 const grouped=measurementSeries([d,c,b,a,{...a,id:'deleted',deleted:true},{...a,id:'future',input:{...a.input,business_date:'2026-10-25'}}],'2026-10-24');assert.equal(grouped.length,3);const first=grouped.find(g=>g.support==='一次提醒'&&g.metricVersion===1);assert.deepEqual(first.points.map(p=>p.value),[60,null]);assert.equal(first.points.length,2);
 const method=(id,version)=>({id,version,input:{title:id}}),source={actual_methods:[method('m',1),method('n',2)]};
 const same=measurementSeries([{...a,source_snapshot:source},{...b,source_snapshot:{actual_methods:[method('n',2),method('m',1)]}},{...c,input:{...c.input,support:'一次提醒'},source_snapshot:{actual_methods:[method('m',2),method('n',2)]}}],'2026-10-24');assert.equal(same.length,2);assert.equal(same.find(g=>g.methodBasis.includes('m 第1版')).points.length,2);
 const text={...a,metric:{...metric,input:{...metric.input,template:'steps'}},input:{...a.input,text:'自己完成了打开水龙头，涂皂需示范'}};assert.equal(measureValue(text),null);assert(measurementSeries([text],'2026-10-24')[0].points[0].label.includes('需示范'));
});
test('daily distributions retain source counts at 90-day boundaries and exclude future dates',()=>{
 const rows=[{id:'same',kind:'family',date:'2026-10-24'},{id:'same',kind:'lesson',date:'2026-10-24'},{id:'boundary',kind:'family',date:'2026-07-27'},{id:'old',kind:'family',date:'2026-07-26'},{id:'outside',kind:'family',date:'2026-07-25'},{id:'future',kind:'family',date:'2026-10-25'}];
 const days=dailyDistribution(rows,'2026-10-24');assert.equal(days.length,90);assert.equal(days[0].date,'2026-07-27');assert.deepEqual(days.at(-1),{date:'2026-10-24',family:1,lesson:1,count:2});assert.equal(days[0].count,1);assert.equal(days.reduce((n,d)=>n+d.count,0),3);
});
test('review queue groups shared methods once, retains other goals and does not mutate the source order',()=>{
 const goals=['甲目标','乙目标'].map((title,i)=>({id:'g'+i,version:1,input:{...newGoal(),title}})),data={...emptySnapshot,goals,methods:[{id:'m',version:1,input:{...newMethod(),title:'共同方法',goal_ids:['g0','g1']}}],knowledge:[{input:{id:'k',kind:'experience',content:'经验',goal_ids:[],method_ids:['m']}}]};
 const schedule=(kind,id)=>({object_kind:kind,object_id:id,version:1,generation:1,due_date:'2026-10-24'}),a=[schedule('method','m'),schedule('goal','g1'),schedule('knowledge','k'),schedule('method','m')],before=JSON.stringify(a),groups=groupReviewQueue(a,data),items=groups.flatMap(g=>g.items);
 assert.equal(items.length,3);assert.equal(items.filter(i=>i.schedule.object_kind==='method').length,1);assert.equal(items.find(i=>i.schedule.object_kind==='method').otherGoals.length,1);assert.equal(JSON.stringify(a),before);
});

import {planningKnowledge,goalChoiceGroups,recommendationReason} from '../apps/client/src/planning-context.ts';
import {fieldMatrix,guidanceFor,observationExamples} from '../apps/client/src/field-guidance.ts';
import {methodUsage} from '../apps/client/src/method-usage.ts';
import {previewContext,seenEvidence,startReviewBatch,batchResult,decodeReviewBatch,nextReviewDate} from '../apps/client/src/review-flow.ts';
test('planning excludes closed knowledge, favors explicit key flags and deduplicates goal groups',()=>{
 const goals=['a','b','c','d'].map((id,i)=>({id,version:1,input:{...newGoal(),title:id,level:i===3?'long':i===2?'medium':'short'}}));
 const knowledge=['following','resolved','paused','active','retired','pending'].map((state,i)=>({input:{id:'k'+i,kind:i<3||i===5?'difficulty':'experience',goal_ids:['a'],method_ids:[]},details:{state,key:i===5}}));
 const data={...emptySnapshot,goals,knowledge};assert.deepEqual(planningKnowledge(data,'a').map(k=>k.input.id),['k5','k0','k3']);
 const report={suggested_ids:['a','a'],goals:[{id:'a',last_date:null,attention_anchor:'2020-01-01',ratio:null},{id:'b',last_date:'2020-02-02',ratio:2}]};
 const groups=goalChoiceGroups(data,report,'');assert.deepEqual(groups.flatMap(g=>g.goals.map(g=>g.id)),['a','b','c']);assert.match(recommendationReason(goals[0],report),/尚无记录/);assert.match(recommendationReason(goals[1],report),/14天/);assert.equal(goalChoiceGroups(data,report,'absent').length,0);
});
test('guidance retains persistent questions, short placeholders and full non-value examples for all goal templates',()=>{
 for(const key of ['count','duration','steps','text']){const g=guidanceFor('实际表现：目标',observationExamples[key]);assert.ok(g.question);assert.ok(Array.from(g.shortExample).length<=15);assert.equal(g.fullExample,observationExamples[key]);assert.match(g.fullExample,/例如/);}
 assert.match(guidanceFor('本领名称').question,/帮助孩子/);assert.equal(guidanceFor('搜索知识').fullExample,'');
 assert.equal(guidanceFor('预期帮助条件（可选）').question,'准备在什么帮助条件下观察？');
 assert.equal(guidanceFor('预期帮助条件（可选）').fullExample,'例如：先不提示，需要时再示范');
 assert.equal(fieldMatrix['goal.domain'].example,'选择或自定义');assert.equal(fieldMatrix['goal.due'].question,'预计何时回看目标？');
 assert.deepEqual(guidanceFor('实际情况／必填 1'),{question:'大人怎样回应，后来呢？',shortExample:'例如：换玩具后离开了',fullExample:'例如：换玩具后离开了'});
 assert.equal(guidanceFor('成人回应／可选 1').fullExample,'例如：先回应他的手势，再示范小步骤');
 assert.equal(guidanceFor('原因猜测／可选 1').question,'有什么尚未确定的猜测？');
 assert.equal(guidanceFor('本次条件／可选 1').question,'在什么场景和帮助条件下观察？');
 assert.deepEqual(guidanceFor('具体做法／必填'),{question:'大人具体怎样帮助？',shortExample:'例如：先跟着他的玩法',fullExample:'例如：先跟着他的玩法'});
 assert.equal(guidanceFor('观察意图／可选').fullExample,'例如：愿意再试一次');
 assert.equal(guidanceFor('适用条件／可选').question,'什么时候适合使用？');
 assert.deepEqual(guidanceFor('新方法名称','例如：示范后少提醒'),{question:'怎样称呼这个做法？',shortExample:'例如：示范后少提醒',fullExample:'例如：示范后少提醒'});
});
test('method usage counts actual sources and immutable versions, excluding planned deleted and future records',()=>{
 const snap=(v)=>({id:'m',version:v,input:{title:'历史方法'+v}});
 const session=(id,day,v,status='completed')=>({id,status,created_at:day,input:{business_date:day},actual_targets:[],actual_methods:[snap(v)],focus_goals:[]});
 const data={...emptySnapshot,sessions:[session('s','2020-01-01',1),session('planned','2020-01-01',2,'planned'),session('future','2030-01-01',2)],lessons:[{id:'l',status:'recorded',created_at:'2020-01-02',observation:{business_date:'2020-01-02'},organization:{targets:[],focus_goals:[],methods:[snap(2)]}}]};
 const u=methodUsage(data,'m','2020-02-01');assert.equal(u.family,1);assert.equal(u.lesson,1);assert.equal(u.last,'2020-01-02');assert.deepEqual(u.versions.map(v=>v.version),[2,1]);assert.equal(u.versions[0].title,'历史方法2');
});
test('three-evidence preview consumes only displayed sources and their annotations',()=>{
 const evidence=Array.from({length:5},(_,i)=>({source:{kind:'feedback',id:'f'+i,version:i+1},seen_versions:{['feedback:f'+i]:i+1,['annotation:a'+i]:1}}));
 const c=previewContext({evidence,seen_versions:seenEvidence(evidence),has_more:false});assert.equal(c.evidence.length,3);assert.equal(c.has_more,true);assert.equal(c.server_more,false);assert.equal(c.buffer.length,5);assert.equal(Object.keys(c.seen_versions).length,6);assert.equal(c.seen_versions['feedback:f3'],undefined);assert.equal(c.seen_versions['annotation:a3'],undefined);
});
test('review batch resumes exact generations, defers without completion and preserves receipts and default dates',()=>{
 const schedules=[{object_kind:'goal',object_id:'g',generation:2,due_date:'2020-01-01'}];const data={...emptySnapshot,goals:[{id:'g',input:{...newGoal(),title:'目标',level:'short'}}]};
 const batch=startReviewBatch([...schedules,...schedules],data);assert.equal(batch.items.length,1);assert.deepEqual(decodeReviewBatch(JSON.stringify(batch)),batch);
 const r={object_kind:'goal',object_id:'g',generation:2,action:'defer',note:'稍后一起看',next_date:'2020-01-04',reminders:true,decision:'continue'};
 assert.equal(batchResult(batch,r,{}).items[0].state,'deferred');assert.equal(batchResult(batch,{...r,generation:3},{}).items[0].state,'pending');
 const done=batchResult(batch,{...r,action:'complete'},{review:{id:'r',decision:'adjust',note:'换个小步骤',action_ids:['a']},schedule:{due_date:'2020-01-18'}});assert.equal(done.items[0].reviewId,'r');assert.deepEqual(done.items[0].actionIds,['a']);assert.equal(done.items[0].nextDate,'2020-01-18');
 const c={next_defaults:{weekly:'2020-01-04',fortnightly:'2020-01-18',monthly:'2020-02-01',current:'2020-01-18'}};
 assert.equal(nextReviewDate({...r,next_date:null},c,data),'2020-01-18');assert.equal(nextReviewDate({...r,decision:'maintain',next_date:null},c,data),'2020-01-18');assert.equal(nextReviewDate({...r,decision:'achieve'},c,data),null);assert.equal(nextReviewDate({...r,reminders:false},c,data),null);
 assert.throws(()=>decodeReviewBatch('{broken'));assert.throws(()=>decodeReviewBatch(JSON.stringify({...batch,items:[{...batch.items[0],generation:0}]})));
});

import {restoreReviewContext,revealBuffered} from '../apps/client/src/review-context.ts';
import {reconcileReviewBatch} from '../apps/client/src/review-flow.ts';
import {objectActions,objectEvidence} from '../apps/client/src/object-context.ts';
import {calendarCounts} from '../apps/client/src/prd-sessions.ts';
test('review previews follow server calendar cadence for maintained, restored and archived goals',()=>{
 const data={...emptySnapshot,goals:[{id:'short',input:{...newGoal(),level:'short'}},{id:'long',input:{...newGoal(),level:'long'}},{id:'archived',input:{...newGoal(),archived:true}}]};
 const c={next_defaults:{weekly:'2020-01-04',fortnightly:'2020-01-18',monthly:'2020-02-01',current:null}};
 const r={object_kind:'goal',object_id:'short',reminders:true,decision:'maintain',next_date:null};
 assert.equal(nextReviewDate(r,c,data),'2020-01-18');assert.equal(nextReviewDate({...r,object_id:'long'},c,data),'2020-02-01');assert.equal(nextReviewDate({...r,object_id:'archived',next_date:'2020-01-07'},c,data),null);assert.equal(nextReviewDate({...r,decision:'restore',state:'maintaining'},c,data),'2020-01-18');assert.equal(nextReviewDate({...r,decision:'pause'},c,data),null);
});
test('resumed review restores previously opened pages and never acknowledges newly arrived evidence',async()=>{
 const e=n=>({source:{kind:'moment',id:'e'+n,version:1},seen_versions:{['moment:e'+n]:1}});
 const rows=Array.from({length:61},(_,i)=>e(i)),saved=Object.fromEntries(rows.slice(31).map(e=>['moment:'+e.source.id,1]));
 const page=n=>({evidence:rows.slice(n*30,n*30+30),page:n,has_more:n<2,evidence_signature:'stable',schedule:{version:1},object_version:1});const reads=[];
 const c=await restoreReviewContext({seen_versions:saved},async n=>{reads.push(n);return page(n)});
 assert.deepEqual(reads,[0,1,2]);assert.equal(c.evidence.length,30);assert.equal(c.buffer.length,61);assert.equal(c.seen_versions['moment:e0'],undefined);assert.equal(c.page,2);
 const more=revealBuffered(c);assert.equal(more.evidence.length,60);assert.equal(more.has_more,true);assert.equal(more.seen_versions['moment:e30'],undefined);assert.equal(revealBuffered(more).evidence.length,61);assert.equal(revealBuffered(more).has_more,false);
 await assert.rejects(()=>restoreReviewContext({seen_versions:saved},async n=>n?{...page(n),evidence_signature:'changed'}:page(0)),/依据或安排有更新/);assert.equal(Object.keys(saved).length,30);
});
test('review batch refresh reflects corrected or deleted receipts and fails without overwriting progress',async()=>{
 const item={kind:'goal',id:'g',generation:1,due:'2020-01-01',title:'目标',state:'completed',reviewId:'r',note:'旧说明',actionIds:['a']};const batch={schema:'skarmar.review-batch.v1',started:'2020-01-01',items:[item]};
 const r={id:'r',object_kind:'goal',object_id:'g',generation:1,version:2,deleted:false,note:'更正后的说明',decision:'continue',action_ids:['a']};
 const updated=await reconcileReviewBatch(batch,[],async()=>{throw new Error('unused')},async()=>r);assert.equal(updated.items[0].note,'更正后的说明');assert.equal(batch.items[0].note,'旧说明');
 const deleted=await reconcileReviewBatch(updated,[],async()=>{throw new Error('unused')},async()=>({...r,deleted:true}));assert.equal(deleted.items[0].state,'changed');assert.deepEqual(deleted.items[0].actionIds,[]);
 assert.equal(batchResult(batch,{action:'revise'},{...r,note:'立即更正'}).items[0].note,'立即更正');assert.equal(batchResult(batch,{action:'delete'},{...r,deleted:true}).items[0].state,'changed');
 await assert.rejects(()=>reconcileReviewBatch(updated,[],async()=>null,async()=>{throw new Error('offline')}),/offline/);assert.equal(updated.items[0].state,'completed');
 const pending={...batch,items:[{...item,state:'pending',reviewId:undefined}]};const shared=await reconcileReviewBatch(pending,[],async()=>({last_review:r,schedule:{due_date:'2020-01-18'}}),async()=>r);assert.equal(shared.items[0].state,'completed');assert.equal(shared.items[0].reviewId,'r');
});
test('object details scope actual observations to historical targets and long focus, with explicit action links',()=>{
 const target=(id,note,ancestors=[])=>({goal:{id,input:{title:id}},input:{note},ancestors});
 const s={id:'s',status:'completed',created_at:'2020-01-01',input:{business_date:'2020-01-01',time_slot:'晚上'},actual_targets:[target('a','ONLY_A',[{id:'parent'}]),target('b','OTHER_B')],actual_methods:[{id:'m'}],focus_goals:[{id:'focus'}]};
 const l={id:'l',status:'recorded',created_at:'2020-01-02',observation:{business_date:'2020-01-02',notes:'WHOLE_CLASS'},organization:{targets:[{goal:{id:'a',input:{parent_id:'parent'}},ancestors:[{id:'parent'}],target:{note:'CLASS_A'}},{goal:{id:'b',input:{parent_id:null}},ancestors:[],target:{note:'CLASS_B'}}],methods:[{id:'m'}],focus_goals:[]}};
 const data={...emptySnapshot,sessions:[s],lessons:[l],methods:[{id:'m',input:{goal_ids:['a']}}],actions:[{input:{id:'a1',goal_ids:[],method_ids:['m']}},{input:{id:'a2',goal_ids:['b'],method_ids:[]}}]};
 assert.deepEqual(objectEvidence(data,{kind:'goal',id:'a'},'2020-02-01').map(e=>e.note),['CLASS_A','ONLY_A']);assert.doesNotMatch(objectEvidence(data,{kind:'goal',id:'focus'},'2020-02-01')[0].note,/ONLY_A|OTHER_B/);assert.equal(objectEvidence(data,{kind:'method',id:'m'},'2020-02-01')[0].contextOnly,true);assert.deepEqual(objectActions(data,{kind:'goal',id:'a'}).map(a=>a.input.id),['a1']);assert.deepEqual(objectEvidence(data,{kind:'goal',id:'parent'},'2020-02-01').map(e=>e.note),['CLASS_A','ONLY_A']);
});
test('calendar separates completed practice from pending and future plans',()=>{
 const s=(id,date,status)=>({id,status,input:{business_date:date}});const c=calendarCounts([s('a','2020-01-01','completed'),s('b','2020-01-01','planned'),s('c','2020-01-02','summary_draft'),s('d','2030-01-01','planned'),s('e','2030-01-01','completed'),s('f','2020-01-01','deleted')],'2020-02-01');
 assert.deepEqual(c.completed,{'2020-01-01':1});assert.deepEqual(c.pending,{'2020-01-01':1,'2020-01-02':1,'2030-01-01':1});
});

import {DraftStore,DraftConflictError,draftText} from '../apps/client/src/draft-store.ts';
function draftDisk(initial={}){
 const disk={raw:JSON.stringify(initial),backups:null,sequence:0,writes:0,fail:false,chain:Promise.resolve(),backupGate:null};
 const decode=raw=>{const v=raw===null?{}:JSON.parse(raw);if(!v||Array.isArray(v)||typeof v!=='object'||Object.values(v).some(x=>!x||typeof x!=='object'||typeof x.text!=='string'))throw new Error('invalid draft map');return v;};
 const io=()=>({read:async()=>disk.raw,write:async raw=>{if(disk.fail)throw new Error('disk full');disk.raw=raw;disk.writes++;},readBackups:async()=>disk.backups,writeBackups:async raw=>{if(disk.backupGate)await disk.backupGate;if(disk.fail)throw new Error('disk full');disk.backups=raw;},lock:fn=>{const result=disk.chain.catch(()=>undefined).then(fn);disk.chain=result.catch(()=>undefined);return result;},id:()=>String(++disk.sequence),now:()=> '2026-10-25T00:00:00Z',changed:()=>{}});
 return {...disk,get disk(){return disk;},make:()=>new DraftStore(decode,decode(disk.raw),io()),read:()=>decode(disk.raw)};
}
test('draft windows merge only changed records and never remove another window\'s unrelated work',async()=>{
 const d=draftDisk(),a=d.make(),b=d.make();await Promise.all([a.save({a:{text:'A'}}),b.save({b:{text:'B'}})]);assert.deepEqual(d.read(),{a:{text:'A'},b:{text:'B'}});
 await a.save({a:{text:'A2'}});assert.equal(d.read().b.text,'B');await a.save({});assert.deepEqual(d.read(),{b:{text:'B'}});
 await b.save({b:{text:'B2'}});assert.deepEqual(d.read(),{b:{text:'B2'}});
});
test('same-record draft conflict preserves both texts across reload and restores only on explicit selection',async()=>{
 const d=draftDisk({g:{text:'initial'}}),a=d.make(),b=d.make();await a.save({g:{text:'A'}});await assert.rejects(()=>b.save({g:{text:'B'}}),DraftConflictError);assert.equal(d.read().g.text,'A');
 await assert.rejects(()=>b.save({g:{text:'B2'},other:{text:'also preserved'}}),DraftConflictError);const copies=await b.backups();assert.equal(Object.keys(copies.items).length,2);assert(copies.items&&Object.values(copies.items).some(v=>v.local.text==='B2'&&v.remote.text==='A'));
 const c=d.make(),backup=Object.entries((await c.backups()).items).find(([,v])=>v.local?.text==='B2')[0];const restored=await c.restore(backup);assert.equal(restored.value.text,'B2');assert.equal(d.read().g.text,'B2');assert(Object.values((await c.backups()).items).some(v=>v.local?.text==='A'));
 await assert.rejects(()=>a.save({g:{text:'late A'}}),DraftConflictError);assert.equal(d.read().g.text,'B2');
});
test('draft reconciliation backs up local input, never changes pending request bytes, and leaves server operations to the form',async()=>{
 const pending={id:'g',operation_id:'exact-original',value:{text:'payload'}};const d=draftDisk({g:{text:'initial'}}),a=d.make(),b=d.make();await a.save({g:{text:'saved in A'}});await assert.rejects(()=>b.save({g:{text:'B pending',pending}}),DraftConflictError);
 const latest=await b.adopt();assert.equal(latest.g.text,'saved in A');const backups=await b.backups(),local=Object.entries(backups.items).find(([,v])=>v.local?.pending)[0];await b.restore(local);assert.deepEqual(d.read().g.pending,pending);assert.equal(d.read().g.text,'B pending');
});
test('failed draft storage does not advance its base and a later retry persists the latest input',async()=>{
 const d=draftDisk({g:{text:'initial'}}),a=d.make();d.disk.fail=true;await assert.rejects(()=>a.save({g:{text:'not saved'}}),/disk full/);assert.equal(d.read().g.text,'initial');d.disk.fail=false;await a.save({g:{text:'latest'}});assert.equal(d.read().g.text,'latest');
 const writes=d.disk.writes;d.disk.raw='malformed';await assert.rejects(()=>a.save({g:{text:'keep'}}));assert.equal(d.disk.raw,'malformed');assert.equal(d.disk.writes,writes);
});
test('stale windows cannot resurrect drafts removed by a successful submission or drop concurrent edits during adoption',async()=>{
 const d=draftDisk({g:{text:'initial'}}),a=d.make(),b=d.make();await a.save({});await assert.rejects(()=>b.save({g:{text:'late'}}),DraftConflictError);assert.deepEqual(d.read(),{});
 let release;d.disk.backupGate=new Promise(r=>release=r);const adopting=b.adopt();await new Promise(r=>setTimeout(r,0));const saving=b.save({g:{text:'typed while adopting'}});release();d.disk.backupGate=null;await assert.rejects(()=>adopting,/填写有更新/);await assert.rejects(()=>saving,DraftConflictError);assert(Object.values((await b.backups()).items).some(v=>v.local?.text==='typed while adopting'));
});
test('draft recovery previews show user content and exclude internal keys, pending bodies and credentials',()=>{
 const lines=draftText({id:'internal',operation_id:'op',request:{input:{plan:'练习轮流',business_date:'2026-10-25'}},pending:{input:{plan:'do not duplicate'}},password:'secret',token:'secret'});assert.deepEqual(lines,['安排：练习轮流','日期：2026-10-25']);assert(!lines.join().includes('secret'));assert(!lines.join().includes('internal'));
});

import {resetAfterFailure} from '../apps/client/src/pending-request.ts';
test('uncertain operations survive authentication, contract or local retry failures until an authoritative reply',()=>{
 for(const sent of [true,false])for(const status of [401,403])assert.equal(resetAfterFailure(true,sent,{status}),false);
 for(const message of ['登录账号与当前填写不一致，请重新登录原账号后继续。','账号已变更，原账号的待上传记录已暂停','上传空间不匹配，请用原账号重新打开','页面版本已更新，请保留填写后刷新页面','页面版本与新模型不匹配，请保留填写后刷新页面'])assert.equal(resetAfterFailure(true,true,{status:409,message}),false);
 assert.equal(resetAfterFailure(true,false,new Error('disk full')),false);assert.equal(resetAfterFailure(false,false,new Error('disk full')),true);
 for(const hadPending of [true,false])for(const status of [502,503,500])assert.equal(resetAfterFailure(hadPending,true,{status}),false);
 for(const status of [400,409,413,422])assert.equal(resetAfterFailure(true,true,{status,message:'记录已更新，请核对最新内容'}),true);
 assert.equal(resetAfterFailure(false,true,{status:401}),true);assert.equal(resetAfterFailure(true,true,{status:404}),false);assert.equal(resetAfterFailure(true,true,{status:404},true),true);
});

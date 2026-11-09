import assert from 'node:assert/strict';
import {test} from 'node:test';
import {emptySnapshot,newGoal,newMethod} from '../apps/client/src/foundation.ts';
import {emptySession,startSummary} from '../apps/client/src/prd-sessions.ts';
import {emptyDetails} from '../apps/client/src/prd-followups.ts';
import {summaryKnowledgeIssue,knowledgeTitle,missingSummaryKnowledge} from '../apps/client/src/summary-knowledge.ts';
import {planFocus,recentReminders} from '../apps/client/src/plan-reminders.ts';
import {todayFocus} from '../apps/client/src/object-priority.ts';
const entry=(kind,stage=null)=>({id:kind,kind,stage,content:'a concrete observation',goal_ids:['g'],method_ids:[],next_review_date:null});
test('summary requires explicit experience stage and nonblank difficulty/content/binding, without mutating drafts',()=>{
 const summary=startSummary(emptySession('2026-11-09'));summary.new_knowledge=[entry('experience')];const before=JSON.stringify(summary);assert.match(summaryKnowledgeIssue(summary),/经验阶段/);assert.equal(JSON.stringify(summary),before);
 summary.new_knowledge[0].stage='idea';assert.equal(summaryKnowledgeIssue(summary),'');summary.new_knowledge.push(entry('difficulty'));assert.equal(summaryKnowledgeIssue(summary),'');summary.new_knowledge[1].content=' ';assert.match(summaryKnowledgeIssue(summary),/第2条困难.*内容/);summary.new_knowledge[1]={...entry('difficulty'),goal_ids:[]};assert.match(summaryKnowledgeIssue(summary),/关联目标或方法/);
});
test('reference titles prefer explicit title; old paragraphs become short first-line labels, never rewritten content',()=>{
 const k={input:{...entry('experience','idea'),content:'first line\nprivate detail'},details:{...emptyDetails(),title:'An explicit name'}};assert.equal(knowledgeTitle(k),'An explicit name');k.details.title='';assert.equal(knowledgeTitle(k),'first line');k.input.content='字'.repeat(80);assert.equal(Array.from(knowledgeTitle(k)).length,37);assert.equal(k.input.content.length,80);
});
test('plan priority reminders are exactly the first six of Today, including method bonuses and temporary priority',()=>{
 const goals=Array.from({length:8},(_,i)=>({id:'g'+i,version:1,input:{...newGoal(),level:'long',title:'g'+i,priority_score:i*10}})),method={id:'m',version:1,input:{...newMethod(),title:'m',goal_ids:['g1','g2'],priority_score:99}};
 goals[0].input.priority_until='2026-11-22';const data={...emptySnapshot,goals,methods:[method]};assert.deepEqual(planFocus(data,'2026-11-09'),todayFocus(data,'2026-11-09').slice(0,6));assert.equal(planFocus(data,'2026-11-09')[0].value.id,'g0');assert.equal(planFocus(data,'2026-11-09')[1].kind,'method');assert.equal(planFocus(data,'2026-11-23')[0].kind,'method');
});
test('recent reminders use original creation and family timezone window, retain four categories and omit inactive/deleted/future',()=>{
 const goals=Array.from({length:5},(_,i)=>({id:'g'+i,version:1,input:{...newGoal(),title:'goal'+i}})),methods=[{id:'m',version:1,input:{...newMethod(),title:'method'}}],knowledge=['experience','difficulty'].map(kind=>({input:entry(kind,kind==='experience'?'idea':null),details:emptyDetails(),created_at:'2026-11-09T00:00:00Z',deleted:false}));
 const data={...emptySnapshot,goals,methods,knowledge,created_at:{...Object.fromEntries(goals.map((g,i)=>[g.id,`2026-11-0${i+1}T00:00:00Z`])),m:'2026-10-26T16:00:00Z'}};
 const rows=recentReminders(data,'2026-11-09');assert.equal(rows.filter(r=>r.kind==='goal').length,3);assert.deepEqual(new Set(rows.map(r=>r.kind)),new Set(['goal','method','experience','difficulty']));assert.equal(recentReminders(data,'2026-11-09',true).length,8);
 goals[0].input.archived=true;knowledge[0].deleted=true;data.created_at.g1='2026-11-09T16:00:00Z';data.created_at.g2='2026-10-26T15:59:59Z';assert(!recentReminders(data,'2026-11-09',true).some(r=>['g0','g1','g2','experience'].includes(r.id)));
});

test('completed-session echo alone is not proof of saved knowledge; require shared rows with matching source, skip draft outputs',()=>{
 const summary=startSummary(emptySession('2026-11-09'));summary.new_knowledge=[entry('experience','idea'),entry('difficulty')];const session={id:'s',status:'completed',input:{summary}},data={knowledge:[{input:entry('experience','idea'),source_session_id:'another'},{input:entry('difficulty'),source_session_id:'s'}]};assert.deepEqual(missingSummaryKnowledge(session,data).map(k=>k.id),['experience']);assert.deepEqual(missingSummaryKnowledge({...session,status:'summary_draft'},data),[]);data.knowledge[0].source_session_id='s';assert.deepEqual(missingSummaryKnowledge(session,data),[]);
});

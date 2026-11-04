import type {Snapshot,Goal} from './foundation';
import type {Coverage} from './prd-sessions';
import {goalReferenceKnowledge} from './plan-related-knowledge.ts';
import {recommendationReason} from './planning-context.ts';

export type ReferenceGoal={goal:Goal;reason:string};
/** A short read-only reminder, not a selection or an inferred activity binding. */
export function activityPlanReferences(data:Snapshot,report:Coverage|null){
 const eligible=data.goals.filter(g=>!g.input.archived&&['active','maintaining'].includes(g.input.state??'active'));
 const long=eligible.filter(g=>g.input.level==='long').sort((a,b)=>(b.input.long_priority??1)-(a.input.long_priority??1)||data.goals.indexOf(a)-data.goals.indexOf(b)).slice(0,2);
 const support=(g:Goal)=>goalReferenceKnowledge(data,g.id).some(r=>r.knowledge.input.kind==='difficulty'&&r.knowledge.details?.priority&&r.paths.some(p=>p.kind!=='ancestor'));
 const row=(g:Goal)=>report?.goals.find(r=>r.id===g.id);
 const recent=eligible.filter(g=>g.input.level!=='long'&&g.input.frequency!=='none').sort((a,b)=>Number(support(b))-Number(support(a))||(row(b)?.ratio??-1)-(row(a)?.ratio??-1)||(row(a)?.last_date??'').localeCompare(row(b)?.last_date??'')||a.input.title.localeCompare(b.input.title,'zh-CN')||a.id.localeCompare(b.id)).slice(0,3);
 const result:ReferenceGoal[]=[...long.map(goal=>({goal,reason:'长期关注，随活动顺带留意'})),...recent.map(goal=>({goal,reason:support(goal)?'有家人标记的重点困难，先看看需要哪些支持':recommendationReason(goal,report)}))];
 return result;
}

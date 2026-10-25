import type {Snapshot} from './foundation';
import type {Session} from './prd-sessions';
import type {Lesson} from './prd-lessons';

export type PlanEvidence={kind:'family';date:string;note:string;record:Session}|{kind:'lesson';date:string;note:string;record:Lesson};
/** Planning reads actual historical membership, not today's reparented hierarchy.
 * Missing goal-specific text stays missing; never reuse another target's note. */
export function planContext(data:Snapshot,goalId:string,today:string,planId:string,selectedActions:string[]=[]){
 const family=data.sessions.filter(s=>s.status==='completed'&&s.input.business_date<=today).flatMap(s=>{
  const targets=s.actual_targets.filter(t=>t.goal.id===goalId||t.ancestors.some(g=>g.id===goalId));
  if(!targets.length&&!s.focus_goals.some(g=>g.id===goalId))return [];
  return [{kind:'family' as const,date:s.input.business_date,note:targets.map(t=>t.input.note).filter(Boolean).join('；')||'明确关注过这个方向，未单独填写文字观察',record:s}];
 }).sort((a,b)=>b.date.localeCompare(a.date)||b.record.created_at.localeCompare(a.record.created_at)||a.record.id.localeCompare(b.record.id));
 const lessons=data.lessons.filter(l=>l.status==='recorded'&&l.observation.business_date<=today).flatMap(l=>{
  const targets=l.organization.targets.filter(t=>t.goal.id===goalId||t.ancestors.some(g=>g.id===goalId));
  if(!targets.length&&!l.organization.focus_goals.some(g=>g.id===goalId))return [];
  return [{kind:'lesson' as const,date:l.observation.business_date,note:targets.map(t=>t.target.note).filter(Boolean).join('；')||'已明确关联，未单独填写这个目标的课堂表现',record:l}];
 }).sort((a,b)=>b.date.localeCompare(a.date)||b.record.created_at.localeCompare(a.record.created_at)||a.record.id.localeCompare(b.record.id));
 const methods=data.methods.filter(m=>m.input.goal_ids.includes(goalId)).map(m=>m.id);
 const actions=data.actions.filter(a=>(a.status==='pending'||a.status==='in_plan'&&a.plan_id===planId||selectedActions.includes(a.input.id))&&(a.input.goal_ids.includes(goalId)||a.input.method_ids.some(id=>methods.includes(id))));
 return {family:family[0] as PlanEvidence|undefined,lesson:lessons[0] as PlanEvidence|undefined,actions};
}

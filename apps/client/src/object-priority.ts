import type {Snapshot,Goal,Method} from './foundation';
import type {Schedule} from './prd-followups';
import {historyEvidence,matchesEvidence,daysAgo} from './prd-history.ts';
import {goalReferenceKnowledge} from './plan-related-knowledge.ts';
export type Attention={rank:number;level:'high'|'watch'|'normal'|'low';reasons:string[];stale:boolean};
export const priorityName=(value:number|null|undefined)=>({2:'高关注',1:'普通关注',0:'低关注'}[value??1]??'普通关注');
export const dueReviews=(schedules:Schedule[],today:string)=>schedules.filter(s=>s.reminders&&(s.eligible||s.mode==='resume')&&Boolean(s.due_date&&s.due_date<=today)).sort((a,b)=>(a.due_date??'').localeCompare(b.due_date??'')||a.object_kind.localeCompare(b.object_kind)||a.object_id.localeCompare(b.object_id));
const activeGoal=(g:Goal)=>!g.input.archived&&['active','maintaining'].includes(g.input.state??'active');
export function goalAttention(data:Snapshot,g:Goal,today:string,schedules:Schedule[]=[]):Attention{
 const priority=g.input.long_priority??1,rows=historyEvidence(data,today),last=rows.find(r=>matchesEvidence(r,g.id,'goal',g.input.level==='long'))?.date;
 const anchor=last??g.input.attention_start,period=({weekly:7,fortnightly:14,monthly:30} as Record<string,number>)[g.input.frequency??(g.input.state==='maintaining'?'monthly':'weekly')];
 const stale=Boolean(anchor&&period&&daysAgo(anchor,today)>=period),due=dueReviews(schedules,today).some(s=>s.object_kind==='goal'&&s.object_id===g.id);
 const difficulty=goalReferenceKnowledge(data,g.id).some(r=>r.knowledge.input.kind==='difficulty'&&r.knowledge.details?.priority&&r.paths.some(p=>p.kind!=='ancestor'));
 const inactive=!activeGoal(g),signal=difficulty?0:due?10:stale?20:40;
 return {stale,rank:inactive?900:(2-priority)*100+signal,level:inactive||priority===0?'low':priority===2?'high':difficulty||due||stale?'watch':'normal',reasons:[priorityName(priority),...(inactive?['暂停、已达成或已收起']:[]),...(difficulty?['有关联的重点困难，先考虑支持条件']:[]),...(due?['已到期复盘']:[]),...(stale?['超过家庭关注间隔；可灵活调整活动']:[])]};
}
export function methodAttention(data:Snapshot,m:Method,today:string,schedules:Schedule[]=[]):Attention{
 const priority=m.input.priority??(m.input.pinned?2:1),inactive=(m.input.state??'active')!=='active',goals=data.goals.filter(g=>m.input.goal_ids.includes(g.id)),completed=goals.length>0&&goals.every(g=>g.input.state==='achieved'||g.input.archived);
 const trouble=data.knowledge.some(k=>!k.deleted&&k.input.kind==='difficulty'&&['pending','following'].includes(k.details?.state||'pending')&&(k.input.method_ids.includes(m.id)||k.input.goal_ids.some(id=>m.input.goal_ids.includes(id))));
 const due=dueReviews(schedules,today).some(s=>s.object_kind==='method'&&s.object_id===m.id),stale=goals.some(g=>goalAttention(data,g,today,schedules).stale);
 const signal=m.input.needs_attention?0:trouble?5:due?10:stale?20:40;
 return {stale,rank:inactive?950:completed?850:(2-priority)*100+signal,level:inactive||completed||priority===0?'low':m.input.pinned||priority===2?'high':m.input.needs_attention||trouble||due||stale?'watch':'normal',reasons:[...(m.input.pinned?['家人手动置顶']:[]),priorityName(priority),...(inactive?['暂停或停用']:[]),...(completed?['仅关联已达成或收起目标']:[]),...(m.input.needs_attention?[`需要注意：${m.input.attention_reason}`]:[]),...(trouble?['有关联的未解决困难，先检查做法']:[]),...(due?['已到期复盘']:[]),...(stale?['关联目标超过家庭关注间隔']:[])]};
}
export function prioritizedLongDirections(data:Snapshot,today:string,schedules:Schedule[]=[]){return data.goals.filter(g=>g.input.level==='long'&&activeGoal(g)).sort((a,b)=>goalAttention(data,a,today,schedules).rank-goalAttention(data,b,today,schedules).rank||a.input.title.localeCompare(b.input.title,'zh-CN')||a.id.localeCompare(b.id));}

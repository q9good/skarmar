import type {Snapshot,Goal,Method,PriorityPolicy} from './foundation';
import type {Schedule} from './prd-followups';
import {historyEvidence,matchesEvidence,daysAgo} from './prd-history.ts';
import {goalReferenceKnowledge} from './plan-related-knowledge.ts';
export type Attention={score:number;base:number;bonus:number;temporary:boolean;rank:number;level:'high'|'watch'|'normal'|'low';reasons:string[];stale:boolean};
export const priorityName=(value:number|null|undefined)=>({2:'高关注',1:'普通关注',0:'低关注'}[value??1]??'普通关注');
export const defaultPriorityPolicy:PriorityPolicy={version:0,minimum:0,maximum:100,method_per_goal:2,method_bonus_cap:20};
export const priorityPolicy=(data:Snapshot)=>data.priority_policy??defaultPriorityPolicy;
export function priorityValue(data:Snapshot,item:Goal|Method,kind:'goal'|'method',today:string){
 const p=priorityPolicy(data),v=item.input,legacy=kind==='goal'?(item as Goal).input.long_priority??1:(item as Method).input.priority??((item as Method).input.pinned?2:1);
 const raw=v.priority_score??legacy*50,base=Math.max(p.minimum,Math.min(p.maximum,Math.round(raw)));
 const count=kind==='method'?new Set((item as Method).input.goal_ids.filter(id=>data.goals.some(g=>g.id===id&&!g.input.archived&&['active','maintaining'].includes(g.input.state??'active')))).size:0;
 const bonus=Math.min(count*p.method_per_goal,p.method_bonus_cap),temporary=Boolean(v.priority_until&&v.priority_until>=today);
 return {base,bonus,score:base+bonus,temporary,count,until:v.priority_until};
}
export function comparePriority(data:Snapshot,a:Goal|Method,b:Goal|Method,kindA:'goal'|'method',kindB:'goal'|'method',today:string,schedules:Schedule[]=[]){
 const aa=kindA==='goal'?goalAttention(data,a as Goal,today,schedules):methodAttention(data,a as Method,today,schedules),bb=kindB==='goal'?goalAttention(data,b as Goal,today,schedules):methodAttention(data,b as Method,today,schedules);
 if(aa.rank>=1e11||bb.rank>=1e11)return aa.rank-bb.rank;
 return Number(bb.temporary)-Number(aa.temporary)||bb.score-aa.score||((Date.parse(data.created_at?.[b.id]??'')||0)-(Date.parse(data.created_at?.[a.id]??'')||0))||aa.rank-bb.rank||a.input.title.localeCompare(b.input.title,'zh-CN')||a.id.localeCompare(b.id);
}
export function priorityLabel(data:Snapshot,item:Goal|Method,kind:'goal'|'method',today:string){const v=priorityValue(data,item,kind,today);return `${v.score}${v.bonus?`（基础${v.base}＋关联${v.bonus}）`:''}${v.temporary?' · 临时优先':''}`;}
export const twoWeekPriority=(today:string)=>new Date(Date.parse(today+'T00:00:00Z')+13*86400000).toISOString().slice(0,10);
export function todayFocus(data:Snapshot,today:string,schedules:Schedule[]=[]){
 const long=data.goals.filter(g=>activeGoal(g)&&(g.input.level==='long'||Boolean(g.input.priority_until&&g.input.priority_until>=today)));
 const connected=new Set(long.map(g=>g.id));for(const g of data.goals){let p=g.input.parent_id;const seen=new Set([g.id]);while(p&&!seen.has(p)){seen.add(p);if(connected.has(p)){connected.add(g.id);break;}p=data.goals.find(v=>v.id===p)?.input.parent_id??null;}}
 return [...long.map(value=>({kind:'goal' as const,value})),...data.methods.filter(m=>(m.input.state??'active')==='active'&&m.input.goal_ids.some(id=>data.goals.some(g=>g.id===id&&activeGoal(g))&&(connected.has(id)||Boolean(m.input.priority_until&&m.input.priority_until>=today)))&&!m.input.goal_ids.every(id=>data.goals.some(g=>g.id===id&&(g.input.archived||g.input.state==='achieved')))).map(value=>({kind:'method' as const,value}))].sort((a,b)=>comparePriority(data,a.value,b.value,a.kind,b.kind,today,schedules));
}
export const dueReviews=(schedules:Schedule[],today:string)=>schedules.filter(s=>s.reminders&&(s.eligible||s.mode==='resume')&&Boolean(s.due_date&&s.due_date<=today)).sort((a,b)=>(a.due_date??'').localeCompare(b.due_date??'')||a.object_kind.localeCompare(b.object_kind)||a.object_id.localeCompare(b.object_id));
const activeGoal=(g:Goal)=>!g.input.archived&&['active','maintaining'].includes(g.input.state??'active');
export function goalAttention(data:Snapshot,g:Goal,today:string,schedules:Schedule[]=[]):Attention{
 const numeric=priorityValue(data,g,'goal',today),p=priorityPolicy(data),priority=g.input.long_priority??1,rows=historyEvidence(data,today),last=rows.find(r=>matchesEvidence(r,g.id,'goal',g.input.level==='long'))?.date;
 const anchor=last??g.input.attention_start,period=({weekly:7,fortnightly:14,monthly:30} as Record<string,number>)[g.input.frequency??(g.input.state==='maintaining'?'monthly':'weekly')];
 const stale=Boolean(anchor&&period&&daysAgo(anchor,today)>=period),due=dueReviews(schedules,today).some(s=>s.object_kind==='goal'&&s.object_id===g.id);
 const difficulty=goalReferenceKnowledge(data,g.id).some(r=>r.knowledge.input.kind==='difficulty'&&r.knowledge.details?.priority&&r.paths.some(p=>p.kind!=='ancestor'));
 const inactive=!activeGoal(g),signal=difficulty?0:due?10:stale?20:40;
 return {...numeric,stale,rank:inactive?1e12:(numeric.temporary?-1e9:0)-numeric.score*1000+signal,level:inactive?'low':numeric.temporary||numeric.base>=p.minimum+(p.maximum-p.minimum)*.75?'high':numeric.base<=p.minimum+(p.maximum-p.minimum)*.25?'low':difficulty||due||stale?'watch':'normal',reasons:[`优先级 ${numeric.score}`,`基础 ${numeric.base}`,...(numeric.temporary?[`临时优先至 ${numeric.until}`]:[]),...(inactive?['暂停、已达成或已收起']:[]),...(difficulty?['有关联的重点困难，先考虑支持条件']:[]),...(due?['已到期复盘']:[]),...(stale?['超过家庭关注间隔；可灵活调整活动']:[])]};
}
export function methodAttention(data:Snapshot,m:Method,today:string,schedules:Schedule[]=[]):Attention{
 const numeric=priorityValue(data,m,'method',today),p=priorityPolicy(data),priority=m.input.priority??(m.input.pinned?2:1),inactive=(m.input.state??'active')!=='active',goals=data.goals.filter(g=>m.input.goal_ids.includes(g.id)),completed=goals.length>0&&goals.every(g=>g.input.state==='achieved'||g.input.archived);
 const trouble=data.knowledge.some(k=>!k.deleted&&k.input.kind==='difficulty'&&['pending','following'].includes(k.details?.state||'pending')&&(k.input.method_ids.includes(m.id)||k.input.goal_ids.some(id=>m.input.goal_ids.includes(id))));
 const due=dueReviews(schedules,today).some(s=>s.object_kind==='method'&&s.object_id===m.id),stale=goals.some(g=>goalAttention(data,g,today,schedules).stale);
 const signal=m.input.needs_attention?0:trouble?5:due?10:stale?20:40;
 return {...numeric,stale,rank:inactive||completed?1e12+(inactive?1000:0):(numeric.temporary?-1e9:0)-numeric.score*1000+signal,level:inactive||completed?'low':numeric.temporary||numeric.base>=p.minimum+(p.maximum-p.minimum)*.75?'high':numeric.base<=p.minimum+(p.maximum-p.minimum)*.25?'low':m.input.needs_attention||trouble||due||stale?'watch':'normal',reasons:[`优先级 ${numeric.score}`,`基础 ${numeric.base} ＋ ${numeric.count}个在用目标关联加分 ${numeric.bonus}`,...(numeric.temporary?[`临时优先至 ${numeric.until}`]:[]),...(inactive?['暂停或停用']:[]),...(completed?['仅关联已达成或收起目标']:[]),...(m.input.needs_attention?[`需要注意：${m.input.attention_reason}`]:[]),...(trouble?['有关联的未解决困难，先检查做法']:[]),...(due?['已到期复盘']:[]),...(stale?['关联目标超过家庭关注间隔']:[])]};
}
export function prioritizedLongDirections(data:Snapshot,today:string,schedules:Schedule[]=[]){return data.goals.filter(g=>g.input.level==='long'&&activeGoal(g)).sort((a,b)=>comparePriority(data,a,b,'goal','goal',today,schedules));}

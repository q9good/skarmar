import type {Snapshot} from './foundation';
import type {Session} from './prd-sessions';
import type {Lesson} from './prd-lessons';
export type Evidence={id:string;kind:'family'|'lesson';date:string;stamp:string;direct:string[];below:string[];focus:string[];methods:string[];record:Session|Lesson};
export const dateShift=(date:string,n:number)=>new Date(Date.parse(date+'T00:00:00Z')+n*86400000).toISOString().slice(0,10);
export const daysAgo=(date:string,today:string)=>Math.round((Date.parse(today+'T00:00:00Z')-Date.parse(date+'T00:00:00Z'))/86400000);
export function historyEvidence(data:Snapshot,today:string,source:'family'|'lesson'|'combined'='family'):Evidence[]{
 const rows:Evidence[]=[];
 if(source!=='lesson')for(const s of data.sessions)if(s.status==='completed'&&s.input.business_date<=today)rows.push({id:s.id,kind:'family',date:s.input.business_date,stamp:s.created_at,direct:s.actual_targets.map(t=>t.goal.id),below:[...new Set(s.actual_targets.flatMap(t=>t.ancestors.map(g=>g.id)))],focus:s.focus_goals.map(g=>g.id),methods:s.actual_methods.map(m=>m.id),record:s});
 if(source!=='family')for(const l of data.lessons)if(l.status==='recorded'&&l.observation.business_date<=today)rows.push({id:l.id,kind:'lesson',date:l.observation.business_date,stamp:l.created_at,direct:l.organization.targets.map(t=>t.goal.id),below:[...new Set(l.organization.targets.flatMap(t=>[...t.ancestors.map(g=>g.id),...(t.goal.input.parent_id?[t.goal.input.parent_id]:[])]))],focus:l.organization.focus_goals.map(g=>g.id),methods:l.organization.methods.map(m=>m.id),record:l});
 return rows.sort((a,b)=>b.date.localeCompare(a.date)||b.stamp.localeCompare(a.stamp)||a.kind.localeCompare(b.kind)||a.id.localeCompare(b.id));
}
// Membership uses the historical snapshot, not today's parent/method mapping.
export function matchesEvidence(row:Evidence,id:string,kind:'goal'|'method',long=false){return kind==='method'?row.methods.includes(id):long?row.focus.includes(id)||row.below.includes(id):row.direct.includes(id)||row.below.includes(id);}
export function timeDistribution(rows:Evidence[],today:string){
 const start=dateShift(today,-89),window=rows.filter(r=>r.date>=start&&r.date<=today);
 const weeks=Array.from({length:13},(_,i)=>{const end=dateShift(today,-7*(12-i)),from=dateShift(end,-6);return {start:from<start?start:from,end,count:window.filter(r=>r.date>=from&&r.date<=end).length};});
 return {start,end:today,count:window.length,last:rows.filter(r=>r.date<=today).map(r=>r.date).sort().at(-1)??null,weeks};
}

export function prioritizeMethods(data:Snapshot,overdueGoalIds:string[]){
 const overdue=new Set(overdueGoalIds);
 return data.methods.map(m=>{
  const difficult=data.knowledge.some(k=>!k.deleted&&k.input.kind==='difficulty'&&['pending','following'].includes(k.details?.state||'pending')&&(k.input.method_ids.includes(m.id)||k.input.goal_ids.some(id=>m.input.goal_ids.includes(id))));
  const delayed=m.input.goal_ids.some(id=>overdue.has(id));
  const achieved=m.input.goal_ids.length>0&&m.input.goal_ids.every(id=>data.goals.find(g=>g.id===id)?.input.state==='achieved');
  const rank=m.input.pinned||m.input.needs_attention?0:difficult?1:delayed?2:achieved?4:3;
  return {id:m.id,rank,reason:m.input.pinned?'家人手动置顶':m.input.needs_attention?`家人标记需注意：${m.input.attention_reason}`:difficult?'关联未解决困难，先看看是否需要调整':delayed?'关联目标已超出家庭记录的期望间隔':achieved?'仅关联已达成目标，保留供复习':'使用中的方法',title:m.input.title};
 }).sort((a,b)=>a.rank-b.rank||a.title.localeCompare(b.title,'zh-CN')||a.id.localeCompare(b.id));
}

export function dailyDistribution(rows:Evidence[],today:string){
 const counts=new Map<string,{family:number;lesson:number}>();
 for(const r of rows){if(r.date>today||r.date<dateShift(today,-89))continue;const count=counts.get(r.date)??{family:0,lesson:0};count[r.kind==='family'?'family':'lesson']++;counts.set(r.date,count);}
 return Array.from({length:90},(_,i)=>{const date=dateShift(today,i-89),count=counts.get(date)??{family:0,lesson:0};return {date,...count,count:count.family+count.lesson};});
}

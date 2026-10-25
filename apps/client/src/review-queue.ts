import type {Snapshot} from './foundation';
import type {Schedule} from './prd-followups';
export type ReviewGroup={id:string;title:string;items:{schedule:Schedule;otherGoals:string[]}[]};
/** Planning groups use current explicit relations. A method linked to two goals
 * appears once; secondary relations remain readable. Never duplicate its review. */
export function groupReviewQueue(schedules:Schedule[],data:Snapshot):ReviewGroup[]{
 const groups=new Map<string,ReviewGroup>(),seen=new Set<string>();
 const sorted=[...schedules].sort((a,b)=>(a.due_date??'9999').localeCompare(b.due_date??'9999')||a.object_kind.localeCompare(b.object_kind)||a.object_id.localeCompare(b.object_id));
 for(const s of sorted){const key=s.object_kind+':'+s.object_id;if(seen.has(key))continue;seen.add(key);
  const knowledge=s.object_kind==='knowledge'?data.knowledge.find(k=>k.input.id===s.object_id):null;
  const methodIds=s.object_kind==='method'?[s.object_id]:knowledge?.input.method_ids??[];
  const goalIds=[...new Set(s.object_kind==='goal'?[s.object_id]:[...(knowledge?.input.goal_ids??[]),...data.methods.filter(m=>methodIds.includes(m.id)).flatMap(m=>m.input.goal_ids)])];
  const goals=goalIds.map(id=>data.goals.find(g=>g.id===id)).filter(g=>g!==undefined).sort((a,b)=>a.input.title.localeCompare(b.input.title,'zh-CN')||a.id.localeCompare(b.id));
  const id=goals[0]?.id??'unbound',group=groups.get(id)??{id,title:goals[0]?.input.title??'其他待回看事项',items:[]};
  group.items.push({schedule:s,otherGoals:goals.slice(1).map(g=>g.input.title)});groups.set(id,group);
 }
 return [...groups.values()];
}
export const caregiverLabels:Record<string,string>={easy:'轻松',okay:'还行',stretched:'吃力'};
export const reviewObjectTitle=(s:Pick<Schedule,'object_kind'|'object_id'>,data:Snapshot)=>s.object_kind==='goal'?data.goals.find(g=>g.id===s.object_id)?.input.title??'已删除目标':s.object_kind==='method'?data.methods.find(m=>m.id===s.object_id)?.input.title??'已删除方法':data.knowledge.find(k=>k.input.id===s.object_id)?.details?.title||data.knowledge.find(k=>k.input.id===s.object_id)?.input.content||'已删除对象';

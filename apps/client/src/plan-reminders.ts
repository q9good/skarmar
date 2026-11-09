import type {Snapshot} from './foundation';
import {todayFocus} from './object-priority.ts';
import type {Schedule} from './prd-followups.ts';
import {stateOf} from './prd-followups.ts';
import {knowledgeTitle} from './summary-knowledge.ts';
export const planFocus=(data:Snapshot,today:string,schedules:Schedule[]=[])=>todayFocus(data,today,schedules).slice(0,6);
export type RecentReminder={id:string;kind:'goal'|'method'|'experience'|'difficulty';title:string;created_at:string};
/** Creation time only. Last edit never turns old material into new material. */
export function recentReminders(data:Snapshot,today:string,all=false):RecentReminder[]{
 const start=new Date(Date.parse(today+'T00:00:00Z')-13*86400000).toISOString().slice(0,10);
 const rows:RecentReminder[]=[...data.goals.filter(g=>!g.input.archived&&['active','maintaining'].includes(g.input.state??'active')).map(g=>({id:g.id,kind:'goal' as const,title:g.input.title,created_at:data.created_at?.[g.id]??''})),...data.methods.filter(m=>(m.input.state??'active')==='active').map(m=>({id:m.id,kind:'method' as const,title:m.input.title,created_at:data.created_at?.[m.id]??''})),...data.knowledge.filter(k=>!k.deleted&&(k.input.kind==='difficulty'?['pending','following'].includes(stateOf(k)):stateOf(k)==='active')).map(k=>({id:k.input.id,kind:k.input.kind,title:knowledgeTitle(k),created_at:k.created_at}))];
 const date=new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}),counts:Record<string,number>={};
 return rows.filter(r=>{const stamp=Date.parse(r.created_at);if(!Number.isFinite(stamp))return false;const day=date.format(new Date(stamp));return day>=start&&day<=today;}).sort((a,b)=>Date.parse(b.created_at)-Date.parse(a.created_at)||a.kind.localeCompare(b.kind)||a.id.localeCompare(b.id)).filter(r=>all||((counts[r.kind]=(counts[r.kind]??0)+1)<=3));
}

import type {Snapshot,Goal} from './foundation';
import type {Coverage} from './prd-sessions';
import {stateOf} from './prd-followups.ts';
export function planningKnowledge(data:Snapshot,id:string){
 return data.knowledge.filter(k=>!k.deleted&&(k.input.kind==='difficulty'?['pending','following'].includes(stateOf(k)):stateOf(k)==='active')&&(k.input.goal_ids.includes(id)||data.methods.some(m=>m.input.goal_ids.includes(id)&&k.input.method_ids.includes(m.id)))).sort((a,b)=>Number(Boolean(b.details?.key))-Number(Boolean(a.details?.key))||a.input.id.localeCompare(b.input.id));
}
export function recommendationReason(g:Goal,report:Coverage|null){
 const row=report?.goals.find(r=>r.id===g.id);if(!row)return '按当前推进状态留意，可自由选择';
 if(!row.last_date)return `尚无记录；从${row.attention_anchor}开始关注`;
 const frequency=g.input.frequency??(g.input.state==='maintaining'?'monthly':'weekly');
 const period=({weekly:7,fortnightly:14,monthly:30} as Record<string,number>)[frequency];
 return period&&row.ratio!==null?`最近记录${row.last_date}；已${Math.round(row.ratio*period)}天，家庭设定每${period}天留意一次`:`最近记录${row.last_date}；未设置关注节奏`;
}
export function goalChoiceGroups(data:Snapshot,report:Coverage|null,search:string){
 const remaining=data.goals.filter(g=>g.input.level!=='long'&&g.input.title.includes(search));const used=new Set<string>();
 const take=(ids:string[])=>ids.flatMap(id=>{const g=remaining.find(g=>g.id===id);if(!g||used.has(id))return [];used.add(id);return[g];});
 const suggested=take(report?.suggested_ids??[]);
 const recent=take(remaining.filter(g=>report?.goals.find(r=>r.id===g.id)?.last_date).sort((a,b)=>(report?.goals.find(r=>r.id===b.id)?.last_date??'').localeCompare(report?.goals.find(r=>r.id===a.id)?.last_date??'')||a.id.localeCompare(b.id)).slice(0,5).map(g=>g.id));
 return [{title:'按关注节奏建议',goals:suggested},{title:'最近在练',goals:recent},...(['medium','short'] as const).map(level=>({title:level==='medium'?'其他中期目标':'其他近期本领',goals:take(remaining.filter(g=>g.input.level===level).map(g=>g.id))}))].filter(g=>g.goals.length);
}

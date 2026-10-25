import type {Snapshot} from './foundation';
import type {Session} from './prd-sessions';
import type {Lesson} from './prd-lessons';
import {historyEvidence} from './prd-history.ts';
export type ObjectKey={id:string;kind:'goal'|'method'};
export function objectActions(data:Snapshot,object:ObjectKey){
 const methodIds=object.kind==='goal'?data.methods.filter(m=>m.input.goal_ids.includes(object.id)).map(m=>m.id):[object.id];
 return data.actions.filter(a=>object.kind==='goal'&&a.input.goal_ids.includes(object.id)||a.input.method_ids.some(id=>methodIds.includes(id)));
}
/** Use membership and notes from the recorded versions. Never place another
 * target's observations under the selected goal, including long focus goals. */
export function objectEvidence(data:Snapshot,object:ObjectKey,today:string){
 return historyEvidence(data,today,'combined').flatMap(e=>{
  const matched=object.kind==='method'?e.methods.includes(object.id):e.direct.includes(object.id)||e.below.includes(object.id)||e.focus.includes(object.id);
  if(!matched)return [];
  const method=object.kind==='method';let notes:string[]=[];
  if(e.kind==='family')notes=(e.record as Session).actual_targets.filter(t=>method||t.goal.id===object.id||t.ancestors.some(g=>g.id===object.id)).map(t=>t.input.note).filter(Boolean);
  else notes=(e.record as Lesson).organization.targets.filter(t=>method||t.goal.id===object.id||t.ancestors.some(g=>g.id===object.id)).map(t=>t.target.note).filter(Boolean);
  const note=notes.join('；')||(e.focus.includes(object.id)?'明确关注过这个方向，未单独填写文字观察':e.kind==='family'?'已明确关联，未单独填写这个目标的家庭表现':'已明确关联，未单独填写这个目标的课堂表现');
  return [{...e,note,contextOnly:method}];
 });
}

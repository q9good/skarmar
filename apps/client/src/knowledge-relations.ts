import type {Snapshot} from './foundation';
import type {Knowledge} from './prd-lessons';

/** Direct applicability and applicability via a method are separate relations.
 * One knowledge item may have both paths, but is still displayed only once. */
export function knowledgeForGoal(data:Snapshot,id:string){
 const methods=data.methods.filter(m=>m.input.goal_ids.includes(id)),seen=new Set<string>();
 return data.knowledge.flatMap(knowledge=>{
  if(knowledge.deleted||seen.has(knowledge.input.id))return [];
  const direct=knowledge.input.goal_ids.includes(id),via=methods.filter(m=>knowledge.input.method_ids.includes(m.id));
  if(!direct&&!via.length)return [];seen.add(knowledge.input.id);return [{knowledge,direct,via}];
 });
}
export function knowledgeRelation(data:Snapshot,k:Knowledge,goalId?:string,methodId?:string){
 if(methodId)return '直接关联本方法';
 if(!goalId)return '';
 const via=data.methods.filter(m=>m.input.goal_ids.includes(goalId)&&k.input.method_ids.includes(m.id)).map(m=>m.input.title);
 return [k.input.goal_ids.includes(goalId)?'直接关联本目标':'',via.length?'经方法关联：'+via.join('、'):''].filter(Boolean).join('；');
}
export function knowledgeSource(data:Snapshot,k:Knowledge){
 if(k.source_session_id){const s=data.sessions.find(s=>s.id===k.source_session_id);return `来源：家庭干预第${k.source_session_version??'未记'}版${s&&['deleted','cancelled'].includes(s.status)?' · 来源记录已删除或取消':''}`;}
 if(k.source_lesson_id){const l=data.lessons.find(l=>l.id===k.source_lesson_id);return `来源：课堂观察第${k.source_observation_version}版${l&&l.status!=='recorded'?' · 来源记录已删除或撤销':''}`;}
 if(k.source_event_id)return '来源：实践反馈或跟进记录';
 if(k.source_review_id)return '来源：复盘记录';
 if(k.derived_from?.length)return '来源：原经验／困难的明确派生，详情保留原版本与差异';
 return '来源：家人独立录入';
}

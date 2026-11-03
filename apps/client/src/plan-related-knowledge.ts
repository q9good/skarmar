import type {Snapshot} from './foundation';
import type {SessionInput} from './prd-sessions';
import {stateOf} from './prd-followups.ts';
/** A plan only carries knowledge tied to chosen goals/their ancestors or chosen
 * methods. A method's relationship to a goal alone does not mean it was chosen. */
export function planRelatedKnowledge(data:Snapshot,input:Pick<SessionInput,'targets'|'methods'|'focus_goal_ids'>){
 const goals=new Set<string>(),methods=new Set(input.methods.map(m=>m.id));
 for(const id of [...input.targets.map(t=>t.goal_id),...input.focus_goal_ids]){
  let current:string|null=id;const visited=new Set<string>();
  while(current&&!visited.has(current)){visited.add(current);goals.add(current);current=data.goals.find(g=>g.id===current)?.input.parent_id??null;}
 }
 const seen=new Set<string>();
 return data.knowledge.filter(k=>{
  if(k.deleted||seen.has(k.input.id))return false;
  if(k.input.kind==='difficulty'?!['pending','following'].includes(stateOf(k)):stateOf(k)!=='active')return false;
  const related=k.input.goal_ids.some(id=>goals.has(id))||k.input.method_ids.some(id=>methods.has(id));
  if(related)seen.add(k.input.id);return related;
 }).map(knowledge=>({knowledge,
  goals:data.goals.filter(g=>goals.has(g.id)&&knowledge.input.goal_ids.includes(g.id)),
  methods:data.methods.filter(m=>methods.has(m.id)&&knowledge.input.method_ids.includes(m.id)),
 })).sort((a,b)=>Number(Boolean(b.knowledge.details?.key))-Number(Boolean(a.knowledge.details?.key))||a.knowledge.input.id.localeCompare(b.knowledge.input.id));
}

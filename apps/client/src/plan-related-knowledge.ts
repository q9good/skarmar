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

export type ReferencePath={goalId:string;kind:'direct'|'method'|'ancestor';methodId?:string;ancestorId?:string};
export type KnowledgeReference={knowledge:Snapshot['knowledge'][number];paths:ReferencePath[]};
const usable=(k:Snapshot['knowledge'][number])=>!k.deleted&&(k.input.kind==='difficulty'?['pending','following'].includes(stateOf(k)):stateOf(k)==='active');
const referenceOrder=(a:KnowledgeReference,b:KnowledgeReference)=>Number(Boolean(b.knowledge.details?.priority||b.knowledge.details?.key))-Number(Boolean(a.knowledge.details?.priority||a.knowledge.details?.key))||Number(b.paths.some(p=>p.kind==='direct'))-Number(a.paths.some(p=>p.kind==='direct'))||a.knowledge.input.id.localeCompare(b.knowledge.input.id);

/** Bounded applicability paths: own goal, its active methods, and direct parent
 * principles. Never walk through a shared method to another goal's knowledge. */
export function goalReferenceKnowledge(data:Snapshot,goalId:string):KnowledgeReference[]{
 const methods=data.methods.filter(m=>(m.input.state??'active')==='active'&&m.input.goal_ids.includes(goalId));
 const ancestors:string[]=[],seen=new Set([goalId]);let id=data.goals.find(g=>g.id===goalId)?.input.parent_id;
 while(id&&!seen.has(id)){
  seen.add(id);const g=data.goals.find(g=>g.id===id);if(!g)break;
  if(!g.input.archived)ancestors.push(id);id=g.input.parent_id;
 }
 const added=new Set<string>();
 return data.knowledge.flatMap(knowledge=>{
  if(!usable(knowledge)||added.has(knowledge.input.id))return [];
  const paths:ReferencePath[]=[];
  if(knowledge.input.goal_ids.includes(goalId))paths.push({goalId,kind:'direct'});
  for(const m of methods)if(knowledge.input.method_ids.includes(m.id))paths.push({goalId,kind:'method',methodId:m.id});
  for(const parent of ancestors)if(knowledge.input.goal_ids.includes(parent))paths.push({goalId,kind:'ancestor',ancestorId:parent});
  if(!paths.length)return [];added.add(knowledge.input.id);return [{knowledge,paths}];
 }).sort(referenceOrder);
}

export function referencesForGoals(data:Snapshot,goalIds:string[]):KnowledgeReference[]{
 const rows=new Map<string,KnowledgeReference>();
 for(const goalId of new Set(goalIds))for(const row of goalReferenceKnowledge(data,goalId)){
  const old=rows.get(row.knowledge.input.id);
  if(old){
   for(const path of row.paths)if(!old.paths.some(p=>JSON.stringify(p)===JSON.stringify(path)))old.paths.push(path);
  }else rows.set(row.knowledge.input.id,{knowledge:row.knowledge,paths:[...row.paths]});
 }
 return [...rows.values()].sort(referenceOrder);
}

export function referencePathLabel(data:Snapshot,path:ReferencePath){
 const title=data.goals.find(g=>g.id===path.goalId)?.input.title??'原目标';
 if(path.kind==='direct')return `直接关联：${title}`;
 if(path.kind==='method')return `经方法「${data.methods.find(m=>m.id===path.methodId)?.input.title??'原方法'}」关联：${title}`;
 return `上级方向「${data.goals.find(g=>g.id===path.ancestorId)?.input.title??'原方向'}」的原则 · ${title}`;
}

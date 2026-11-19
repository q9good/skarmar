import type {SessionInput,Summary} from './prd-sessions';
import type {Knowledge} from './prd-lessons';
/** A display title is not a rewrite of the saved observation. */
export function knowledgeTitle(k:Knowledge){const title=k.details?.title?.trim();if(title)return title;const first=k.input.content.trim().split(/\r?\n/)[0]||'尚未命名';return Array.from(first).length>36?Array.from(first).slice(0,36).join('')+'…':first;}
export function summaryKnowledgeIssue(summary:Summary|null|undefined){
 if(!summary)return '';
 for(const [index,k] of summary.new_knowledge.entries()){
  const label=`第${index+1}条${k.kind==='experience'?'经验':'困难'}`;
  if(!k.content.trim())return `${label}还没有内容。请填写，或明确取消这条新增内容。`;
  if(k.kind==='experience'&&!k.stage)return `${label}还没有选择经验阶段。请选择“想法／可尝试／多次观察到有帮助”，再正式提交。`;
  if(!k.goal_ids.length&&!k.method_ids.length)return `${label}还没有关联目标或方法。请调整知识关联后再提交。`;
 }
 return '';
}
/** Confirm the actual shared rows, rather than treating the session's echoed input as proof. */
export function missingSummaryKnowledge(expected:import('./prd-sessions').Session|undefined,data:{knowledge:Knowledge[]}){
 return expected?.status==='completed'?(expected.input.summary?.new_knowledge??[]).filter(k=>!data.knowledge.some(v=>v.input.id===k.id&&v.source_session_id===expected.id)):[];
}

/** Resolve only links that the author left following the actual goal and methods.
 * Local draft metadata never enters a server request. Explicit links remain as entered. */
export function resolveSummaryLinks(input:SessionInput, following:readonly string[]|undefined, goals:readonly {id:string}[]):SessionInput {
 const summary=input.summary;
 if(!summary||!following?.length)return input;
 const actual=summary.targets.filter(t=>t.performed&&goals.some(g=>g.id===t.goal_id));
 const main=input.targets.find(t=>t.role==='main'&&actual.some(a=>a.goal_id===t.goal_id));
 const goalId=main?.goal_id??actual[0]?.goal_id;
 const new_knowledge=summary.new_knowledge.map(k=>following.includes(k.id)
  ? {...k,goal_ids:goalId?[goalId]:[],method_ids:summary.methods.map(m=>m.id)}
  : k);
 return {...input,summary:{...summary,new_knowledge}};
}

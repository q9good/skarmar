import type {Snapshot} from './foundation';
import type {Coverage,Counts} from './prd-sessions';

export type CoverageTopic='goals'|'methods'|'tags';
export type CoverageItem={id:string;label:string;counts:Counts;band?:number;last?:string|null;detail:string};
export const coverageColors=['#F3F6EF','#E4EEDC','#CADDB9','#94B681','#4E7656'];
export function coverageLegend(days:number){return days===7?'0／1／2／3–4／5+':days===30?'0／1／2–3／4–7／8+':'0／1–3／4–8／9–20／21+';}

/** Each row is one object, not one session. A multi-target session may hit
 * several rows. All counts and source components come from one server report. */
export function coverageItems(data:Snapshot,report:Coverage,topic:CoverageTopic,level:string,unrecorded=false):CoverageItem[]{
 if(topic==='methods'){
  const ids=[...new Set([...data.methods.filter(m=>(m.input.state??'active')==='active').map(m=>m.id),...Object.keys(report.methods)])];
  return ids.map(id=>({id,label:data.methods.find(m=>m.id===id)?.input.title??'历史方法',counts:report.methods[id]??{family:0,lesson:0,selected:0},detail:'仅统计确认实际使用，不按计划选择计数；0次只表示本窗口未记录使用'}));
 }
 if(topic==='tags')return [
  ...Object.entries(report.tags).map(([id,n])=>({id:'tag:'+id,label:id,counts:{family:n,lesson:0,selected:n},detail:'家庭正式总结中的实际标签；同条同标签只计一次'})),
  ...(report.unclassified>0?[{id:'untagged',label:'未分类（未填标签）',counts:{family:report.unclassified,lesson:0,selected:report.unclassified},detail:'已正式记录家庭干预，但没有填写活动标签，不等于没有干预'}]:[]),
 ];
 return data.goals.filter(g=>g.input.level===level&&!g.input.archived).flatMap(g=>{
  const row=report.goals.find(r=>r.id===g.id);if(!row)return [];
  const counts=level==='long'?row.focus:row.counts;
  if(unrecorded&&counts.selected!==0)return [];
  return [{id:g.id,label:g.input.title,counts,band:level==='long'?undefined:row.band,last:row.last_date,
   detail:level==='long'?`明确长期关注；下级证据另列：家庭${row.descendants.family}／机构${row.descendants.lesson}，不与关注数相加`:`直接命中${row.direct.selected}／下级证据${row.descendants.selected}（同条合并去重）；${row.last_date?'最近实际记录 '+row.last_date:'尚无记录 · 开始关注 '+row.attention_anchor}`}];
 });
}

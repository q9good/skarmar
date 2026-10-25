import type {EvidenceRecord} from './prd-observations';
export type MeasurePoint={id:string;date:string;value:number|null;label:string};
export type MeasureSeries={key:string;name:string;metricId:string;metricVersion:number;goalVersion:number;template:string;unit:string;support:string;sourceName:string;methodBasis:string;points:MeasurePoint[];comparable:boolean};
function methodBasis(r:EvidenceRecord){
 const s=r.source_snapshot as {actual_methods?:{id:string;version:number;input?:{title?:string}}[];organization?:{methods?:{id:string;version:number;input?:{title?:string}}[]};input?:{summary?:{confirmation?:string}}}|null;
 const methods=s?.actual_methods??s?.organization?.methods;
 if(methods?.length)return {key:methods.map(m=>`${m.id}@${m.version}`).sort().join('|'),label:methods.map(m=>`${m.input?.title??'方法'} 第${m.version}版`).join('、')};
 return {key:s?.input?.summary?.confirmation==='none'?'none':'unknown',label:s?.input?.summary?.confirmation==='none'?'明确未使用库中方法':'方法版本未记录'};
}
export function measureValue(r:EvidenceRecord):number|null{
 const p=r.input,kind=r.metric?.input.template;
 if(kind==='count')return typeof p.opportunities==='number'&&Number.isSafeInteger(p.opportunities)&&p.opportunities>0&&typeof p.achieved==='number'&&Number.isSafeInteger(p.achieved)&&p.achieved>=0&&p.achieved<=p.opportunities?p.achieved/p.opportunities*100:null;
 return kind==='duration'&&typeof p.amount==='number'&&Number.isFinite(p.amount)&&p.amount>=0?p.amount:null;
}
export function measurementSeries(records:EvidenceRecord[],today:string):MeasureSeries[]{
 const groups=new Map<string,MeasureSeries>();
 for(const r of records){
  if(r.kind!=='measurement'||r.deleted||!r.metric||!r.goal||typeof r.input.business_date!=='string'||r.input.business_date>today)continue;
  const metric=r.metric,template=String(metric.input.template),support=String(r.input.support??'').trim(),basis=methodBasis(r);
  const sourceKind=String((r.input.source as {kind?:string}|null)?.kind??'independent');
  const key=JSON.stringify([metric.id,metric.version,r.goal.version,support,sourceKind,basis.key]);
  const group=groups.get(key)??{key,name:String(metric.input.name),metricId:metric.id,metricVersion:metric.version,goalVersion:r.goal.version,template,sourceName:({session:'家庭干预',lesson:'机构课程',moment:'瞬间观察',independent:'独立测量'} as Record<string,string>)[sourceKind]??'其他来源',unit:template==='count'?'%':String(metric.input.unit??''),support,methodBasis:basis.label,points:[],comparable:!!support};
  const value=measureValue(r),label=template==='count'?(value===null?'未记录有效机会数／做到数，不计算比例':`${r.input.achieved}／${r.input.opportunities} 次（${Math.round(value*10)/10}%）`):template==='duration'?(value===null?'实际值未记录':`${value} ${group.unit}`.trim()):String(r.input.text||'文字观察未记录');
  group.points.push({id:r.id,date:r.input.business_date,value,label});groups.set(key,group);
 }
 return [...groups.values()].map(g=>({...g,points:g.points.sort((a,b)=>a.date.localeCompare(b.date)||a.id.localeCompare(b.id))})).sort((a,b)=>(b.points.at(-1)?.date??'').localeCompare(a.points.at(-1)?.date??'')||a.key.localeCompare(b.key));
}

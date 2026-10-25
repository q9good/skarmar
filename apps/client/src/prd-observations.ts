import type {Goal} from './foundation';
import type {Audit,Source} from './prd-followups';
export type ObservationKind='metric'|'measurement'|'moment'|'needs';
export type EvidenceRecord={creator_id?:string;creator_name?:string;created_at?:string;source_deleted?:boolean;id:string;kind:ObservationKind;version:number;input:Record<string,unknown>;audit:Audit;deleted:boolean;goal:Goal|null;metric:EvidenceRecord|null;source_snapshot:unknown};
export type ObserveWrite={id:string;operation_id:string;expected_version:number|null;kind:ObservationKind;action:'save'|'delete';input:Record<string,unknown>;note:string};
export type ObserveDraft={command:ObserveWrite;pending?:ObserveWrite;backup?:ObserveWrite};
export const needsId='00000000-0000-4000-8000-000000000001';
export const needsFields=[['communication','沟通方式','怎样表达想要、拒绝或休息？','例如：用手势表示休息'],['sensory','感官需要','什么环境让他舒服，什么需要调整？','例如：嘈杂时希望到安静处'],['discomfort','难受的信号','大人可以留意哪些具体信号？','例如：捂耳、躲开、反复揉眼'],['calming','帮助平静的做法','怎样陪伴、休息或提供帮助？','例如：留出安静空间，等他愿意回来'],['health','已知身体情况','有哪些已确认、值得家人留意的信息？','例如：最近睡眠不足，遵医嘱处理身体不适']] as const;
export const observeNames={metric:'观察指标',measurement:'目标测量',moment:'瞬间观察',needs:'需要与帮助'};
export function normalizedObservation(command:ObserveWrite):ObserveWrite{
 const input={...command.input};
 if(command.kind==='measurement')for(const key of ['opportunities','achieved','amount']){
  const raw=input[key];if(raw===''||raw==null){input[key]=null;continue;}
  const value=typeof raw==='number'?raw:Number(raw);
  if(!/^\d+(\.\d+)?$/.test(String(raw))||!Number.isFinite(value)||value<0||(key!=='amount'&&!Number.isSafeInteger(value)))throw new Error('次数须为非负整数，时长或数量须为非负数字。');input[key]=value;
 }
 return {...command,input};
}
export function measurementText(record:EvidenceRecord):string{
 const p=record.input,kind=record.metric?.input.template;
 if(kind==='count'){
  const n=p.opportunities,k=p.achieved;
  if(typeof n!=='number'||typeof k!=='number'||n<=0)return '未计算做到比例（没有机会或未记录机会数）';
  return `做到 ${k}／${n} 次，${Math.round(k/n*1000)/10}%（该次机会内的比例）`;
 }
 return kind==='duration'?`实际 ${p.amount??'未记录'} ${record.metric?.input.unit??''}`:String(p.text??'未记录');
}
export function measurementSegment(record:EvidenceRecord){return `${record.metric?.id??''}:${record.metric?.version??0}:${String(record.input.support??'')}`;}
export function decodeObservationDrafts(raw:string|null):Record<string,ObserveDraft>{
 if(raw===null)return {};const v=JSON.parse(raw) as Record<string,ObserveDraft>;
 const valid=(w:ObserveWrite)=>{
  if(!w||typeof w!=='object'||typeof w.id!=='string'||!w.id||typeof w.operation_id!=='string'||!w.operation_id||!['metric','measurement','moment','needs'].includes(w.kind)||!['save','delete'].includes(w.action)||!(w.expected_version===null||Number.isSafeInteger(w.expected_version)&&w.expected_version>0)||typeof w.note!=='string'||!w.input||typeof w.input!=='object'||Array.isArray(w.input))return false;
  const p=w.input,str=(k:string)=>typeof p[k]==='string',integer=(k:string)=>Number.isSafeInteger(p[k])&&Number(p[k])>0;
  if(w.kind==='metric')return str('goal_id')&&integer('goal_version')&&str('name')&&['count','duration','steps','text'].includes(String(p.template))&&['criterion','unit','support'].every(k=>p[k]===undefined||str(k));
  if(w.kind==='moment')return str('business_date')&&str('text')&&Array.isArray(p.goal_ids)&&p.goal_ids.every(id=>typeof id==='string');
  if(w.kind==='needs')return needsFields.every(([k])=>p[k]===undefined||str(k));
  const source=p.source as Source|null;
  return str('metric_id')&&integer('metric_version')&&str('business_date')&&str('support')&&str('text')&&['opportunities','achieved','amount'].every(k=>p[k]==null||typeof p[k]==='string'||typeof p[k]==='number'&&Number.isFinite(p[k]))&&(!source||['session','lesson','moment'].includes(source.kind)&&typeof source.id==='string'&&Number.isSafeInteger(source.version)&&source.version>0);
 };
 if(!v||typeof v!=='object'||Array.isArray(v)||Object.entries(v).some(([id,d])=>!d||!valid(d.command)||id!==d.command.id||d.pending&&(!valid(d.pending)||d.pending.id!==id||d.pending.operation_id!==d.command.operation_id||JSON.stringify(d.pending)!==JSON.stringify(normalizedObservation(d.command)))||d.backup&&(!valid(d.backup)||d.backup.id!==id)))throw new Error('本机观察草稿格式不兼容，原始数据已保留，请勿清除浏览器数据。');
 return v;
}
export const emptyMeasurement=(metric:EvidenceRecord,date:string)=>({metric_id:metric.id,metric_version:metric.version,business_date:date,support:String(metric.input.support??''),opportunities:null,achieved:null,amount:null,text:'',source:null as Source|null});

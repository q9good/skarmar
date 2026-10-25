import type {Snapshot} from './foundation';
import {measurementText,needsFields,type EvidenceRecord} from './prd-observations.ts';
export type BriefOptions={from:string;to:string;goalIds:string[];parts:string[]};
export function buildBrief(data:Snapshot,records:EvidenceRecord[],o:BriefOptions):string{
 const validDate=(v:string)=>/^\d{4}-\d{2}-\d{2}$/.test(v)&&Number.isFinite(Date.parse(v+'T00:00:00Z'))&&new Date(v+'T00:00:00Z').toISOString().slice(0,10)===v;
 if(!validDate(o.from)||!validDate(o.to)||o.from>o.to)throw new Error('请核对摘要的起止日期。');
 const has=(part:string)=>o.parts.includes(part),inWindow=(date:string)=>date>=o.from&&date<=o.to;
 const lines=[`家庭简要更新：${o.from} 至 ${o.to}`,`目标：${data.goals.filter(g=>o.goalIds.includes(g.id)).map(g=>g.input.title).join('、')}`,'这是家人选择的已保存观察，供讨论与提问，不作疗效或诊断结论。'];
 if(has('family'))for(const s of data.sessions){
  if(s.status!=='completed'||!inWindow(s.input.business_date))continue;
  const ts=s.actual_targets.filter(t=>o.goalIds.includes(t.goal.id)||t.ancestors.some(g=>o.goalIds.includes(g.id)));
  if(!ts.length&&!s.focus_goals.some(g=>o.goalIds.includes(g.id)))continue;
  if(ts.length)lines.push(`${s.input.business_date} 家庭：${ts.map(t=>`${t.goal.input.title}：${t.input.note}`).join('；')}`);
  else lines.push(`${s.input.business_date} 明确长期关注：${s.focus_goals.filter(g=>o.goalIds.includes(g.id)).map(g=>g.input.title).join('、')}；未单独填写这一方向的文字观察，不带出其他目标表现。`);
  for(const signal of s.input.summary?.signals.filter(s=>s.goal_ids.some(id=>o.goalIds.includes(id)))??[])lines.push(`明确关联的参与信号：${signal.kind}（家庭，${s.input.business_date}）`);
 }
 if(has('lesson'))for(const l of data.lessons){
  const selected=l.organization.targets.some(t=>o.goalIds.includes(t.goal.id)||t.ancestors.some(g=>o.goalIds.includes(g.id)))||l.organization.focus_goals.some(g=>o.goalIds.includes(g.id));
  if(l.status==='recorded'&&inWindow(l.observation.business_date)&&selected)lines.push(`${l.observation.business_date} 课程 ${l.course.name}：${l.observation.notes}；老师建议：${l.observation.teacher_notes}`);
 }
 if(has('difficulty'))for(const k of data.knowledge){
  const selected=k.input.goal_ids.some(id=>o.goalIds.includes(id))||k.input.method_ids.some(mid=>data.methods.some(m=>m.id===mid&&m.input.goal_ids.some(id=>o.goalIds.includes(id))));
  if(!k.deleted&&k.input.kind==='difficulty'&&['pending','following'].includes(k.details?.state||'pending')&&selected)lines.push(`目前困难：${k.input.content}；已有依据：${k.details?.evidence??''}`);
 }
 for(const r of records.filter(r=>!r.deleted)){
  if(r.kind==='moment'&&has('moment')&&inWindow(String(r.input.business_date))&&(r.input.goal_ids as string[]).some(id=>o.goalIds.includes(id)))lines.push(`${r.input.business_date} 瞬间观察（不计干预次数）：${r.input.text}`);
  if(r.kind==='measurement'&&has('measurement')&&inWindow(String(r.input.business_date))&&r.goal&&o.goalIds.includes(r.goal.id))lines.push(`${r.input.business_date} ${r.goal.input.title}的测量：${measurementText(r)}；指标第${r.metric?.version}版，标准：${r.metric?.input.criterion||'未设置'}；帮助条件：${r.input.support||'未记录'}${r.source_deleted?'；来源已删除或撤销，保留当时快照':''}`);
  if(r.kind==='needs'&&has('needs'))for(const [key,label] of needsFields)if(r.input[key])lines.push(`${label}（家人提供）：${r.input[key]}`);
 }
 return lines.join('\n');
}

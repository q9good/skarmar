import type {Snapshot} from './foundation';
import type {EvidenceRecord} from './prd-observations';

export type TodayRecordKind='lesson'|'session'|'moment';
export type TodayRecord={key:string;id:string;kind:TodayRecordKind;label:string;title:string;author:string;timing:string;created_at:string;sync:string};

const weekdays=['周日','周一','周二','周三','周四','周五','周六'];
export function businessDateLabel(date:string){
 const [year,month,day]=date.split('-').map(Number);
 const weekday=new Date(Date.UTC(year,month-1,day)).getUTCDay();
 return date+' '+weekdays[weekday];
}
export function businessTime(stamp:string){
 const date=new Date(stamp);
 if(!stamp||!Number.isFinite(date.getTime()))return '时间未记录';
 return new Intl.DateTimeFormat('zh-CN',{timeZone:'Asia/Shanghai',hour:'2-digit',minute:'2-digit',hour12:false}).format(date);
}
function syncLabel(stamp:string,now:number){
 const time=Date.parse(stamp),delta=now-time;
 return Number.isFinite(time)&&delta>=-60_000&&delta<=5*60_000?'刚保存 · 已同步':'已同步';
}
export function todayRecords(data:Snapshot,moments:EvidenceRecord[],today:string,now=Date.now()):TodayRecord[]{
 const records:TodayRecord[]=[];
 for(const lesson of data.lessons)if(lesson.status==='recorded'&&lesson.observation.business_date===today)records.push({
  key:'lesson:'+lesson.id,id:lesson.id,kind:'lesson',label:'机构课程',title:lesson.course.name+' · '+lesson.teacher.name,
  author:lesson.creator_name,timing:businessTime(lesson.created_at),created_at:lesson.created_at,sync:syncLabel(lesson.created_at,now),
 });
 for(const session of data.sessions)if(session.input.business_date===today&&!['deleted','cancelled'].includes(session.status))records.push({
  key:'session:'+session.id,id:session.id,kind:'session',label:session.status==='completed'?'家庭干预':session.status==='planned'?'家庭干预计划':'总结草稿',
  title:session.input.summary?.targets.find(target=>target.performed)?.observation||session.input.plan||'已保存的家庭安排',
  author:session.creator_name,timing:session.input.time_slot,created_at:session.created_at,sync:syncLabel(session.created_at,now),
 });
 for(const moment of moments)if(moment.kind==='moment'&&!moment.deleted&&moment.input.business_date===today){
  const created=moment.created_at||moment.audit.at;
  records.push({key:'moment:'+moment.id,id:moment.id,kind:'moment',label:'瞬间观察',title:String(moment.input.text||'已保存的瞬间观察'),author:moment.creator_name||moment.audit.actor_name,timing:businessTime(created),created_at:created,sync:syncLabel(created,now)});
 }
 return records.sort((a,b)=>b.created_at.localeCompare(a.created_at)||a.key.localeCompare(b.key));
}

import { decodeDrafts, type Catalog, type Snapshot, type Write, type Saved } from './foundation.ts';
import { decodeLessonDrafts, type Lesson, type LessonWrite } from './prd-lessons.ts';
export type QueueWrite = {kind:'catalog';value:Write}|{kind:'lesson';value:LessonWrite};
export type QueueEntry = {write:QueueWrite;status:'queued'|'uncertain'|'blocked'|'confirmed'|'linked';reason:string;replacement_id?:string;attempted?:boolean;receipt?:Saved|Lesson};
export type Outbox = {schema:'skarmar.lesson-queue.v1';entries:Record<string,QueueEntry>};
export type QueueIdentity = {user:{id:string;active:boolean};space:string;demo:boolean};
export const emptyOutbox = ():Outbox => ({schema:'skarmar.lesson-queue.v1',entries:{}});
export function decodeOutbox(raw:string|null):Outbox {
  if(raw===null)return emptyOutbox();
  const v=JSON.parse(raw) as Outbox;
  if(!v||v.schema!=='skarmar.lesson-queue.v1'||!v.entries||Array.isArray(v.entries)||typeof v.entries!=='object')throw new Error('上传队列版本不兼容，请保留浏览器数据，不会自动覆盖或重试。');
  for(const [id,e] of Object.entries(v.entries)) {
    if(!e||!['queued','uncertain','blocked','confirmed','linked'].includes(e.status)||typeof e.reason!=='string'||(e.attempted!==undefined&&typeof e.attempted!=='boolean')||!e.write||e.write.value?.id!==id||e.write.value.expected_version!==null)throw new Error('上传队列无法读取，原始数据已保留。');
    const r=e.write.value;if(!r.id||!r.operation_id)throw new Error('上传操作编号缺失，原内容已保留。');
    if(e.write.kind==='catalog') {
      decodeDrafts(JSON.stringify({[id]:r}));
      if(!('mutation' in r)||r.mutation.kind!=='catalog'||!r.mutation.value.active||!r.mutation.value.name.trim())throw new Error('目录依赖不兼容，已暂停上传。');
    } else if(e.write.kind==='lesson') {
      decodeLessonDrafts(JSON.stringify({[id+':observation']:{request:r,phase:'basic'}}));
      if(!('action' in r)||r.action.kind!=='observation')throw new Error('上传队列只能新增课记。');
    } else throw new Error('未知的待上传类型，原始数据已保留。');
    if((e.status==='confirmed'||e.status==='linked')&&!validReceipt(e))throw new Error('上传回执无法读取，原始队列已保留。');
    if(e.status==='linked'&&(e.write.kind!=='catalog'||!e.replacement_id||!e.receipt||!('kind' in e.receipt)||e.receipt.kind!=='catalog'||e.receipt.value.id!==e.replacement_id))throw new Error('选项核对回执不完整，原始队列已保留。');
    if(e.status==='confirmed'&&(!e.receipt||('value' in e.receipt?e.receipt.value.id:e.receipt.id)!==id||(e.write.kind==='catalog'?(!('kind' in e.receipt)||e.receipt.kind!=='catalog'):('kind' in e.receipt))))throw new Error('上传回执不完整，原始数据已保留。');
  }
  return v;
}
function validReceipt(e:QueueEntry):boolean{
 try{
  const r=e.receipt;if(!r)return false;
  if(e.write.kind==='catalog')return 'kind' in r&&r.kind==='catalog'&&Number.isSafeInteger(r.value.version)&&r.value.version>0&&typeof r.value.id==='string'&&['course','teacher','domain'].includes(r.value.input?.kind)&&typeof r.value.input.name==='string'&&typeof r.value.input.active==='boolean';
  if('kind' in r||!Number.isSafeInteger(r.version)||r.version<1||!['recorded','withdrawn','deleted'].includes(r.status)||typeof r.creator_id!=='string'||typeof r.creator_name!=='string'||typeof r.created_at!=='string'||!Array.isArray(r.acknowledgements)||!r.acknowledgements.every(id=>typeof id==='string'))return false;
  decodeLessonDrafts(JSON.stringify({[r.id+':observation']:{phase:'basic',request:{id:r.id,operation_id:'receipt-check',expected_version:r.version,action:{kind:'observation',value:r.observation}}}}));
  const validEntity=(v:unknown,kind:'goal'|'method')=>{const row=v as {id:string;version:number;input:unknown};if(!row||!Number.isSafeInteger(row.version)||row.version<1)return false;decodeDrafts(JSON.stringify({[row.id]:{id:row.id,operation_id:'receipt-check',expected_version:row.version,change_note:'',mutation:{kind,value:row.input}}}));return true;};
  if(!r.course||!r.teacher||r.course.kind!=='course'||r.teacher.kind!=='teacher'||typeof r.course.name!=='string'||typeof r.teacher.name!=='string')return false;
  const o=r.organization;return Boolean(o&&Array.isArray(o.targets)&&o.targets.every(t=>typeof t.target?.goal_id==='string'&&typeof t.target?.note==='string'&&validEntity(t.goal,'goal')&&Array.isArray(t.ancestors)&&t.ancestors.every(g=>validEntity(g,'goal')))&&Array.isArray(o.methods)&&o.methods.every(m=>validEntity(m,'method'))&&Array.isArray(o.focus_goals)&&o.focus_goals.every(g=>validEntity(g,'goal'))&&typeof o.state==='string'&&typeof o.outcome==='string'&&Number.isSafeInteger(r.observation_version)&&Number.isSafeInteger(r.organization_version)&&Number.isSafeInteger(r.request_generation)&&typeof r.marked_error==='boolean');
 }catch{return false;}
}
export type QueueIO = {
  read:()=>Promise<string|null>;write:(raw:string)=>Promise<void>;
  lock:<T>(fn:()=>Promise<T>)=>Promise<T>;
  identity:()=>Promise<QueueIdentity>;runtime:()=>Promise<{model:string;contract:string}>;state:()=>Promise<Snapshot>;
  send:(write:QueueWrite)=>Promise<Saved|Lesson>;online:()=>boolean;active:()=>boolean;
  accepted:(receipt:Saved|Lesson)=>void;changed:(outbox:Outbox)=>void;
};
export class LessonOutbox {
  value=emptyOutbox();ready=false;private busy:Promise<void>|null=null;
  readonly scope:{user:string;space:string};private io:QueueIO;
  constructor(scope:{user:string;space:string},io:QueueIO){this.scope=scope;this.io=io;}
  async load(){return this.io.lock(async()=>{this.value=decodeOutbox(await this.io.read());this.ready=true;this.io.changed(this.value);});}
  private async save(value:Outbox){await this.io.write(JSON.stringify(value));this.value=value;this.io.changed(value);}
  async enqueue(write:QueueWrite,attempted=false){
    if(!this.ready)throw new Error('本机队列尚未读入，请保留填写。');
    await this.io.lock(async()=>{
      const next=decodeOutbox(await this.io.read()),id=write.value.id,old=next.entries[id];
      if(old&&JSON.stringify(old.write)!==JSON.stringify(write))throw new Error('同一课记已有待上传版本，请先核对，不能覆盖。');
      if(!old){next.entries[id]={write,status:attempted?'uncertain':'queued',attempted,reason:''};decodeOutbox(JSON.stringify(next));await this.save(next);}else{this.value=next;this.io.changed(next);}
    });
  }
  private async verifiedSnapshot(){
    const identity=await this.io.identity(),runtime=await this.io.runtime();
    if(identity.demo||!identity.user.active||identity.user.id!==this.scope.user||identity.space!==this.scope.space)throw new Error('请用原账号和空间核对待上传选项。');
    if(runtime.model!=='prd_v1'||runtime.contract!=='skarmar.prd.v1')throw new Error('页面与服务版本不兼容，请保留队列后更新。');
    const snapshot=await this.io.state();if(snapshot.schema!=='skarmar.prd.v1')throw new Error('资料版本不匹配');return snapshot;
  }
  async availableCatalogs(kind:'course'|'teacher'){const snapshot=await this.verifiedSnapshot();return snapshot.catalog.filter(c=>c.input.kind===kind&&c.input.active);}
  async resolveCatalog(originalId:string,existingId:string){
    await this.io.lock(async()=>{
      const next=decodeOutbox(await this.io.read()),entry=next.entries[originalId];
      if(!entry||entry.write.kind!=='catalog'||entry.status!=='blocked'||entry.write.value.mutation.kind!=='catalog')throw new Error('请先核对已暂停的新增选项。');
      const expectedKind=entry.write.value.mutation.value.kind;
      const snapshot=await this.verifiedSnapshot(),existing=snapshot.catalog.find(c=>c.id===existingId&&c.input.kind===expectedKind&&c.input.active);
      if(!existing)throw new Error('这个选项已停用或不存在，请重新选择。');
      if(snapshot.catalog.some(c=>c.id===originalId))throw new Error('原编号已在服务器存在，请先重试核对原操作回执。');
      const dependents=Object.values(next.entries).filter(e=>e.write.kind==='lesson'&&e.write.value.action.kind==='observation'&&(e.write.value.action.value.course_id===originalId||e.write.value.action.value.teacher_id===originalId));
      if(dependents.some(e=>e.attempted!==false||e.status==='uncertain'||e.status==='confirmed'))throw new Error('有关课记曾尝试上传，请先核对回执，不能直接更换原请求。');
      for(const dependent of dependents){if(dependent.write.kind!=='lesson'||dependent.write.value.action.kind!=='observation')continue;
        const obs=dependent.write.value.action.value;dependent.write={kind:'lesson',value:{...dependent.write.value,operation_id:globalThis.crypto.randomUUID(),action:{kind:'observation',value:{...obs,course_id:obs.course_id===originalId?existingId:obs.course_id,teacher_id:obs.teacher_id===originalId?existingId:obs.teacher_id}}}};dependent.status='queued';dependent.reason='';
      }
      entry.status='linked';entry.replacement_id=existing.id;entry.receipt={kind:'catalog',value:existing};entry.reason='经本人核对，改用已有选项';await this.save(next);this.io.accepted(entry.receipt);
    });return this.flush();
  }
  async resume(id:string){await this.io.lock(async()=>{
    const next=decodeOutbox(await this.io.read()),e=next.entries[id];
    if(e?.status==='blocked'){e.status=e.attempted?'uncertain':'queued';e.reason='';await this.save(next);}
  });return this.flush();}
  async cancel(id:string):Promise<string[]> {
    if(!this.ready)throw new Error('本机队列尚未读入，请保留填写。');
    return this.io.lock(async()=>{
      const next=decodeOutbox(await this.io.read()),entry=next.entries[id];
      if(!entry||entry.status!=='queued'||entry.attempted!==false)throw new Error('只有从未尝试上传的待上传内容可以取消；响应未知或已暂停的内容必须先核对。');
      const lessonUses=(candidate:QueueEntry,catalogId:string)=>candidate.write.kind==='lesson'&&candidate.write.value.action.kind==='observation'&&(candidate.write.value.action.value.course_id===catalogId||candidate.write.value.action.value.teacher_id===catalogId);
      if(entry.write.kind==='catalog'&&Object.values(next.entries).some(candidate=>candidate!==entry&&lessonUses(candidate,id)))throw new Error('这个选项正被待上传课记使用；请取消相应课记，系统会处理不再使用的选项。');
      const removed=[id];delete next.entries[id];
      if(entry.write.kind==='lesson'&&entry.write.value.action.kind==='observation'){
        const dependencies=[entry.write.value.action.value.course_id,entry.write.value.action.value.teacher_id];
        for(const dependency of new Set(dependencies)){
          const candidate=next.entries[dependency];
          if(candidate?.write.kind==='catalog'&&candidate.status==='queued'&&candidate.attempted===false&&!Object.values(next.entries).some(value=>lessonUses(value,dependency))){
            delete next.entries[dependency];removed.push(dependency);
          }
        }
      }
      decodeOutbox(JSON.stringify(next));await this.save(next);return removed;
    });
  }
  // May only remove confirmed receipts. An ambiguous request must remain immutable.
  async forget(id:string){await this.io.lock(async()=>{
    const next=decodeOutbox(await this.io.read());if(next.entries[id]?.status!=='confirmed')return;
    delete next.entries[id];await this.save(next);
  });}
  async flush(){
    // A submission can be enqueued while an earlier scan owns the storage lock.
    // Joining that old scan can leave the new record waiting for the timer.
    // Each requested flush therefore gets a scan after all earlier scans finish.
    const task=(this.busy??Promise.resolve()).catch(()=>undefined).then(()=>this.run());
    this.busy=task;
    try{await task;}finally{if(this.busy===task)this.busy=null;}
  }
  private async run(){
    if(!this.ready||!this.io.online()||!this.io.active())return;
    await this.io.lock(async()=>{
      const next=decodeOutbox(await this.io.read());this.value=next;this.io.changed(next);
      const pending=Object.values(next.entries).filter(e=>e.status==='queued'||e.status==='uncertain');if(!pending.length)return;
      // A cookie can change in another tab. Both the preflight and the server
      // transaction check identity; never use a cached identity to authorize replay.
      const identity=await this.io.identity();
      if(identity.demo||!identity.user.active||identity.user.id!==this.scope.user||identity.space!==this.scope.space){
        for(const e of pending){e.status='blocked';e.reason='当前账号或空间已变化，请登录原账号后核对并继续。';}await this.save(next);return;
      }
      const runtime=await this.io.runtime();
      if(runtime.model!=='prd_v1'||runtime.contract!=='skarmar.prd.v1'){
        for(const e of pending){e.status='blocked';e.reason='页面与服务版本不兼容，请保留队列后更新页面并核对。';}await this.save(next);return;
      }
      const snapshot=await this.io.state();if(snapshot.schema!=='skarmar.prd.v1')throw new Error('服务模型不匹配，已暂停队列。');
      // Stable catalog IDs must exist before any dependent lesson is attempted.
      const entries=Object.entries(next.entries).sort((a,b)=>Number(a[1].write.kind==='lesson')-Number(b[1].write.kind==='lesson'));
      for(const [id,e] of entries){
        if(!this.io.active()||!this.io.online())break;
        if(e.status==='confirmed'||e.status==='linked'){this.io.accepted(e.receipt!);continue;}
        if(e.status==='blocked')continue;
        if(e.write.kind==='lesson'&&e.status!=='uncertain'){
          const obs=e.write.value.action;if(obs.kind!=='observation')throw new Error('无法重放该课记');
          const invalid=([['course',obs.value.course_id],['teacher',obs.value.teacher_id]] as const).find(([kind,cid])=>{
            const dep=next.entries[cid];if(dep&&dep.status!=='confirmed'&&dep.status!=='linked')return true;
            const current=snapshot.catalog.find(c=>c.id===cid)??(dep?.receipt&&'value' in dep.receipt?dep.receipt.value as Catalog:undefined);
            return !current||current.input.kind!==kind||!current.input.active;
          });
          if(invalid){e.status='blocked';e.reason='课程或老师尚未创建、已停用或需要核对；原课记保留，暂不计节数。';await this.save(next);continue;}
        }
        // Persist the exact ambiguous request BEFORE sending. Reopening/retrying
        // retrieves the original operation result even if the response was lost.
        e.status='uncertain';e.attempted=true;e.reason='正在核对同一次操作的服务器回执';await this.save(next);
        let result:Saved|Lesson;
        try{result=await this.io.send(e.write);}catch(error){
          const status=(error as {status?:number})?.status;
          if(status&&[400,401,403,404,409,413,422].includes(status))e.status='blocked';
          e.reason=error instanceof Error?error.message:'连接中断，请联网后重试同一次操作';await this.save(next);
          if(e.status==='uncertain'||status===401||status===403)break;continue;
        }
        // Even a local cleanup failure must not claim that a confirmed write failed.
        const resultId='value' in result?result.value.id:result.id;
        if(resultId!==id||(e.write.kind==='catalog'?(!('kind' in result)||result.kind!=='catalog'):('kind' in result)))throw new Error('服务器回执格式不匹配，原请求已保留待核对。');
        e.status='confirmed';e.receipt=result;e.reason='服务器已确认';this.io.accepted(result);
        try{await this.save(next);}catch{throw new Error('服务器已确认保存，本机回执暂未保存；请保留浏览器数据，再次核对不会重复记录。');}
      }
    });
  }
}

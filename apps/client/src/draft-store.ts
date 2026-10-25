/** Local draft writes are per-entry intents, serialized across windows. The
 * server operation and its exact pending body remain untouched. */
export type DraftBackup<T>={entry:string;at:string;local?:T;remote?:T};
export type DraftBackups<T>={schema:'skarmar.draft-backups.v1';items:Record<string,DraftBackup<T>>};
export type DraftIO={read:()=>Promise<string|null>;write:(raw:string)=>Promise<void>;readBackups:()=>Promise<string|null>;writeBackups:(raw:string)=>Promise<void>;lock:<R>(fn:()=>Promise<R>)=>Promise<R>;id:()=>string;now:()=>string;changed:()=>void};
const same=(a:unknown,b:unknown)=>JSON.stringify(a)===JSON.stringify(b);
const own=(v:object,k:string)=>Object.hasOwn(v,k);
export class DraftConflictError extends Error {constructor(){super('另一窗口已修改这份草稿；本窗口填写已另存保留，请先核对。');}}
export class DraftStore<T>{
 private base:Record<string,T>;private local:Record<string,T>;private dirty=new Map<string,T|undefined>();private serial:Promise<void>=Promise.resolve();private generation=0;private owner:string;
 private decode:(raw:string|null)=>Record<string,T>;private io:DraftIO;
 constructor(decode:(raw:string|null)=>Record<string,T>,initial:Record<string,T>,io:DraftIO){this.decode=decode;this.io=io;this.base={...initial};this.local={...initial};this.owner=io.id();}
 async backups():Promise<DraftBackups<T>>{
  const raw=await this.io.readBackups();if(raw===null)return {schema:'skarmar.draft-backups.v1',items:{}};
  const v=JSON.parse(raw);if(!v||v.schema!=='skarmar.draft-backups.v1'||!v.items||typeof v.items!=='object'||Array.isArray(v.items))throw new Error('保留的窗口填写无法读取，原始内容未改动。');
  for(const b of Object.values(v.items) as DraftBackup<T>[]){if(!b||typeof b.entry!=='string'||typeof b.at!=='string')throw new Error('保留的窗口填写格式异常，原内容未改动。');for(const kind of ['local','remote'] as const)if(own(b,kind))this.decode(JSON.stringify({[b.entry]:b[kind]}));}
  return v;
 }
 private async backup(items:Record<string,DraftBackup<T>>){const old=await this.backups();await this.io.writeBackups(JSON.stringify({...old,items:{...old.items,...items}}));this.io.changed();}
 save(next:Record<string,T>):Promise<void>{
  try{this.decode(JSON.stringify(next));}catch(e){return Promise.reject(e);}
  for(const k of new Set([...Object.keys(this.local),...Object.keys(next)]))if(!same(this.local[k],next[k]))this.dirty.set(k,next[k]);
  this.local={...next};++this.generation;
  const task=this.serial.catch(()=>undefined).then(()=>this.flush());this.serial=task;return task;
 }
 private async flush(){if(!this.dirty.size)return;
  await this.io.lock(async()=>{
   const edits=new Map(this.dirty),current=this.decode(await this.io.read()),conflicts=[...edits].filter(([k,v])=>!same(current[k],this.base[k])&&!same(current[k],v));
   if(conflicts.length){await this.backup(Object.fromEntries([...edits].map(([k,local])=>[this.owner+':'+k,{entry:k,at:this.io.now(),...(local===undefined?{}:{local}),...(current[k]===undefined?{}:{remote:current[k]})}])));throw new DraftConflictError();}
   const merged={...current};for(const [k,v]of edits){if(v===undefined)delete merged[k];else merged[k]=v;}
   this.decode(JSON.stringify(merged));await this.io.write(JSON.stringify(merged));
   for(const [k,v]of edits){if(v===undefined)delete this.base[k];else this.base[k]=v;if(same(this.dirty.get(k),v))this.dirty.delete(k);}
   this.io.changed();
  });
 }
 /** Explicit reconciliation backs up current local content before accepting
  * the shared root. Editing while this is in flight cancels adoption. */
 adopt():Promise<Record<string,T>>{
  const task=this.serial.catch(()=>undefined).then(()=>this.io.lock(async()=>{
   const generation=this.generation,current=this.decode(await this.io.read()),copies:Record<string,DraftBackup<T>>={};
   for(const [k,local]of Object.entries(this.local))if(!same(local,current[k]))copies[this.io.id()]={entry:k,at:this.io.now(),local,...(current[k]===undefined?{}:{remote:current[k]})};
   if(Object.keys(copies).length)await this.backup(copies);
   if(generation!==this.generation)throw new Error('核对期间填写有更新，原内容仍保留，请再次核对。');
   this.base={...current};this.local={...current};this.dirty.clear();return current;
  }));this.serial=task.then(()=>undefined,()=>undefined);return task;
 }
 restore(backupId:string):Promise<{entry:string;value:T}>{
  const task=this.serial.catch(()=>undefined).then(()=>this.io.lock(async()=>{
   const generation=this.generation,copies=await this.backups(),b=copies.items[backupId];if(!b||b.local===undefined)throw new Error('这份保留记录没有可载入的填写。');
   const current=this.decode(await this.io.read()),prior=current[b.entry],local=this.local[b.entry],snapshots:Record<string,DraftBackup<T>>={};
   if(prior!==undefined)snapshots[this.io.id()]={entry:b.entry,at:this.io.now(),local:prior};
   if(local!==undefined&&!same(local,prior))snapshots[this.io.id()]={entry:b.entry,at:this.io.now(),local};
   if(Object.keys(snapshots).length)await this.backup(snapshots);
   if(generation!==this.generation)throw new Error('载入期间填写有更新，原内容仍保留，请再次核对。');
   const merged={...current,[b.entry]:b.local};this.decode(JSON.stringify(merged));await this.io.write(JSON.stringify(merged));
   this.base[b.entry]=b.local;this.local[b.entry]=b.local;this.dirty.delete(b.entry);this.io.changed();return {entry:b.entry,value:b.local};
  }));this.serial=task.then(()=>undefined,()=>undefined);return task;
 }
}
const displayNames:Record<string,string>={title:'名称',name:'名称',content:'内容',plan:'安排',observation:'实际观察',note:'说明',notes:'观察',meaning:'生活意义',criteria:'观察标准',description:'旧做法说明',steps:'具体做法',observe:'观察意图',scope:'适用条件',guidance:'指导来源',expected:'期望',adult_response:'成人回应',guess:'原因猜测',conditions:'条件',evidence:'依据',text:'观察',support:'帮助条件',basis:'比较口径',business_date:'日期',time_slot:'时段',teacher_notes:'老师建议',progress_notes:'进展观察',interest_notes:'兴趣与互动',difficulty_notes:'困难观察',review_message:'给家人的留言',change_note:'修改说明'};
export function draftText(value:unknown):string[]{const lines:string[]=[];function walk(v:unknown){if(!v||typeof v!=='object')return;for(const [key,x]of Object.entries(v)){if(key==='pending'||key==='backup'||key==='conflict_backup')continue;if(displayNames[key]&&typeof x==='string'&&x.trim())lines.push(displayNames[key]+'：'+x);else if(x&&typeof x==='object')walk(x);}}walk(value);return [...new Set(lines)];}

import {DetailOverlay} from './overlay';
import React,{createContext,useContext,useEffect,useMemo,useRef,useState} from 'react';
import AsyncStorage from '@react-native-async-storage/async-storage';
import {AppState,Platform,View} from 'react-native';
import {request} from './api';
import {useAuth} from './auth';
import {Pressable,Text} from './readability';
import {LessonOutbox,emptyOutbox,type Outbox,type QueueIdentity} from './prd-outbox';
import {type Saved,type Snapshot,type Catalog,emptySnapshot,mergeSnapshot} from './foundation';
import {type Lesson} from './prd-lessons';
type Service={queue:LessonOutbox;value:Outbox;ready:boolean;error:string;sync:()=>Promise<void>;catalog:Catalog[]};
const Context=createContext<Service|null>(null);
export function useLessonOutbox(){const value=useContext(Context);if(!value)throw new Error('课记上传服务尚未加载');return value;}
const localLocks=new Map<string,Promise<unknown>>();
async function locked<T>(key:string,fn:()=>Promise<T>):Promise<T>{
  if(Platform.OS==='web'){
    if(!navigator.locks)throw new Error('此浏览器暂不支持安全的多窗口上传，请更新Safari或Chrome；原填写不会清除。');
    return navigator.locks.request(key,fn);
  }
  const last=localLocks.get(key)??Promise.resolve(),task=last.catch(()=>undefined).then(fn);localLocks.set(key,task);return task;
}
async function timed<T>(path:string,method='GET',body?:unknown,space?:string){
  const abort=new AbortController(),timer=setTimeout(()=>abort.abort(),15000);
  try{return await request<T>(path,method,body,{signal:abort.signal,space});}finally{clearTimeout(timer);}
}
export function LessonOutboxProvider({children,accepted,cacheLoaded,data}:{children:React.ReactNode;accepted:(r:Saved|Lesson)=>void;cacheLoaded:(s:Snapshot)=>void;data:Snapshot}){
  const {identity}=useAuth(),scope=`${identity.space}:${identity.user.id}`,key=`skarmar:${scope}:lesson-queue:v1`,cacheKey=`skarmar:${scope}:snapshot:v1`;
  const [value,setValue]=useState(emptyOutbox),[ready,setReady]=useState(false),[error,setError]=useState('');
  const alive=useRef(true),callbacks=useRef({accepted,cacheLoaded});callbacks.current={accepted,cacheLoaded};
  const queue=useMemo(()=>new LessonOutbox({user:identity.user.id,space:identity.space},{
    read:()=>AsyncStorage.getItem(key),write:raw=>AsyncStorage.setItem(key,raw),lock:fn=>locked(key,fn),
    online:()=>Platform.OS!=='web'||navigator.onLine,active:()=>alive.current,
    identity:()=>timed<QueueIdentity>('/auth/me'),runtime:()=>timed('/runtime'),state:()=>timed<Snapshot>('/prd/state'),
    send:write=>timed<Saved|Lesson>('/prd/queue','POST',{user_id:identity.user.id,write},identity.space),
    accepted:r=>{if(alive.current)callbacks.current.accepted(r);},changed:v=>{if(alive.current)setValue({...v,entries:{...v.entries}});}
  }),[key,identity.user.id,identity.space]);
  const sync=async()=>{try{await queue.flush();if(alive.current)setError('');}catch(e){if(alive.current)setError(e instanceof Error?e.message:'上传暂未完成，输入仍保留。');}};
  const syncRef=useRef(sync);syncRef.current=sync;
  useEffect(()=>{
    alive.current=true;
    void queue.load().then(()=>{if(alive.current){setReady(true);void syncRef.current();}}).catch(e=>{if(alive.current)setError(e instanceof Error?e.message:'队列读取失败');});
    // This cache is only displayed inside the already authenticated actor/space.
    // It never authorizes replay or replaces the current server state.
    void AsyncStorage.getItem(cacheKey).then(raw=>{
      if(!raw||!alive.current)return;const v=JSON.parse(raw) as {scope:string;data:Snapshot};
      if(v.scope!==scope||v.data?.schema!=='skarmar.prd.v1'||!Array.isArray(v.data.catalog)||v.data.catalog.some(c=>!c||typeof c.id!=='string'||!Number.isSafeInteger(c.version)||!c.input||!['course','teacher'].includes(c.input.kind)||typeof c.input.name!=='string'||typeof c.input.active!=='boolean'))throw new Error('本机快照格式不兼容');
      callbacks.current.cacheLoaded({...emptySnapshot,catalog:v.data.catalog});
    }).catch(()=>{if(alive.current)setError('本机课程老师缓存暂不可用，请联网刷新；上传队列独立保留。');});
    const sync=()=>{if(Platform.OS!=='web'||document.visibilityState==='visible')void syncRef.current();};
    const timer=setInterval(sync,30000),subscription=AppState.addEventListener('change',s=>{if(s==='active')sync();});
    if(Platform.OS==='web'){window.addEventListener('online',sync);window.addEventListener('focus',sync);window.addEventListener('storage',sync);document.addEventListener('visibilitychange',sync);}
    return()=>{alive.current=false;clearInterval(timer);subscription.remove();if(Platform.OS==='web'){window.removeEventListener('online',sync);window.removeEventListener('focus',sync);window.removeEventListener('storage',sync);document.removeEventListener('visibilitychange',sync);}};
  },[queue,key]);
  useEffect(()=>{if(!ready)return;let cancelled=false;void locked(cacheKey,async()=>{
    if(cancelled)return;const raw=await AsyncStorage.getItem(cacheKey);let old=emptySnapshot;
    if(raw){const v=JSON.parse(raw) as {scope:string;data:Snapshot};if(v.scope!==scope||v.data.schema!=='skarmar.prd.v1'||!Array.isArray(v.data.catalog))throw new Error('本机课程老师缓存不兼容');old={...emptySnapshot,catalog:v.data.catalog};}
    await AsyncStorage.setItem(cacheKey,JSON.stringify({scope,data:{...emptySnapshot,catalog:mergeSnapshot(old,{...emptySnapshot,catalog:data.catalog}).catalog}}));
  }).catch(()=>{if(alive.current)setError('本机课程老师缓存未能更新；正式保存结果以服务器回执为准。');});return()=>{cancelled=true;};},[data,ready,cacheKey]);
  const catalog=Object.values(value.entries).filter(e=>e.write.kind==='catalog'&&e.status!=='confirmed'&&e.status!=='linked').map(e=>{const r=e.write.value as import('./foundation').Write;return {id:r.id,version:0,input:r.mutation.value} as Catalog;});
  return <Context.Provider value={{queue,value,ready,error,sync,catalog}}>{children}</Context.Provider>;
}
export function LessonQueueStatus(){
  const {queue,value,ready,error,sync}=useLessonOutbox();const [resolveId,setResolveId]=useState<string|null>(null),[options,setOptions]=useState<Catalog[]>([]),[problem,setProblem]=useState(''),[resolving,setResolving]=useState(false);const pending=Object.values(value.entries).filter(e=>e.status!=='confirmed'&&e.status!=='linked');
  if(!error&&!problem&&!pending.length&&!resolveId)return null;
  const button=(label:string,action:()=>void)=><Pressable accessibilityRole="button" onPress={action} style={{padding:12,borderRadius:8,backgroundColor:'#E9F2E9'}}><Text>{label}</Text></Pressable>;
  return <View style={{gap:10,padding:12,borderRadius:10,backgroundColor:'#FFF5DE'}}>
    <Text accessibilityRole="header">待上传课记与选项：{pending.length}</Text><Text>只保存在本机的内容不计正式课时。重开页面联网后继续；关闭网页时不保证后台上传。</Text>
    {error&&<Text accessibilityRole="alert">{error}</Text>}{problem&&<Text accessibilityRole="alert">{problem}</Text>}
    {pending.map(e=><View key={e.write.value.id} style={{gap:6}}><Text>{e.write.kind==='catalog'&&e.write.value.mutation.kind==='catalog'?`新增${e.write.value.mutation.value.kind==='course'?'课程':'老师'}：${e.write.value.mutation.value.name}`:e.write.kind==='lesson'&&e.write.value.action.kind==='observation'?`课记：${e.write.value.action.value.business_date}`:'课记'} · {e.status==='blocked'?'已暂停':e.status==='uncertain'?'待核对回执':'待上传'}</Text>{e.reason&&<Text>{e.reason}</Text>}{e.status==='blocked'&&button(`核对后继续上传：${e.write.value.id.slice(0,8)}`,()=>void queue.resume(e.write.value.id).catch(()=>undefined))}{e.status==='blocked'&&e.write.kind==='catalog'&&button(`改用已有选项：${e.write.value.mutation.kind==='catalog'?e.write.value.mutation.value.name:''}`,()=>void(async()=>{setProblem('');try{if(e.write.kind==='catalog'&&e.write.value.mutation.kind==='catalog'){const rows=await queue.availableCatalogs(e.write.value.mutation.value.kind);setOptions(rows);setResolveId(e.write.value.id);}}catch(err){setProblem(err instanceof Error?err.message:'暂时无法核对选项');}})())}</View>)}
    {ready&&button('检查连接并继续上传',()=>void sync())}
    {resolveId&&<DetailOverlay title="核对待上传的课程或老师" close={()=>{if(!resolving)setResolveId(null);}}><Text>选择服务器已有的正确选项。只有尚未尝试上传的课记会据此调整；原填写保留，不自动合并相似名称。</Text>{problem&&<Text accessibilityRole="alert">{problem}</Text>}{options.map(option=><Pressable key={option.id} accessibilityRole="button" disabled={resolving} onPress={()=>void(async()=>{setResolving(true);setProblem('');try{await queue.resolveCatalog(resolveId,option.id);setResolveId(null);}catch(err){setProblem(err instanceof Error?err.message:'暂未核对完成');}finally{setResolving(false);}})()} style={{padding:12,borderRadius:8,backgroundColor:'#E9F2E9'}}><Text>核对并改用：{option.input.name}</Text></Pressable>)}{!options.length&&<Text>目前没有可用选项，可先由家人维护目录后再核对。</Text>}</DetailOverlay>}

  </View>;
}

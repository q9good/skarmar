import React,{useSyncExternalStore} from 'react';
import {Text} from './readability';
type Status='saving'|'saved'|'failed';
const values=new Map<string,Status>(),latest=new Map<string,number>(),listeners=new Map<string,Set<()=>void>>();
const emit=(key:string,status:Status)=>{values.set(key,status);listeners.get(key)?.forEach(fn=>fn());};
export function trackLocalSave<T>(key:string,promise:Promise<T>):Promise<T>{
 const seq=(latest.get(key)??0)+1;latest.set(key,seq);emit(key,'saving');
 void promise.then(()=>{if(latest.get(key)===seq)emit(key,'saved');},()=>{if(latest.get(key)===seq)emit(key,'failed');});return promise;
}
export function SaveStatus({storageKey,sharing,pending}:{storageKey:string;sharing:boolean;pending:boolean}){
 const local=useSyncExternalStore(fn=>{const set=listeners.get(storageKey)??new Set();set.add(fn);listeners.set(storageKey,set);return()=>{set.delete(fn);};},()=>values.get(storageKey)??'saved',()=> 'saved' as Status);
 return <Text role="status">{sharing?'正在保存，请稍候…':pending?'上次结果尚未确认，重试将核对同一次操作。':local==='saving'?'正在保存在这台设备…':local==='failed'?'本机填写尚未保存，请保留页面后重试。':'已自动保存在这台设备；提交后家人才能看到。'}</Text>;
}

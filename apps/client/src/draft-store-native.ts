import AsyncStorage from '@react-native-async-storage/async-storage';
import {Platform} from 'react-native';
import {DraftStore} from './draft-store';
const locks=new Map<string,Promise<unknown>>(),listeners=new Map<string,Set<()=>void>>();
export const backupKey=(key:string)=>key+':window-backups:v1';
export const subscribeDrafts=(key:string,fn:()=>void)=>{const set=listeners.get(key)??new Set();set.add(fn);listeners.set(key,set);return()=>{set.delete(fn);};};
export function createDraftStore<T>(key:string,decode:(raw:string|null)=>Record<string,T>,initial:Record<string,T>){
 return new DraftStore(decode,initial,{read:()=>AsyncStorage.getItem(key),write:raw=>AsyncStorage.setItem(key,raw),readBackups:()=>AsyncStorage.getItem(backupKey(key)),writeBackups:raw=>AsyncStorage.setItem(backupKey(key),raw),id:()=>globalThis.crypto.randomUUID(),now:()=>new Date().toISOString(),changed:()=>listeners.get(key)?.forEach(fn=>fn()),lock:async<R>(fn:()=>Promise<R>)=>{if(Platform.OS==='web'){if(!navigator.locks)throw new Error('此浏览器不支持安全的多窗口草稿保存，请更新Safari或Chrome；当前填写仍保留。');return navigator.locks.request('skarmar-draft:'+key,fn);}const task=(locks.get(key)??Promise.resolve()).catch(()=>undefined).then(fn);locks.set(key,task);return task;}});
}

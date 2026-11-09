import React,{useEffect,useRef,useState} from 'react';
import {View} from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import {Text} from './readability';
import {GuidedField} from './guided-field';
import {UiButton} from './ui-components';
import {DetailOverlay} from './overlay';
import {useAuth} from './auth';
import {request,ApiError,authSessionGuard} from './api';
import {dataKey} from './data-epoch';
import {uuid} from './draft';
import {defaultPriorityPolicy} from './object-priority';
import type {PriorityPolicy} from './foundation';
type Pending={kind:'priority_settings';value:{operation_id:string;expected_version:number;value:PriorityPolicy}};
export function PriorityPolicyPanel({initial,close,saved}:{initial?:PriorityPolicy;close:()=>void;saved:(policy:PriorityPolicy)=>Promise<void>}){
 const auth=useAuth(),key=dataKey(`skarmar:${auth.identity.space}:${auth.identity.user.id}:priority-policy:v1`),[value,setValue]=useState(initial??defaultPriorityPolicy),[pending,setPending]=useState<Pending|null>(null),[ready,setReady]=useState(false),[busy,setBusy]=useState(false),[error,setError]=useState('');const serial=useRef(Promise.resolve()),flight=useRef(false);
 const store=(v:PriorityPolicy,p:Pending|null)=>{const task=serial.current.catch(()=>undefined).then(()=>AsyncStorage.setItem(key,JSON.stringify({value:v,pending:p})));serial.current=task;return task;};
 useEffect(()=>{let alive=true;void AsyncStorage.getItem(key).then(raw=>{if(!alive)return;if(raw){const d=JSON.parse(raw);if(!d.value||!['version','minimum','maximum','method_per_goal','method_bonus_cap'].every(k=>Number.isSafeInteger(d.value[k])))throw new Error('本机规则填写无法读取，请保留浏览器数据');setValue(d.value);setPending(d.pending??null);}setReady(true);}).catch(e=>{if(alive)setError(String(e.message));});return()=>{alive=false;};},[key]);
 return <DetailOverlay title="数字优先级规则" close={()=>{if(!busy)close();}}><Text>数字越大越靠前。方法有效分＝基础分＋关联在用目标数×加分（不超过上限）。修改范围只约束有效分，不改写旧记录的原始分值。临时优先是独立展示规则。</Text>
 {(['minimum','maximum','method_per_goal','method_bonus_cap'] as const).map(k=><GuidedField key={k} label={{minimum:'基础分最小值',maximum:'基础分最大值',method_per_goal:'每个关联目标加分',method_bonus_cap:'关联加分上限'}[k]} value={String(value[k])} numeric disabled={!ready||busy||Boolean(pending)} change={raw=>{if(/^\d{0,5}$/.test(raw)){const next={...value,[k]:Number(raw||0)};setValue(next);void store(next,null).catch(e=>setError(e.message));}}}/>)}
 {error&&<Text accessibilityRole="alert">{error}</Text>}
 <UiButton title={busy?'正在保存规则…':pending?'重试同一次规则保存':'保存优先级规则'} disabled={!ready||busy} press={()=>void(async()=>{if(flight.current)return;flight.current=true;setBusy(true);setError('');const p=pending??{kind:'priority_settings' as const,value:{operation_id:uuid(),expected_version:value.version,value}};const guard=authSessionGuard();try{await store(value,p);setPending(p);guard();const confirmed=await request<PriorityPolicy>('/prd/process','POST',p);await AsyncStorage.removeItem(key);setPending(null);await saved(confirmed);close();}catch(e){if(e instanceof ApiError&&[400,401,403,409,413,422].includes(e.status)){setPending(null);await store(value,null).catch(()=>undefined);}setError(e instanceof Error?e.message:'规则保存未确认，请保留填写并重试');}finally{setBusy(false);flight.current=false;}})()}/>
 {!pending&&<UiButton title="保留填写并载入最新规则" disabled={busy||!ready} press={()=>void(async()=>{try{await AsyncStorage.setItem(key+':backup:'+Date.now(),JSON.stringify(value));const next=await request<PriorityPolicy>('/prd/process?view=priority_settings');setValue(next);await store(next,null);setError('');}catch(e){setError(e instanceof Error?e.message:'规则暂未读到');}})()}/>}<UiButton title="保留规则草稿并退出" disabled={busy} press={close}/>
 </DetailOverlay>;
}

import React from 'react';
import {View} from 'react-native';
import {Text} from './readability';
import {GuidedField} from './guided-field';
import {UiCheck,UiButton} from './ui-components';
import {DateField} from './controls';
import {localDate} from './draft';
import {priorityPolicy,priorityValue,twoWeekPriority} from './object-priority';
import type {GoalInput,MethodInput,Snapshot} from './foundation';
export function PriorityFields({value,kind,data,change,disabled=false}:{value:GoalInput|MethodInput;kind:'goal'|'method';data:Snapshot;change:(v:Record<string,unknown>)=>void;disabled?:boolean}){
 const p=priorityPolicy(data),v=priorityValue(data,{id:'draft',version:1,input:value} as never,kind,localDate());
 return <View style={{gap:8}}><GuidedField label={kind==='goal'?'目标数字优先级':'方法基础优先级'} question={`数字越大越靠前，范围${p.minimum}至${p.maximum}`} value={String(value.priority_score??v.base)} change={raw=>{if(/^\d{0,5}$/.test(raw))change({priority_score:Number(raw||0)});}} numeric disabled={disabled}/>
 <Text>基础分范围 {p.minimum}–{p.maximum}，越大越靠前。{kind==='method'?`每个关联的在用目标加${p.method_per_goal}分，上限${p.method_bonus_cap}分；按保存后的真实关系计算。`:''}</Text>
 <UiCheck title="临时优先展示两周" checked={Boolean(value.priority_until&&value.priority_until>=localDate())} disabled={disabled} press={()=>change({priority_until:value.priority_until&&value.priority_until>=localDate()?null:twoWeekPriority(localDate())})}/>
 {value.priority_until&&<><Text>临时优先至 {value.priority_until}（含当天），之后恢复原分。</Text><UiButton title="从今天重新设置两周" disabled={disabled} press={()=>change({priority_until:twoWeekPriority(localDate())})}/><DateField label="临时优先截止日期" value={value.priority_until} disabled={disabled} onChange={date=>change({priority_until:date||null})}/></>}
 </View>;
}

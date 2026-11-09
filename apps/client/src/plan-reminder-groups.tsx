import React,{useState} from 'react';
import {View} from 'react-native';
import {Text} from './readability';
import {UiButton,UiSection} from './ui-components';
import {priorityLabel} from './object-priority';
import {planFocus,recentReminders} from './plan-reminders';
import type {Snapshot} from './foundation';
import type {Schedule} from './prd-followups';
export function PlanReminderGroups({data,today,schedules,openGoal,openMethod,openKnowledge}:{data:Snapshot;today:string;schedules?:Schedule[];openGoal:(id:string)=>void;openMethod:(id:string)=>void;openKnowledge:(k:Snapshot['knowledge'][number])=>void}){
 const [all,setAll]=useState(false),focus=planFocus(data,today,schedules),recent=recentReminders(data,today,all),total=recentReminders(data,today,true).length;
 const names={goal:'目标',method:'方法',experience:'经验',difficulty:'困难'};
 return <View testID="plan-reminders" style={{gap:14}}><UiSection title="优先提醒 · 前6项" tone="family" description="与今天的长期关注使用同一筛选和排序。只是提醒，不要求本次全部完成。"><View testID="plan-priority-reminders" style={{gap:8}}>{focus.map(({kind,value},i)=><UiButton key={kind+value.id} title={`计划提醒${names[kind]}：${value.input.title}`} label={`${i+1}. ${names[kind]} · ${value.input.title} · ${priorityLabel(data,value,kind,today)}`} tone={kind} press={()=>kind==='goal'?openGoal(value.id):openMethod(value.id)}/>)}{!focus.length&&<Text>还没有当前长期关注。可以先安排轻松的活动。</Text>}</View></UiSection>
 <UiSection title="最近新增" tone="family" description="最近两周，每类先显示3项。按创建时间倒排，不把修改当新增。"><View testID="plan-recent-reminders" style={{gap:8}}>{recent.map(r=><UiButton key={r.kind+r.id} title={`最近新增${names[r.kind]}：${r.title}`} label={`${names[r.kind]} · ${r.title}`} tone={r.kind} press={()=>{if(r.kind==='goal')openGoal(r.id);else if(r.kind==='method')openMethod(r.id);else{const k=data.knowledge.find(k=>k.input.id===r.id);if(k)openKnowledge(k);}}}/>)}{!recent.length&&<Text>最近两周没有新增条目。</Text>}{(all||total>recent.length)&&<UiButton title={all?'收起最近新增':'展开全部近期新增（'+total+'）'} variant="link" press={()=>setAll(v=>!v)}/>}</View></UiSection></View>;
}

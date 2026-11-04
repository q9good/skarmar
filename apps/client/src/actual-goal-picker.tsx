import React,{useState} from 'react';
import {View} from 'react-native';
import {Text} from './readability';
import {UiCheck} from './ui-components';
import {GuidedField} from './guided-field';
import {goalMethodHierarchy,type GoalBranch} from './goal-method-hierarchy';
import {neutral} from './ui-palette';
import {levelNames,goalStateName,type Snapshot} from './foundation';

export function ActualGoalPicker({data,selected,toggle,disabled}:{data:Snapshot;selected:string[];toggle:(id:string)=>void;disabled:boolean}){
 const [search,setSearch]=useState(''),term=search.trim(),tree=goalMethodHierarchy(data.goals,[]);
 const matches=(row:GoalBranch):boolean=>row.goal.input.title.includes(term)||row.children.some(matches);
 const branch=(row:GoalBranch,above=false):React.ReactNode=>{
  const own=above||row.goal.input.title.includes(term);if(term&&!own&&!matches(row))return null;
  const g=row.goal;
  return <View key={g.id} style={{gap:8,paddingLeft:10,borderLeftWidth:1,borderColor:neutral.border}}>
   {g.input.level==='long'?<Text accessibilityRole="header" style={{fontWeight:'700'}}>{g.input.title} · 长期方向</Text>:<><UiCheck title={'选择目标：'+g.input.title} checked={selected.includes(g.id)} disabled={disabled} press={()=>toggle(g.id)}/><Text style={{fontSize:14,color:neutral.muted}}>{levelNames[g.input.level]} · {goalStateName(g.input)}{g.input.archived?' · 已收起':''}</Text></>}
   {row.children.map(child=>branch(child,own))}
  </View>;
 };
 return <View style={{gap:12}}><Text>只选择本次真正涉及的目标，不需要完成参考列表。长期方向可在总结补充中明确关注。</Text><GuidedField label="搜索目标" value={search} change={setSearch} disabled={disabled} multiline={false}/><Text>已选实际目标 {selected.length} 个</Text>{tree.roots.map(row=>branch(row))}{!tree.roots.some(matches)&&<Text>没有匹配的目标；已有选择仍保留。</Text>}</View>;
}

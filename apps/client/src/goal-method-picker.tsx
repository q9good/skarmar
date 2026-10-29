import React,{useState} from 'react';
import {View} from 'react-native';
import {Pressable,Text} from './readability';
import {GuidedField} from './guided-field';
import {goalMethodHierarchy,type GoalBranch} from './goal-method-hierarchy';
import {levelNames,type Goal,type Method} from './foundation';

export function GoalMethodPicker({goals,methods,goalIds,methodIds,toggleGoal,toggleMethod,disabled=false}:{goals:Goal[];methods:Method[];goalIds:string[];methodIds:string[];toggleGoal:(id:string)=>void;toggleMethod:(id:string)=>void;disabled?:boolean}){
  const {roots,unlinked}=goalMethodHierarchy(goals,methods);
  const containsSelected=(row:GoalBranch):boolean=>goalIds.includes(row.goal.id)||row.methods.some(m=>methodIds.includes(m.id))||row.children.some(containsSelected);
  const [expanded,setExpanded]=useState<Set<string>>(()=>{
    const ids=new Set<string>();const visit=(row:GoalBranch)=>{if(containsSelected(row))ids.add(row.goal.id);row.children.forEach(visit);};roots.forEach(visit);return ids;
  });
  const [search,setSearch]=useState('');const term=search.trim();
  const matches=(row:GoalBranch):boolean=>row.goal.input.title.includes(term)||row.methods.some(m=>m.input.title.includes(term))||row.children.some(matches);
  const check=(kind:'目标'|'方法',id:string,title:string,selected:boolean,press:()=>void)=><Pressable accessibilityRole="checkbox" accessibilityLabel={`关联${kind}：${title}`} accessibilityState={{checked:selected,disabled}} aria-checked={selected} disabled={disabled} onPress={press} style={{padding:10,minHeight:44,borderRadius:8,backgroundColor:selected?'#D9EBDD':'#FFFFFF',gap:4}}><Text>{selected?'☑':'☐'} {kind}：{title}</Text></Pressable>;
  const methodRow=(method:Method)=><View key={method.id} style={{gap:4}}>{check('方法',method.id,method.input.title,methodIds.includes(method.id),()=>toggleMethod(method.id))}{method.input.goal_ids.filter(id=>goals.some(g=>g.id===id)).length>1&&<Text style={{fontSize:13,color:'#53665A'}}>同时关联多个目标，各处勾选保持一致</Text>}</View>;
  const branch=(row:GoalBranch):React.ReactNode=>{
    if(term&&!matches(row))return null;
    const {goal,children,methods:linked}=row,open=Boolean(term)||expanded.has(goal.id),hasChildren=children.length+linked.length>0;
    return <View key={goal.id} style={{gap:8,borderLeftWidth:2,borderColor:'#B3C9BA',paddingLeft:10}}>
      {check('目标',goal.id,goal.input.title,goalIds.includes(goal.id),()=>toggleGoal(goal.id))}
      {hasChildren&&<Pressable accessibilityRole="button" accessibilityLabel={`${open?'收起':'展开'}关联层级：${goal.input.title}`} accessibilityState={{expanded:open,disabled}} disabled={disabled} onPress={()=>setExpanded(old=>{const next=new Set(old);next.has(goal.id)?next.delete(goal.id):next.add(goal.id);return next;})} style={{paddingVertical:8,minHeight:44}}><Text style={{color:'#34765B'}}>{open?'▾':'▸'} 下级目标 {children.length} · 方法 {linked.length}{!open&&containsSelected(row)?' · 包含已选项':''}</Text></Pressable>}
      {open&&<View style={{gap:10,paddingLeft:8}}>{children.map(branch)}{linked.filter(m=>!term||goal.input.title.includes(term)||m.input.title.includes(term)).map(methodRow)}</View>}
    </View>;
  };
  return <View style={{gap:12}}>
    <Text>已选 {goalIds.length} 个目标、{methodIds.length} 个方法。分别勾选，可跨层级多选；选择目标不会自动勾选方法。</Text>
    {(goals.length+methods.length>15)&&<GuidedField label="查找关联目标或方法" value={search} change={setSearch} multiline={false}/>}
    {(['long','medium','short'] as const).map(level=>{const rows=roots.filter(r=>r.goal.input.level===level&&(!term||matches(r)));return rows.length?<View key={level} style={{gap:10}}><Text accessibilityRole="header" style={{fontWeight:'600'}}>{levelNames[level]}{level!=='long'?'（独立或未设上级）':''}</Text>{rows.map(branch)}</View>:null;})}
    {unlinked.some(m=>m.input.title.includes(term))&&<View style={{gap:8}}><Text accessibilityRole="header">未关联现有目标的方法</Text>{unlinked.filter(m=>m.input.title.includes(term)).map(methodRow)}</View>}
    {term&&!roots.some(matches)&&!unlinked.some(m=>m.input.title.includes(term))&&<Text>没有匹配项，已有选择仍保留。</Text>}
    {!goals.length&&!methods.length&&<Text>尚无目标或方法，可以先在对应页面创建。</Text>}
  </View>;
}

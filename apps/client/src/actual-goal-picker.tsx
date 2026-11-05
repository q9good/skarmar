import React,{useId,useState} from 'react';
import {View} from 'react-native';
import {Text,TextInput,Pressable} from './readability';
import {UiCheck,UiButton} from './ui-components';
import {goalMethodHierarchy,type GoalBranch} from './goal-method-hierarchy';
import {actualGoalRelations,visibleGoals} from './actual-goal-relations';
import {neutral,palettes} from './ui-palette';
import {levelNames,goalStateName,type Snapshot} from './foundation';

export function ActualGoalPicker({data,selected,toggle,disabled,refreshControl,inspect}:{data:Snapshot;selected:string[];toggle:(id:string)=>void;disabled:boolean;refreshControl?:React.ReactNode;inspect?:(id:string)=>void}){
 const [search,setSearch]=useState(''),[showArchived,setShowArchived]=useState(false),[expanded,setExpanded]=useState<Set<string>>(()=>actualGoalRelations(data.goals,selected).associated),prefix=useId();
 const term=search.trim(),relations=actualGoalRelations(data.goals,selected),visible=new Set(visibleGoals(data.goals,showArchived,selected).map(g=>g.id)),tree=goalMethodHierarchy(data.goals,[]);
 const retained=(row:GoalBranch):boolean=>visible.has(row.goal.id)||row.children.some(retained);
 const matches=(row:GoalBranch):boolean=>retained(row)&&(row.goal.input.title.includes(term)||row.children.some(matches));
 const branch=(row:GoalBranch,above=false):React.ReactNode=>{
  const g=row.goal,own=above||g.input.title.includes(term);if(!retained(row)||term&&!own&&!matches(row))return null;
  const children=row.children.filter(retained),open=Boolean(term)||expanded.has(g.id),direct=relations.explicit.has(g.id),derived=relations.inherited.has(g.id),checked=direct?true:derived?'mixed' as const:false,contentId=prefix+'-children-'+g.id;
  return <View key={g.id} testID={'actual-goal-branch-'+g.id} style={{gap:8,paddingLeft:8,borderLeftWidth:2,borderColor:direct||derived?palettes.family.border:neutral.border}}>
   <View style={{gap:7,padding:10,borderRadius:12,backgroundColor:neutral.surface,borderWidth:1,borderColor:direct||derived?palettes.family.border:neutral.border}}>
    <Pressable accessibilityRole="checkbox" accessibilityLabel={'选择目标：'+g.input.title} accessibilityState={{checked,disabled}} aria-checked={checked} disabled={disabled} onPress={()=>toggle(g.id)} style={{paddingVertical:10,minHeight:46}}><Text style={{fontSize:18,fontWeight:direct||derived?'700':'500',color:direct||derived?palettes.family.ink:neutral.ink}}>{direct?'☑':derived?'▣':'☐'} {g.input.title}</Text></Pressable>
    <View style={{flexDirection:'row',alignItems:'center',gap:10}}><Text style={{flex:1,fontSize:14,color:neutral.muted}}>{levelNames[g.input.level]} · {goalStateName(g.input)}{g.input.archived?' · 已收起':''}</Text>{inspect&&<UiButton title={'查看目标来源：'+g.input.title} label="详情" variant="link" press={()=>inspect(g.id)}/>}</View>
    {derived&&<Text style={{fontSize:14,color:palettes.family.ink}}>由下级自动关联{direct?'，也已直接选择':'；不必重复填写表现'}</Text>}
    {!visible.has(g.id)&&<Text style={{fontSize:14,color:neutral.muted}}>保留上级层级；直接勾选仍可关联此目标。</Text>}
    {children.length>0&&(term?<Text style={{fontSize:14,color:neutral.muted}}>匹配的下级路径已展开</Text>:<Pressable accessibilityRole="button" accessibilityLabel={`${open?'收起':'展开'}下级目标：${g.input.title}`} accessibilityState={{expanded:open}} aria-expanded={open} aria-controls={contentId} onPress={()=>setExpanded(old=>{const next=new Set(old);if(next.has(g.id))next.delete(g.id);else next.add(g.id);return next;})} style={{minHeight:44,paddingVertical:10}}><Text style={{fontWeight:'600',color:palettes.family.ink}}>{open?'▾ 收起':'▸ 展开'}下级目标 · {children.length}项</Text></Pressable>)}
   </View>
   {children.length>0&&<View nativeID={contentId} style={{gap:10,paddingLeft:10,...(!open?{display:'none' as const}:{})}}>{children.map(child=>branch(child,own))}</View>}
  </View>;
 };
 return <View style={{gap:14}}>
  <Text>各级目标都可选择；选下级会同时关联上级。</Text>
  <TextInput accessibilityLabel="搜索目标" value={search} onChangeText={setSearch} editable={!disabled} placeholder="搜索已保存的目标" style={{minHeight:46,padding:10,borderWidth:1,borderColor:neutral.border,borderRadius:10,backgroundColor:neutral.surface}}/>
  <View style={{gap:8,flexDirection:'row',alignItems:'center',flexWrap:'wrap'}}><Text style={{flex:1,minWidth:160}}>已选 {selected.length} 个 · 自动关联 {Array.from(relations.inherited.keys()).filter(id=>!relations.explicit.has(id)).length} 个</Text>{refreshControl}</View>
  {!term&&<View style={{flexDirection:'row',gap:12,flexWrap:'wrap'}}><UiButton title="展开全部目标" label="展开全部" press={()=>setExpanded(new Set(data.goals.map(g=>g.id)))}/><UiButton title="收起全部目标" label="收起全部" press={()=>setExpanded(new Set())}/></View>}
  {selected.filter(id=>!data.goals.some(g=>g.id===id)).map((id,i)=><View key={id} style={{gap:8}}><Text>原填写中的目标已不在当前目录；观察草稿仍保留。</Text><UiButton title={`解除未读到目标的关联 ${i+1}`} disabled={disabled} press={()=>toggle(id)}/></View>)}
  {tree.roots.map(row=>branch(row))}
  {!tree.roots.some(matches)&&<Text>没有匹配的目标；已有选择仍保留。</Text>}
  <UiCheck title="显示已收起目标" checked={showArchived} disabled={disabled} press={()=>setShowArchived(v=>!v)}/>
  <Text style={{fontSize:14,color:neutral.muted}}>与目标页使用同一份已保存目录，不沿用页面筛选。自动关联不需重复写表现，也不额外计次；取消最后一个下级后，上级自动关联会移除。直接勾选上级则会保留。</Text>
 </View>;
}

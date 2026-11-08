import {goalMethodDiagram,type DiagramNode} from './goal-method-diagram';
import {GoalMethodDiagramView} from './goal-method-diagram-view';
import {SectionTabs} from './section-tabs';
import React,{useState,useId} from 'react';
import {View,useWindowDimensions} from 'react-native';
import {Text,Pressable,TextInput} from './readability';
import {UiButton,UiCheck} from './ui-components';
import {Disclosure} from './disclosure';
import {DetailOverlay} from './overlay';
import {neutral,palettes} from './ui-palette';
import {goalMethodHierarchy,type GoalBranch} from './goal-method-hierarchy';
import {goalReferenceKnowledge,referencePathLabel} from './plan-related-knowledge';
import {goalAttention,methodAttention,type Attention} from './object-priority';
import {levelNames,type Snapshot,type Method} from './foundation';
import {localDate} from './draft';
import type {Schedule} from './prd-followups';
type Props={data:Snapshot;schedules?:Schedule[];reference?:boolean;openGoal?:(id:string)=>void;openMethod?:(id:string)=>void;editGoal?:(id:string)=>void;editMethod?:(id:string)=>void;openKnowledge?:(id:string)=>void;prioritize?:(kind:'goal'|'method',id:string)=>void};
export function GoalMethodMap({data,schedules=[],reference=false,openGoal,openMethod,editGoal,editMethod,openKnowledge,prioritize}:Props){
 const {width}=useWindowDimensions(),[orientation,setOrientation]=useState<'vertical'|'horizontal'>(()=>width<600?'vertical':'horizontal');
 const [view,setView]=useState<'diagram'|'list'>('diagram'),[selectedNode,setSelectedNode]=useState<DiagramNode|null>(null);
 const [showKnowledge,setShowKnowledge]=useState(false),[showInactive,setShowInactive]=useState(false),[search,setSearch]=useState(''),[knowledge,setKnowledge]=useState<Snapshot['knowledge'][number]|null>(null),prefix=useId(),today=localDate();
 const retained=new Set(data.goals.filter(g=>showInactive||!g.input.archived&&['active','maintaining'].includes(g.input.state??'active')).map(g=>g.id));
 for(const id of [...retained]){let p=data.goals.find(g=>g.id===id)?.input.parent_id;const seen=new Set([id]);while(p&&!seen.has(p)){seen.add(p);const g=data.goals.find(g=>g.id===p);if(!g)break;retained.add(p);p=g.input.parent_id;}}
 const goals=data.goals.filter(g=>retained.has(g.id)),methods=data.methods.filter(m=>showInactive||(m.input.state??'active')==='active'),tree=goalMethodHierarchy(goals,methods);
 const [expanded,setExpanded]=useState<Set<string>>(()=>new Set(data.goals.filter(g=>g.input.level==='long').map(g=>g.id)));
 const term=search.trim(),goalCache=new Map<string,Attention>(),methodCache=new Map<string,Attention>();
 const ownGoal=(id:string)=>{if(!goalCache.has(id))goalCache.set(id,goalAttention(data,data.goals.find(g=>g.id===id)!,today,schedules));return goalCache.get(id)!;};
 const ownMethod=(m:Method)=>{if(!methodCache.has(m.id))methodCache.set(m.id,methodAttention(data,m,today,schedules));return methodCache.get(m.id)!;};
 const rank=(r:GoalBranch):number=>Math.min(ownGoal(r.goal.id).rank,...r.children.map(rank),...r.methods.map(m=>ownMethod(m).rank));
 const sorted=(rows:GoalBranch[])=>[...rows].sort((a,b)=>rank(a)-rank(b)||a.goal.input.title.localeCompare(b.goal.input.title,'zh-CN')||a.goal.id.localeCompare(b.goal.id));
 const match=(r:GoalBranch):boolean=>r.goal.input.title.includes(term)||r.methods.some(m=>m.input.title.includes(term))||r.children.some(match);
 const diagram=goalMethodDiagram(goals,methods,expanded,new Map([...goals.map(g=>['goal:'+g.id,ownGoal(g.id).rank] as [string,number]),...methods.map(m=>['method:'+m.id,ownMethod(m).rank] as [string,number])]),term,orientation);
 const note=(a:Attention)=><Text style={{fontSize:14,color:neutral.muted}}>{a.reasons.join(' · ')}</Text>;
 const surface=(a:Attention,tone:'goal'|'method')=>({gap:8,padding:12,borderWidth:a.level==='high'?2:1,borderRadius:12,borderColor:a.level==='high'||a.level==='watch'?palettes[tone].ink:neutral.border,backgroundColor:a.level==='high'||a.level==='watch'?palettes[tone].soft:neutral.surface});
 const knowledgeRows=(rows:Snapshot['knowledge'],goalId?:string,methodId?:string)=><View style={{gap:8}}>{rows.map(k=><View key={k.input.id} testID={`map-knowledge-${goalId||methodId}-${k.input.id}`} style={{padding:10,gap:5,borderRadius:8,backgroundColor:palettes[k.input.kind].soft}}><Text style={{fontWeight:'600',color:palettes[k.input.kind].ink}}>{k.input.kind==='difficulty'?'困难':'经验'}：{k.details?.title||k.input.content}</Text>{goalId&&goalReferenceKnowledge(data,goalId).find(r=>r.knowledge.input.id===k.input.id)?.paths.map((p,i)=><Text key={i} style={{fontSize:14,color:neutral.muted}}>{referencePathLabel(data,p)}</Text>)}{methodId&&<Text style={{fontSize:14,color:neutral.muted}}>直接关联本方法</Text>}<UiButton title={`查看关系图知识：${k.details?.title||k.input.content} · ${goalId||methodId||'关系'}`} label="查看完整内容" tone={k.input.kind} variant="link" press={()=>setKnowledge(k)}/></View>)}</View>;
 const methodRow=(m:Method,goalId?:string)=>{const a=ownMethod(m),linked=data.knowledge.filter(k=>!k.deleted&&k.input.method_ids.includes(m.id)&&(k.input.kind==='difficulty'?['pending','following'].includes(k.details?.state||'pending'):(k.details?.state||'active')==='active'));return <View key={m.id} testID={`map-method-${goalId||'other'}-${m.id}`} style={surface(a,'method')}><Text style={{fontSize:17,fontWeight:a.level==='high'?'700':'600',color:palettes.method.ink}}>方法 · {m.input.title}</Text>{note(a)}{m.input.steps&&<Text numberOfLines={2}>{m.input.steps}</Text>}{m.input.goal_ids.length>1&&<Text style={{fontSize:14,color:neutral.muted}}>同时关联 {m.input.goal_ids.length} 个目标，各处是同一个方法。</Text>}<View style={{flexDirection:'row',gap:12,flexWrap:'wrap'}}>{openMethod&&<UiButton title={`查看方法：${m.input.title}`} label="查看方法" tone="method" variant="link" press={()=>openMethod(m.id)}/>} {!reference&&editMethod&&<UiButton title={`编辑方法：${m.input.title}`} label="编辑" tone="method" variant="link" press={()=>editMethod(m.id)}/>}</View>{!reference&&prioritize&&<UiButton title={`调整优先级：${m.input.title}`} label="调整优先级" tone="method" variant="link" press={()=>prioritize('method',m.id)}/>} {showKnowledge&&knowledgeRows(linked,undefined,m.id)}</View>};
 const branch=(r:GoalBranch,above=false):React.ReactNode=>{
  if(term&&!above&&!match(r))return null;
  const own=above||r.goal.input.title.includes(term),a=ownGoal(r.goal.id),children=sorted(r.children);
  const linked=[...r.methods].sort((x,y)=>ownMethod(x).rank-ownMethod(y).rank||x.input.title.localeCompare(y.input.title,'zh-CN'));
  const canExpand=children.length+linked.length>0,open=Boolean(term)||expanded.has(r.goal.id),contentId=prefix+'-'+r.goal.id;
  const refs=showKnowledge?goalReferenceKnowledge(data,r.goal.id):[];
  return <View key={r.goal.id} testID={`map-goal-${r.goal.id}`} style={{gap:10,paddingLeft:8,borderLeftWidth:2,borderColor:neutral.border}}>
   <View style={surface(a,'goal')}>
    <View style={{flexDirection:'row',alignItems:'flex-start',gap:8}}>
     <Text style={{flex:1,fontSize:a.level==='high'?20:18,fontWeight:a.level==='high'?'700':'600',color:palettes.goal.ink}}>{r.goal.input.title}</Text>
     <Text style={{fontSize:14,color:neutral.muted}}>{levelNames[r.goal.input.level]}</Text>
    </View>
    <Text style={{fontSize:14,color:neutral.muted}}>{a.reasons.join(' · ')}{rank(r)<a.rank?' · 下级有优先项目':''}</Text>
    {!reference&&prioritize&&<UiButton title={`调整优先级：${r.goal.input.title}`} label="调整优先级" tone="goal" variant="link" press={()=>prioritize('goal',r.goal.id)}/>}
    {showKnowledge&&refs.length>0&&knowledgeRows(refs.map(v=>v.knowledge),r.goal.id)}
    <View style={{flexDirection:'row',gap:10,alignItems:'center',flexWrap:'wrap'}}>
     {canExpand&&(term?<Text style={{fontSize:14}}>匹配路径已展开</Text>:<Pressable accessibilityRole="button" accessibilityLabel={`${open?'收起':'展开'}关系：${r.goal.input.title}`} accessibilityState={{expanded:open}} aria-expanded={open} aria-controls={contentId} onPress={()=>setExpanded(old=>{const next=new Set(old);next.has(r.goal.id)?next.delete(r.goal.id):next.add(r.goal.id);return next;})} style={{paddingVertical:10,minHeight:44}}><Text style={{fontWeight:'600',color:palettes.goal.ink}}>{open?'▾ 收起':'▸ 展开'} · {children.length}目标／{linked.length}方法</Text></Pressable>)}
     {openGoal&&<UiButton title={`查看目标：${r.goal.input.title}`} label="详情" tone="goal" variant="link" style={{minWidth:44,paddingHorizontal:4}} press={()=>openGoal(r.goal.id)}/>}
     {!reference&&editGoal&&<UiButton title={`编辑目标：${r.goal.input.title}`} label="编辑" tone="goal" variant="link" style={{minWidth:44,paddingHorizontal:4}} press={()=>editGoal(r.goal.id)}/>}
    </View>
   </View>
   {canExpand&&<View nativeID={contentId} style={{gap:10,paddingLeft:8,...(!open?{display:'none' as const}:{})}}>{children.map(c=>branch(c,own))}{linked.filter(m=>!term||own||m.input.title.includes(term)).map(m=>methodRow(m,r.goal.id))}</View>}
  </View>;
 };
 return <View testID={reference?'plan-goal-method-map':'goal-method-map'} style={{gap:12}}><SectionTabs label="关系展示方式" value={view} change={setView} items={[{value:'diagram',title:'节点关系图',tone:'goal'},{value:'list',title:'层级列表',tone:'goal'}]}/><Text>{reference?'点击节点查看做法和知识；这里只参考，不预选实际目标。':'点击节点查看详情、编辑或调整优先级。'}</Text><UiCheck title="显示关联经验和困难" checked={showKnowledge} tone="goal" press={()=>setShowKnowledge(v=>!v)}/>{showKnowledge&&view==='diagram'&&<Text style={{fontSize:14,color:neutral.muted}}>点选节点，在弹出窗口查看关联经验与困难。</Text>}{goals.length+methods.length>15&&<TextInput accessibilityLabel="搜索目标与方法关系" value={search} onChangeText={setSearch} placeholder="输入目标或方法名称" style={{padding:12,minHeight:46,borderWidth:1,borderColor:neutral.border,borderRadius:10}}/>}{view==='list'?<>{sorted(tree.roots).map(r=>branch(r))}{tree.unlinked.length>0&&<View style={{gap:10}}><Text style={{fontWeight:'600'}}>其他方法（不在当前目标范围内）</Text>{tree.unlinked.filter(m=>!term||m.input.title.includes(term)).map(m=>methodRow(m))}</View>}</>:<GoalMethodDiagramView model={diagram} expanded={expanded} attention={n=>n.kind==='goal'?ownGoal(n.id):ownMethod(n.method!)} select={setSelectedNode} toggle={id=>setExpanded(old=>{const next=new Set(old);next.has(id)?next.delete(id):next.add(id);return next;})}/>}<Disclosure title="图表选项">{view==='diagram'&&<SectionTabs label="关系图方向" value={orientation} change={setOrientation} items={[{value:'vertical',title:'纵向',tone:'goal'},{value:'horizontal',title:'横向',tone:'goal'}]}/>}<View style={{gap:10,flexDirection:'row',flexWrap:'wrap'}}><UiButton title="展开全部关系" label="展开全部" tone="goal" press={()=>setExpanded(new Set(goals.map(g=>g.id)))}/><UiButton title="收起全部关系" label="收起全部" tone="goal" press={()=>setExpanded(new Set())}/></View></Disclosure>{!goals.length&&!methods.length&&<Text>还没有目标与方法，可先从维护入口添加。</Text>}{term&&!tree.roots.some(match)&&!tree.unlinked.some(m=>m.input.title.includes(term))&&<Text>没有匹配的关系。</Text>}<Disclosure title="排序与强调说明"><Text>高关注更突出；同级结合重点困难、到期复盘和家庭关注间隔提示。下级或方法有高关注项目时，其所在分支也会提前。设置值不会自动改变，有困难时先检查支持条件，不表示要增加训练量。</Text></Disclosure><UiCheck title="显示暂停、已达成和收起项目" checked={showInactive} tone="goal" press={()=>setShowInactive(v=>!v)}/>{selectedNode&&<DetailOverlay title="关系节点" tone={selectedNode.kind} close={()=>setSelectedNode(null)}>
 <Text style={{fontSize:22,fontWeight:'700'}}>{selectedNode.title}</Text>
 {note(selectedNode.kind==='goal'?ownGoal(selectedNode.id):ownMethod(selectedNode.method!))}
 {selectedNode.kind==='method'&&<Text>{selectedNode.method?.input.steps}</Text>}
 <View style={{gap:8}}>{selectedNode.kind==='goal'?<>
 {openGoal&&<UiButton title={`查看目标：${selectedNode.title}`} label="查看目标详情" tone="goal" press={()=>{const id=selectedNode.id;setSelectedNode(null);openGoal(id);}}/>}
 {!reference&&editGoal&&<UiButton title={`编辑目标：${selectedNode.title}`} label="编辑目标" tone="goal" press={()=>{const id=selectedNode.id;setSelectedNode(null);editGoal(id);}}/>}
 </>:<>
 {openMethod&&<UiButton title={`查看方法：${selectedNode.title}`} label="查看方法详情" tone="method" press={()=>{const id=selectedNode.id;setSelectedNode(null);openMethod(id);}}/>}
 {!reference&&editMethod&&<UiButton title={`编辑方法：${selectedNode.title}`} label="编辑方法" tone="method" press={()=>{const id=selectedNode.id;setSelectedNode(null);editMethod(id);}}/>}
 </>}
 {!reference&&prioritize&&<UiButton title={`调整优先级：${selectedNode.title}`} tone={selectedNode.kind} press={()=>{const n=selectedNode;setSelectedNode(null);prioritize(n.kind,n.id);}}/>}
 <UiCheck title="显示关联经验和困难" checked={showKnowledge} tone="goal" press={()=>setShowKnowledge(v=>!v)}/>
 {showKnowledge&&(selectedNode.kind==='goal'?knowledgeRows(goalReferenceKnowledge(data,selectedNode.id).map(r=>r.knowledge),selectedNode.id):knowledgeRows(data.knowledge.filter(k=>!k.deleted&&k.input.method_ids.includes(selectedNode.id)&&(k.input.kind==='difficulty'?['pending','following'].includes(k.details?.state||'pending'):(k.details?.state||'active')==='active')),undefined,selectedNode.id))}
 </View></DetailOverlay>}{knowledge&&<DetailOverlay title="关系图关联知识" tone={knowledge.input.kind} close={()=>setKnowledge(null)}><Text style={{fontSize:20,fontWeight:'700'}}>{knowledge.details?.title||knowledge.input.content}</Text><Text selectable>{knowledge.input.content}</Text>{knowledge.details?.conditions&&<Text>条件：{knowledge.details.conditions}</Text>}{knowledge.details?.adult_response&&<Text>成人回应：{knowledge.details.adult_response}</Text>}{openKnowledge&&<UiButton title="查看这条知识的完整跟进" tone={knowledge.input.kind} press={()=>{const id=knowledge.input.id;setKnowledge(null);openKnowledge(id);}}/>}</DetailOverlay>}</View>;
}

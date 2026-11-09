import React,{useState} from 'react';
import {View} from 'react-native';
import {Text} from './readability';
import {UiButton,UiCheck} from './ui-components';
import {DetailOverlay} from './overlay';
import {GuidedField} from './guided-field';
import {SectionTabs} from './section-tabs';
import {goalReferenceKnowledge} from './plan-related-knowledge';
import {comparePriority,priorityLabel} from './object-priority';
import {localDate} from './draft';
import {palettes,neutral} from './ui-palette';
import type {Snapshot,Goal,Method} from './foundation';
type Entry={kind:'goal'|'method';id:string};
export function GoalMethodBrowser({data,reference,showKnowledge,openGoal,openMethod,editGoal,editMethod,prioritize,openKnowledge}:{data:Snapshot;reference:boolean;showKnowledge:boolean;openGoal?:(id:string)=>void;openMethod?:(id:string)=>void;editGoal?:(id:string)=>void;editMethod?:(id:string)=>void;prioritize?:(kind:'goal'|'method',id:string)=>void;openKnowledge?:(id:string)=>void}){
 const [entry,setEntry]=useState<Entry|null>(null),[history,setHistory]=useState<Entry[]>([]),[picking,setPicking]=useState(false),[kind,setKind]=useState<'goal'|'method'>('goal'),[search,setSearch]=useState(''),[inactive,setInactive]=useState(false),[knowledge,setKnowledge]=useState<Snapshot['knowledge'][number]|null>(null),today=localDate();
 const item=entry?(entry.kind==='goal'?data.goals:data.methods).find(v=>v.id===entry.id):undefined;
 const choose=(next:Entry,root=false)=>{if(root)setHistory([]);else if(entry)setHistory(v=>[...v,entry]);setEntry(next);setPicking(false);setSearch('');};
 const related:Entry[]=item&&entry?(entry.kind==='goal'?data.methods.filter(m=>m.input.goal_ids.includes(item.id)&&(inactive||(m.input.state??'active')==='active')).map(v=>({kind:'method' as const,id:v.id})):data.goals.filter(g=>(item as Method).input.goal_ids.includes(g.id)&&(inactive||!g.input.archived&&['active','maintaining'].includes(g.input.state??'active'))).map(v=>({kind:'goal' as const,id:v.id}))):[];
 const rows=(entries:Entry[])=>entries.sort((a,b)=>{const aa=(a.kind==='goal'?data.goals:data.methods).find(v=>v.id===a.id)!,bb=(b.kind==='goal'?data.goals:data.methods).find(v=>v.id===b.id)!;return comparePriority(data,aa,bb,a.kind,b.kind,today);}).map(e=>{const v=(e.kind==='goal'?data.goals:data.methods).find(v=>v.id===e.id)!;return <UiButton key={e.kind+e.id} title={`切换到${e.kind==='goal'?'目标':'方法'}：${v.input.title}`} label={`${e.kind==='goal'?'目标':'方法'} · ${v.input.title}  →`} tone={e.kind} press={()=>choose(e)}/>;});
 const refs=item&&entry?(entry.kind==='goal'?goalReferenceKnowledge(data,item.id).map(r=>r.knowledge):data.knowledge.filter(k=>!k.deleted&&k.input.method_ids.includes(item.id)&&(k.input.kind==='difficulty'?['pending','following'].includes(k.details?.state||'pending'):(k.details?.state||'active')==='active'))):[];
 return <View testID="relationship-browser" style={{gap:12}}>
 <View style={{gap:8,flexDirection:'row',flexWrap:'wrap'}}><UiButton title={entry?'重新选择入口':'选择目标或方法作为入口'} tone="goal" press={()=>setPicking(true)}/>{history.length>0&&<UiButton title="返回上一入口" press={()=>{setEntry(history[history.length-1]);setHistory(v=>v.slice(0,-1));}}/>}</View>
 {!entry&&<Text>从一个目标或方法出发，每次只看与它直接关联的项目。点击关联项目即可继续探索。</Text>}
 {entry&&!item&&<Text>这个入口已不在当前目录，请重新选择；原历史记录保持。</Text>}
 {entry&&item&&<><View testID="relationship-current" style={{padding:16,borderWidth:2,borderColor:palettes[entry.kind].ink,borderRadius:12,backgroundColor:palettes[entry.kind].soft,gap:10}}><Text style={{color:palettes[entry.kind].ink,fontWeight:'600'}}>当前{entry.kind==='goal'?'目标':'方法'} · 优先级 {priorityLabel(data,item,entry.kind,today)}</Text><Text accessibilityRole="header" style={{fontSize:22,fontWeight:'700'}}>{item.input.title}</Text><Text>{entry.kind==='goal'?(item as Goal).input.meaning||(item as Goal).input.criteria:(item as Method).input.steps}</Text>
 <View style={{flexDirection:'row',gap:10,flexWrap:'wrap'}}>{(entry.kind==='goal'?openGoal:openMethod)&&<UiButton title="查看入口详情" tone={entry.kind} variant="link" press={()=>entry.kind==='goal'?openGoal?.(entry.id):openMethod?.(entry.id)}/>}{!reference&&<><UiButton title="编辑入口" tone={entry.kind} variant="link" press={()=>entry.kind==='goal'?editGoal?.(entry.id):editMethod?.(entry.id)}/>{prioritize&&<UiButton title="调整入口优先级" tone={entry.kind} variant="link" press={()=>prioritize(entry.kind,entry.id)}/>}</>}</View></View>
 <Text accessibilityRole="header" style={{fontSize:18,fontWeight:'700'}}>关联{entry.kind==='goal'?'方法':'目标'} · {related.length} 项</Text>{rows(related)}{!related.length&&<Text>还没有直接关联的{entry.kind==='goal'?'方法':'目标'}。可以先查看详情，或在维护中补充关系。</Text>}
 {entry.kind==='goal'&&(()=>{const g=item as Goal,parents=data.goals.filter(v=>v.id===g.input.parent_id),children=data.goals.filter(v=>v.input.parent_id===g.id);return parents.length+children.length>0?<View style={{gap:8}}><Text style={{fontWeight:'600'}}>目标层级（不等于方法关系）</Text>{rows([...parents,...children].map(v=>({kind:'goal',id:v.id})))}</View>:null;})()}
 {showKnowledge&&<View style={{gap:8}}><Text style={{fontWeight:'600'}}>关联经验与困难 · {refs.length} 条</Text>{refs.map(k=><UiButton key={k.input.id} title={`浏览知识：${k.details?.title||k.input.content}`} label={`${k.input.kind==='difficulty'?'困难':'经验'} · ${k.details?.title||k.input.content}`} tone={k.input.kind} press={()=>setKnowledge(k)}/>)}{!refs.length&&<Text>尚无当前可参考的经验或困难。</Text>}</View>}</>}
 <UiCheck title="包括暂停和已收起的关联" checked={inactive} press={()=>setInactive(v=>!v)}/>
 {picking&&<DetailOverlay title="选择关系浏览入口" close={()=>setPicking(false)}><SectionTabs label="入口类型" value={kind} change={setKind} items={[{value:'goal',title:'从目标进入',tone:'goal'},{value:'method',title:'从方法进入',tone:'method'}]}/><GuidedField label="查找浏览入口" value={search} change={setSearch} multiline={false}/>{(kind==='goal'?data.goals:data.methods).filter(v=>v.input.title.includes(search)).sort((a,b)=>comparePriority(data,a,b,kind,kind,today)).map(v=><UiButton key={v.id} title={`选择入口：${v.input.title}`} tone={kind} press={()=>choose({kind,id:v.id},true)}/>)}{!(kind==='goal'?data.goals:data.methods).some(v=>v.input.title.includes(search))&&<Text>没有匹配项目。</Text>}</DetailOverlay>}
 {knowledge&&<DetailOverlay title="浏览关联知识" tone={knowledge.input.kind} close={()=>setKnowledge(null)}><Text selectable>{knowledge.input.content}</Text>{knowledge.details?.conditions&&<Text>条件：{knowledge.details.conditions}</Text>}{openKnowledge&&<UiButton title="查看知识完整跟进" press={()=>openKnowledge(knowledge.input.id)}/>}</DetailOverlay>}
 </View>;
}

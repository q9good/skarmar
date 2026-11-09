import React,{useState} from 'react';
import {View} from 'react-native';
import {Text} from './readability';
import {UiButton} from './ui-components';
import {DetailOverlay} from './overlay';
import {stateOf} from './prd-followups';
import {knowledgeTitle} from './summary-knowledge';
import type {Snapshot} from './foundation';
export function SummaryRelatedKnowledge({data,goals,methods}:{data:Snapshot;goals:string[];methods:string[]}){
 const [selected,setSelected]=useState<Snapshot['knowledge'][number]|null>(null);
 const items=data.knowledge.filter(k=>!k.deleted&&(k.input.kind==='difficulty'?['pending','following'].includes(stateOf(k)):stateOf(k)==='active')&&(k.input.goal_ids.some(id=>goals.includes(id))||k.input.method_ids.some(id=>methods.includes(id))));
 if(!items.length)return null;
 return <View testID="summary-associated-knowledge" style={{gap:8}}><Text style={{fontWeight:'600'}}>已关联目标／方法的经验与困难</Text><Text>这里只作参考；查看不会变成本次新增或反馈。</Text>{items.map(k=><View key={k.input.id} style={{gap:4}}><UiButton title={`查看关联${k.input.kind==='difficulty'?'困难':'经验'}：${knowledgeTitle(k)}`} label={`${k.input.kind==='difficulty'?'关联困难':'关联经验'} · ${knowledgeTitle(k)}`} tone={k.input.kind} variant="link" press={()=>setSelected(k)}/><Text style={{fontSize:14}}>关联：{[...k.input.goal_ids.filter(id=>goals.includes(id)).map(id=>'目标 · '+(data.goals.find(g=>g.id===id)?.input.title??'原目标')),...k.input.method_ids.filter(id=>methods.includes(id)).map(id=>'方法 · '+(data.methods.find(m=>m.id===id)?.input.title??'原方法'))].join('；')}</Text></View>)}
 {selected&&<DetailOverlay title="关联知识参考" tone={selected.input.kind} close={()=>setSelected(null)}><Text style={{fontWeight:'700',fontSize:20}}>{knowledgeTitle(selected)}</Text><Text selectable>{selected.input.content}</Text>{selected.details?.conditions&&<Text>条件：{selected.details.conditions}</Text>}<Text>只读查看，关闭后回到原总结填写。</Text></DetailOverlay>}</View>;
}

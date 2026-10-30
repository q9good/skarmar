import React,{useState} from 'react';
import {ContentGroup} from './section-tabs';
import {View} from 'react-native';
import {Pressable,Text} from './readability';
import {localDate} from './draft';
import type {Snapshot} from './foundation';
import {objectActions,objectEvidence,type ObjectKey} from './object-context';
const stateNames:Record<string,string>={pending:'待做',in_plan:'已纳入计划',completed:'已完成',cancelled:'已取消'};
function Button({label,press}:{label:string;press:()=>void}){return <Pressable accessibilityRole="button" accessibilityLabel={label} onPress={press} style={{padding:12,minHeight:46,backgroundColor:'#E9F2E9',borderRadius:8}}><Text>{label}</Text></Pressable>;}
export function ObjectContext({data,object,openAction,openSession,openLesson}:{data:Snapshot;object:ObjectKey;openAction:(id:string)=>void;openSession:(id:string)=>void;openLesson:(id:string)=>void}){
 const [closed,setClosed]=useState(false),[limit,setLimit]=useState(3),actions=objectActions(data,object),visible=actions.filter(a=>closed||['pending','in_plan'].includes(a.status)),rows=objectEvidence(data,object,localDate());
 return <View style={{gap:14}}><ContentGroup title="关联的下一步" tone="tasks">
  {visible.map(a=><View key={a.input.id} style={{gap:6,padding:10,backgroundColor:'#FFFFFF'}}><Text>{stateNames[a.status]??a.status}：{a.input.content}</Text><Text>负责人：{a.assignee_name??'家人待认领'}{a.input.suggested_date?` · 建议${a.input.suggested_date}`:''}</Text>{object.kind==='goal'&&!a.input.goal_ids.includes(object.id)&&<Text>经关联方法找到，请核对是否适用于这个目标。</Text>}{a.source_deleted&&<Text>来源已删除，保留已保存的行动。</Text>}<Button label={`维护关联下一步：${a.input.content}`} press={()=>openAction(a.input.id)}/>{a.status==='in_plan'&&a.plan_id&&<Button label={`查看行动所在计划：${a.input.content}`} press={()=>openSession(a.plan_id!)}/>}</View>)}
  {!visible.length&&<Text>当前没有关联的待做行动。</Text>}{actions.some(a=>!['pending','in_plan'].includes(a.status))&&<Button label={closed?'收起已处理行动':'查看已完成或取消的关联行动'} press={()=>setClosed(v=>!v)}/>}
  </ContentGroup><ContentGroup title="相关实际记录" tone="family">
  {rows.slice(0,limit).map(e=><View key={e.kind+e.id} style={{gap:6,padding:10,backgroundColor:'#FFFFFF'}}><Text>{e.kind==='family'?'家庭观察':'课堂观察'}：{e.date}</Text>{e.contextOnly&&<Text>这是使用过该方法的整次记录背景，不将变化归因于某个方法。</Text>}<Text>{e.note}</Text><Button label={`查看关联${e.kind==='family'?'干预':'课记'}：${e.date} · ${e.kind==='family'?(e.record as import('./prd-sessions').Session).input.time_slot:(e.record as import('./prd-lessons').Lesson).course.name}`} press={()=>e.kind==='family'?openSession(e.id):openLesson(e.id)}/></View>)}
  {!rows.length&&<Text>尚无明确关联的实际记录。</Text>}{rows.length>limit&&<Button label="更多相关实际记录" press={()=>setLimit(n=>n+30)}/>}
 </ContentGroup></View>;
}

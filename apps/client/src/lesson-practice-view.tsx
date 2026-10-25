import {GuidedField} from './guided-field';
import React,{useEffect,useRef,useState} from 'react';
import {View} from 'react-native';
import {Pressable,Text,TextInput} from './readability';
import {Select} from './controls';
import {request} from './api';
import {uuid} from './draft';
import type {Snapshot} from './foundation';
import type {Lesson,OrganizationInput} from './prd-lessons';
import type {Feedback,PracticeFeedback} from './prd-followups';
import {labels} from './prd-followups';
import {ExampleHint} from './input-guidance';
const Button=({title,press,disabled=false}:{title:string;press:()=>void;disabled?:boolean})=><Pressable accessibilityRole="button" disabled={disabled} onPress={press} style={{padding:12,minHeight:48,borderRadius:10,backgroundColor:'#E9F2E9',opacity:disabled?.5:1}}><Text>{title}</Text></Pressable>;
export function LessonPractice({lesson,data,value,change,disabled=false,openKnowledge}:{lesson:Lesson;data:Snapshot;value?:OrganizationInput;change?:(v:Partial<OrganizationInput>)=>void;disabled?:boolean;openKnowledge?:(id:string)=>void}){
 const [eventPage,setEventPage]=useState<{id:string;items:Feedback[]}>({id:lesson.id,items:[]}),[error,setError]=useState(''),[loading,setLoading]=useState(true),[open,setOpen]=useState(Boolean(value?.feedback?.length)),seq=useRef(0);
 const load=async()=>{const s=++seq.current;setLoading(true);setError('');try{const v=await request<Feedback[]>(`/prd/process?view=lesson_feedback&id=${lesson.id}`);if(s===seq.current)setEventPage({id:lesson.id,items:v});}catch{if(s===seq.current)setError('本课已有反馈暂未读到，请刷新后再新增；原填写仍保留。');}finally{if(s===seq.current)setLoading(false);}};
 useEffect(()=>{void load();return()=>{++seq.current;};},[lesson.id,lesson.version]);
 const events=eventPage.id===lesson.id?eventPage.items:[];
 const feedback=value?.feedback??[],goals=[...(value?.targets.map(t=>t.goal_id)??[]),...(value?.focus_goal_ids??[])],methods=value?.method_ids??[];
 const candidates=data.knowledge.filter(k=>!k.deleted&&k.input.kind==='experience'&&(k.input.goal_ids.some(id=>goals.includes(id))||k.input.method_ids.some(id=>methods.includes(id))));
 const patch=(f:PracticeFeedback,v:Partial<PracticeFeedback>)=>change?.({feedback:feedback.map(p=>p.id===f.id?{...p,...v}:p)});
 const update=(k:typeof data.knowledge[number],result:string)=>{
  const existing=feedback.find(f=>f.knowledge_id===k.input.id);
  change?.({feedback:[...feedback.filter(f=>f.knowledge_id!==k.input.id),{...(existing??{id:uuid(),knowledge_id:k.input.id,observation:'',attempt:'',adult_response:'',conditions:k.details?.conditions??'',reason:'',comparable:false,basis:'',support:'',expected_version:null}),result,reason:result==='unhelpful'?existing?.reason??'':''}],feedback_knowledge_versions:{...value?.feedback_knowledge_versions,[k.input.id]:value?.feedback_knowledge_versions?.[k.input.id]??k.version}});
 };
 return <View style={{gap:9,paddingVertical:10}}>
  <Text style={{fontSize:18,fontWeight:'600'}}>本课已有经验反馈</Text>
  {loading&&<Text>正在读取本课反馈…</Text>}{error&&<Text accessibilityRole="alert">{error}</Text>}
  {events.filter(e=>!e.deleted).map(e=><View key={e.id} style={{padding:10,gap:5,backgroundColor:'#FFFFFF'}}><Text>{data.knowledge.find(k=>k.input.id===e.input.knowledge_id)?.input.content??'历史经验'} · {labels[e.input.result]}</Text><Text>{e.input.observation} · {e.author_name}</Text>{e.input.observation_version&&<Text>依据课堂观察第{e.input.observation_version}版，反馈第{e.version}版。</Text>}{openKnowledge&&<Button title={`查看或修订本课反馈：${data.knowledge.find(k=>k.input.id===e.input.knowledge_id)?.details.title??e.input.knowledge_id.slice(0,8)}`} press={()=>openKnowledge(e.input.knowledge_id)}/>}</View>)}
  {!loading&&!events.some(e=>!e.deleted)&&<Text>尚未保存本课对已有经验的反馈；新建经验与反馈已有经验分别记录。</Text>}
  <Button title="刷新本课已有反馈" disabled={disabled||loading} press={()=>void load()}/>
  {value&&change&&<><Button title={open?'收起已有经验反馈填写':'补充已有经验反馈'} disabled={disabled} press={()=>setOpen(v=>!v)}/>{open&&<>
   <Text>只列本课明确目标、方向或实际方法所关联的经验。选择结果后写一句实际观察；和本次整理一起提交。若更换观察版本，请核对已填反馈。</Text><Text>本次依据课堂观察第{value.seen_observation_version}版；“没帮上”不会降低经验阶段。</Text>
   {!candidates.length&&<Text>暂无可反馈的关联经验，可先明确本课目标／方法，或新建一条经验。</Text>}
   {candidates.map(k=>{const f=feedback.find(f=>f.knowledge_id===k.input.id),saved=events.find(e=>e.input.knowledge_id===k.input.id),changed=f&&value.feedback_knowledge_versions?.[k.input.id]!==k.version;return <View key={k.input.id} style={{gap:8,padding:12,borderWidth:1,borderColor:'#A6B5A8',borderRadius:10}}>
    <Text style={{fontWeight:'600'}}>{k.input.content}{k.details?.state==='retired'?'（已停用，回记原实践）':''}</Text>
    {saved?<Text>{saved.deleted?'本课误建反馈已由管理员删除，不能随整理自动恢复；请到跟进中核对。':`本课已记录${labels[saved.input.result]}，重复保存整理不会再计一次；请从上方查看后明确修订。`}</Text>:<>{['helpful','uncertain','unhelpful'].map(result=><Button key={result} title={`本课${labels[result]}：${k.input.content}`} disabled={disabled||loading||!!error} press={()=>update(k,result)}/>)}</>}
    {f&&<>{changed&&<><Text>这条经验已有第{k.version}版，你的反馈仍保留。核对当前经验后再明确采用。</Text><Button title={`已核对经验新版本：${k.input.content}`} disabled={disabled} press={()=>change({feedback_knowledge_versions:{...value.feedback_knowledge_versions,[k.input.id]:k.version}})}/></>}
     <Text>本次选择：{labels[f.result]}{saved?(saved.id===f.id?'；已找到本次提交，重试同一次整理即可取回回执':'；与已有反馈冲突，请先取消此处新增'):''}</Text><GuidedField label={`本课反馈依据：${k.input.content}`} value={f.observation} change={observation=>patch(f,{observation})} disabled={disabled} placeholder='例如：老师用这个做法后，他愿意再尝试一轮'/>
     {f.result==='unhelpful'&&<Select label={`本课没帮上的可能原因：${k.input.content}`} value={f.reason} disabled={disabled} options={[{value:'',label:'暂不判断'},{value:'implementation_uncertain',label:'可能还没做到位'},{value:'no_change_observed',label:'做到了，未观察到变化'}]} onChange={reason=>patch(f,{reason})}/>}
     <Button title={`取消本课新增反馈：${k.input.content}`} disabled={disabled} press={()=>change({feedback:feedback.filter(p=>p.id!==f.id)})}/>
    </>}
   </View>;})}
   {feedback.filter(f=>!candidates.some(k=>k.input.id===f.knowledge_id)).map(f=><View key={f.id} style={{gap:6}}><Text>目标／方法关联已改变，此处原反馈仍保留。请恢复关联或明确取消本次新增。</Text><Text>{f.observation}</Text><Button title={`取消未关联的本课反馈：${f.id.slice(0,8)}`} disabled={disabled} press={()=>change({feedback:feedback.filter(p=>p.id!==f.id)})}/></View>)}
  </>}</>}
 </View>;
}

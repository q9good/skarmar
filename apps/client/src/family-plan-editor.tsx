import React,{useState} from 'react';
import {View} from 'react-native';
import {Text,TextInput} from './readability';
import {UiButton,UiSection,UiCheck} from './ui-components';
import {UiTheme} from './ui-theme';
import {neutral,palettes,overlayColors} from './ui-palette';
import {DetailOverlay} from './overlay';
import {Disclosure} from './disclosure';
import {Select} from './controls';
import {GuidedField} from './guided-field';
import {PlanCoveragePreview} from './plan-coverage-preview';
import {planRelatedKnowledge} from './plan-related-knowledge';
import {chooseMain,type SessionInput,type Coverage} from './prd-sessions';
import type {Snapshot} from './foundation';

type Props={input:SessionInput;data:Snapshot;report:Coverage|null;disabled:boolean;dateControls:React.ReactNode;patch:(v:Partial<SessionInput>)=>void;changeTargets:(ids:string[])=>void;openGoals:()=>void;newMethod:()=>void;openGoal:(id:string)=>void;openCoverage:()=>void;references:React.ReactNode};
export function FamilyPlanEditor({input,data,report,disabled,dateControls,patch,changeTargets,openGoals,newMethod,openGoal,openCoverage,references}:Props){
 const [methodPicker,setMethodPicker]=useState(false),[search,setSearch]=useState(''),[knowledgeDetail,setKnowledgeDetail]=useState<Snapshot['knowledge'][number]|null>(null);
 const related=planRelatedKnowledge(data,input),selected=input.methods.map(r=>r.id),c=palettes.family;
 const methodOptions=[...data.methods].filter(m=>m.input.state==='active'||selected.includes(m.id)).sort((a,b)=>Number(b.input.goal_ids.some(id=>input.targets.some(t=>t.goal_id===id)))-Number(a.input.goal_ids.some(id=>input.targets.some(t=>t.goal_id===id)))||a.input.title.localeCompare(b.input.title,'zh-CN'));
 const toggleMethod=(id:string)=>{const method=data.methods.find(m=>m.id===id);if(method)patch({methods:selected.includes(id)?input.methods.filter(r=>r.id!==id):[...input.methods,{id,version:method.version}]});};
 return <UiTheme tone="family"><View testID="family-plan-editor" style={{gap:18}}>
  <UiSection title="准备做什么" required tone="family" testID="plan-activity-section">
   <TextInput accessibilityLabel="准备做什么" aria-required value={input.plan} onChangeText={plan=>patch({plan})} editable={!disabled} multiline placeholder="例如：一起玩轮流滚球，试着等待和主动求助" accessibilityHint="必填，用一两句话写下这次准备做的事" style={{padding:14,minHeight:104,borderWidth:1,borderColor:c.border,borderRadius:10,backgroundColor:c.page,textAlignVertical:'top'}}/>
   <Text style={{fontSize:14,color:neutral.muted}}>用一两句话写清要做的事。之后再选目标和方法。</Text>
   {dateControls}
  </UiSection>
  <UiSection title="关键目标" tone="goal" description="把最关注的本领放在前面，其他目标可以顺便练或复习。" testID="plan-goals-section">
   {input.targets.map(t=>{const goal=data.goals.find(g=>g.id===t.goal_id),row=report?.goals.find(g=>g.id===t.goal_id);return <View key={t.goal_id} style={{gap:9,paddingBottom:12,borderBottomWidth:1,borderColor:neutral.border}}>
    <View style={{flexDirection:'row',gap:10,alignItems:'center'}}><Text style={{flex:1,fontWeight:'700',fontSize:18}}>{goal?.input.title??'请核对这个目标'}</Text><UiButton title={`移除计划目标：${goal?.input.title??t.goal_id}`} label="移除" variant="link" tone="goal" disabled={disabled} press={()=>changeTargets(input.targets.filter(v=>v.goal_id!==t.goal_id).map(v=>v.goal_id))}/></View>
    <Select compact label={`目标角色：${goal?.input.title}`} value={t.role} disabled={disabled} options={[{value:'main',label:'主要练'},{value:'secondary',label:'顺便练'},...(goal?.input.state==='achieved'||t.role==='review'?[{value:'review',label:'复习'}]:[])]} onChange={role=>patch({targets:role==='main'?chooseMain(input.targets,t.goal_id):input.targets.map(v=>v.goal_id===t.goal_id?{...v,role:role as 'secondary'|'review'}:v)})}/>
    {row&&<Text style={{fontSize:14,color:neutral.muted}}>近30天 · 家庭 {row.counts.family} 次／机构 {row.counts.lesson} 节</Text>}
   </View>;})}
   <UiButton title="选择本次目标" label={input.targets.length?'调整本次目标':'选择本次目标'} tone="goal" disabled={disabled} press={openGoals}/>
  </UiSection>
  <UiSection title="准备使用的方法" tone="method" description="可选。这里记录准备采用的做法，实际用了哪些方法在总结时确认。" testID="plan-methods-section">
   {input.methods.map(ref=>{const method=data.methods.find(m=>m.id===ref.id);return <View key={ref.id} style={{gap:6,paddingBottom:10,borderBottomWidth:1,borderColor:neutral.border}}><View style={{flexDirection:'row',alignItems:'center',gap:10}}><Text style={{flex:1,fontSize:18,fontWeight:'600'}}>{method?.input.title??'请核对这个方法'}</Text><UiButton title={`移除计划方法：${method?.input.title??ref.id}`} label="移除" variant="link" tone="method" disabled={disabled} press={()=>toggleMethod(ref.id)}/></View>{method&&<Disclosure title={`具体做法：${method.input.title}`}><Text>{method.input.steps||method.input.description||'具体做法待补充'}</Text></Disclosure>}</View>;})}
   <UiButton title="选择计划方法" label={input.methods.length?'调整计划方法':'选择计划方法'} tone="method" disabled={disabled} press={()=>{setSearch('');setMethodPicker(true);}}/>
   <UiButton title="快捷新增计划方法" label="没有合适的方法？新建一个" variant="link" tone="method" disabled={disabled||!input.targets.length} press={newMethod}/>
  </UiSection>
  {related.length>0&&<View testID="plan-related-knowledge" style={{gap:12}}>
   <Text accessibilityRole="header" style={{fontSize:20,fontWeight:'700'}}>本次可参考的经验与困难</Text>
   {(['experience','difficulty'] as const).map(kind=>{const rows=related.filter(r=>r.knowledge.input.kind===kind),tone=palettes[kind];return rows.length?<View key={kind} testID={`plan-related-${kind}`} style={{gap:10,padding:16,borderRadius:16,borderWidth:1,borderColor:tone.border,backgroundColor:tone.soft}}>
    <Text accessibilityRole="header" style={{fontSize:18,fontWeight:'700',color:tone.ink}}>{kind==='experience'?'可参考的经验':'需要留意的困难'} · {rows.length}</Text>
    {rows.map(({knowledge:k,goals,methods})=><View key={k.input.id} style={{gap:6,paddingTop:8,borderTopWidth:1,borderColor:tone.border}}><Text style={{fontWeight:'600'}}>{k.details?.title||k.input.content}</Text>{k.details?.title&&k.details.title.trim()!==k.input.content.trim()&&<Text numberOfLines={3}>{k.input.content}</Text>}<Text style={{fontSize:14,color:neutral.muted}}>关联：{[...goals.map(g=>g.input.title),...methods.map(m=>m.input.title)].join('、')}</Text><UiButton title={`查看计划${kind==='experience'?'经验':'困难'}：${k.details?.title||k.input.content}`} label="查看完整说明" tone={kind} variant="link" press={()=>setKnowledgeDetail(k)}/></View>)}
   </View>:null;})}
  </View>}
  <Disclosure title="平衡提醒与其他参考" summary="覆盖频次、历史依据和目标笔记">
   <PlanCoveragePreview data={data} report={report}/><UiButton title="查看全部方向的近况" press={openCoverage}/>
   {input.targets.map(t=><Disclosure key={t.goal_id} title={`目标笔记与参考：${data.goals.find(g=>g.id===t.goal_id)?.input.title}`} summary={t.note?'有笔记':undefined}><GuidedField label={`计划笔记：${data.goals.find(g=>g.id===t.goal_id)?.input.title}`} value={t.note} disabled={disabled} change={note=>patch({targets:input.targets.map(v=>v.goal_id===t.goal_id?{...v,note}:v)})} placeholder="例如：先从熟悉的材料开始"/><UiButton title={`查看目标与标准：${data.goals.find(g=>g.id===t.goal_id)?.input.title}`} tone="goal" press={()=>openGoal(t.goal_id)}/></Disclosure>)}
   {references}
  </Disclosure>
  {methodPicker&&<DetailOverlay title="选择计划方法" tone="method" close={()=>setMethodPicker(false)} footer={<UiButton title="完成方法选择" variant="primary" tone="method" press={()=>setMethodPicker(false)}/>}>
   <Text>与所选目标有关的方法排在前面。选择只代表计划。</Text><GuidedField label="搜索计划方法" value={search} change={setSearch} multiline={false}/>
   {methodOptions.filter(m=>m.input.title.includes(search)).map(m=><View key={m.id} style={{gap:4}}><UiCheck title={`计划方法：${m.input.title}`} checked={selected.includes(m.id)} tone="method" disabled={disabled} press={()=>toggleMethod(m.id)}/>{m.input.goal_ids.some(id=>input.targets.some(t=>t.goal_id===id))&&<Text style={{fontSize:14,color:neutral.muted}}>与本次目标相关</Text>}</View>)}
   {!methodOptions.length&&<Text>还没有可选择的方法，可以返回后新建。</Text>}
   {methodOptions.length>0&&!methodOptions.some(m=>m.input.title.includes(search))&&<Text>没有匹配项，已有选择仍保留。</Text>}
  </DetailOverlay>}
  {knowledgeDetail&&<DetailOverlay title={knowledgeDetail.input.kind==='experience'?'本次相关经验':'本次相关困难'} colors={overlayColors(knowledgeDetail.input.kind)} close={()=>setKnowledgeDetail(null)}><Text style={{fontSize:20,fontWeight:'700'}}>{knowledgeDetail.details?.title||knowledgeDetail.input.content}</Text>{knowledgeDetail.details?.title&&knowledgeDetail.details.title.trim()!==knowledgeDetail.input.content.trim()&&<Text selectable>{knowledgeDetail.input.content}</Text>}{knowledgeDetail.details?.conditions&&<Text>适用条件：{knowledgeDetail.details.conditions}</Text>}{knowledgeDetail.details?.expected&&<Text>希望发生：{knowledgeDetail.details.expected}</Text>}</DetailOverlay>}
 </View></UiTheme>;
}

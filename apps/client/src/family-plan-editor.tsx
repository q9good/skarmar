import {GoalMethodMap} from './goal-method-map';
import type {Schedule} from './prd-followups';
import React,{useState} from 'react';
import {View} from 'react-native';
import {Text,TextInput} from './readability';
import {UiButton,UiSection} from './ui-components';
import {UiTheme} from './ui-theme';
import {neutral,palettes} from './ui-palette';
import {DetailOverlay} from './overlay';
import {Disclosure} from './disclosure';
import {PlanCoveragePreview} from './plan-coverage-preview';
import {planContext,type PlanEvidence} from './plan-context';
import {localDate} from './draft';
import {goalReferenceKnowledge,referencePathLabel,type KnowledgeReference} from './plan-related-knowledge';
import type {SessionInput,Coverage} from './prd-sessions';
import {levelNames,goalStateName,type Snapshot,type Goal} from './foundation';

type Props={schedules?:Schedule[];openGoal?:(id:string)=>void;openMethod?:(id:string)=>void;input:SessionInput;data:Snapshot;report:Coverage|null;disabled:boolean;dateControls:React.ReactNode;patch:(v:Partial<SessionInput>)=>void;openCoverage:()=>void};
export function FamilyPlanEditor({input,data,report,disabled,dateControls,patch,openCoverage,schedules,openGoal,openMethod}:Props){
 const [goal,setGoal]=useState<Goal|null>(null),[detail,setDetail]=useState<KnowledgeReference|null>(null),[evidence,setEvidence]=useState<PlanEvidence|null>(null);
 const c=palettes.family;
 const knowledgeCard=(row:KnowledgeReference)=>{
  const k=row.knowledge,tone=k.input.kind,title=k.details?.title||k.input.content;
  return <View key={k.input.id} testID={'reference-knowledge-'+k.input.id} style={{gap:7,padding:12,borderWidth:1,borderColor:palettes[tone].border,borderRadius:12,backgroundColor:palettes[tone].soft}}>
   <Text style={{fontWeight:'700',color:palettes[tone].ink}}>{title}</Text>
   {k.details?.title&&k.details.title.trim()!==k.input.content.trim()&&<Text numberOfLines={2}>{k.input.content}</Text>}
   {[...new Set(row.paths.map(p=>referencePathLabel(data,p)))].map(label=><Text key={label} style={{fontSize:14,color:neutral.muted}}>{label}</Text>)}
   <UiButton title={'查看参考知识：'+title} label="查看完整说明" variant="link" tone={tone} press={()=>setDetail(row)}/>
  </View>;
 };
 return <UiTheme tone="family"><View testID="family-plan-editor" style={{gap:18}}>
  <UiSection title="准备做什么" required tone="family" testID="plan-activity-section">
   <TextInput accessibilityLabel="准备做什么" aria-required value={input.plan} onChangeText={plan=>patch({plan})} editable={!disabled} multiline placeholder="例如：和孩子玩滚球，看他愿意怎么玩，随时换活动或休息" accessibilityHint="必填，用一两句话写下活动计划；实际目标和方法在总结时选择" style={{padding:14,minHeight:104,borderWidth:1,borderColor:c.border,borderRadius:10,backgroundColor:c.page,textAlignVertical:'top'}}/>
   <Text style={{fontSize:14,color:neutral.muted}}>可以按孩子的状态调整活动，实际目标和方法在结束后确认。</Text>
   {dateControls}
  </UiSection>
  <UiSection title="目标与方法参考" tone="goal" testID="plan-reference-goals" description="只作参考，不要求本次完成，不自动计覆盖。">
   <GoalMethodMap data={data} schedules={schedules} reference openGoal={openGoal??(id=>{const g=data.goals.find(g=>g.id===id);if(g)setGoal(g);})} openMethod={openMethod}/>
  </UiSection>
  <Disclosure title="参考排序与均衡情况" summary="只是提醒，不要求按建议完成">
   <Text>关系图按家人关注优先级、重点困难、到期复盘与家庭关注间隔强调；保留真实层级和多对多方法关系。低关注仍可展开，暂停或已收起内容可按需显示。</Text>
   <Text>知识包括直接关联、经使用中的方法关联，以及上级方向的直接原则。同一条知识合并展示；已解决、暂停、停用和删除条目不自动推荐。间接关联仍需判断是否适合当下。</Text>
   <PlanCoveragePreview data={data} report={report}/><UiButton title="查看全部方向的近况" press={openCoverage}/>
  </Disclosure>
  {(input.targets.length>0||input.methods.length>0||input.action_ids.length>0||input.lesson_sources.length>0||input.focus_goal_ids.length>0||input.tags.length>0||input.context)&&<Disclosure title="已有填写与来源" summary="保留原信息，实际情况在总结确认">
   {input.targets.map(t=><Text key={t.goal_id}>{data.goals.find(g=>g.id===t.goal_id)?.input.title??'原目标'}{t.note?' · '+t.note:''}</Text>)}
   {input.methods.map(m=><Text key={m.id}>原参考方法：{data.methods.find(v=>v.id===m.id)?.input.title??'原方法'} · 第{m.version}版</Text>)}
   {input.action_ids.map(id=><Text key={id}>下一步来源：{data.actions.find(a=>a.input.id===id)?.input.content??'原行动'}</Text>)}
   {input.lesson_sources.map(l=><Text key={l.id}>课堂来源：{data.lessons.find(v=>v.id===l.id)?.course.name??'原课记'} · 第{l.version}版</Text>)}
   {input.focus_goal_ids.map(id=><Text key={id}>原关注点：{data.goals.find(g=>g.id===id)?.input.title??'原方向'}</Text>)}
   {!!input.tags.length&&<Text>原活动标签：{input.tags.join('、')}</Text>}{!!input.context&&<Text>原补充：{input.context}</Text>}
  </Disclosure>}
  {goal&&<DetailOverlay title="目标参考" tone="goal" close={()=>setGoal(null)}><Text style={{fontSize:20,fontWeight:'700'}}>{goal.input.title}</Text><Text>{levelNames[goal.input.level]} · {goalStateName(goal.input)}</Text>{!!goal.input.meaning&&<Text>{goal.input.meaning}</Text>}{!!goal.input.criteria&&<Text>观察标准：{goal.input.criteria}</Text>}<Text>关联方法仅供参考，实际使用在总结时确认。</Text>{(()=>{const context=planContext(data,goal.id,localDate(),'');return [context.family,context.lesson].filter((v):v is PlanEvidence=>Boolean(v)).map(e=><View key={e.kind} style={{gap:5}}><Text>上次{e.kind==='family'?'家庭':'课堂'}观察：{e.date} · {e.note}</Text><UiButton title={`查看上次${e.kind==='family'?'家庭':'课堂'}依据：${goal.input.title}`} label="查看完整依据（只读）" variant="link" tone="goal" press={()=>setEvidence(e)}/></View>);})()}{data.methods.filter(m=>(m.input.state??'active')==='active'&&m.input.goal_ids.includes(goal.id)).map(m=><View key={m.id} style={{gap:5}}><Text style={{fontWeight:'600',color:palettes.method.ink}}>{m.input.title}</Text><Text>{m.input.steps||m.input.description||'具体做法待补充'}</Text></View>)}{goalReferenceKnowledge(data,goal.id).map(knowledgeCard)}</DetailOverlay>}
  {evidence&&<DetailOverlay title="计划参考依据" tone={evidence.kind==='family'?'family':'classroom'} close={()=>setEvidence(null)}><Text>只读参考，关闭后回到目标参考；不会改写本次活动或选择实际目标。</Text><Text>{evidence.date} · {evidence.kind==='family'?'家庭干预':'机构课程'}第{evidence.record.version}版</Text><Text selectable>{evidence.note}</Text></DetailOverlay>}
  {detail&&<DetailOverlay title="参考知识" tone={detail.knowledge.input.kind} close={()=>setDetail(null)}><Text style={{fontSize:20,fontWeight:'700'}}>{detail.knowledge.details?.title||detail.knowledge.input.content}</Text><Text selectable>{detail.knowledge.input.content}</Text>{detail.paths.map((p,i)=><Text key={i}>{referencePathLabel(data,p)}</Text>)}{!!detail.knowledge.details?.conditions&&<Text>适用条件：{detail.knowledge.details.conditions}</Text>}{!!detail.knowledge.details?.adult_response&&<Text>成人回应：{detail.knowledge.details.adult_response}</Text>}{!!detail.knowledge.details?.expected&&<Text>希望发生：{detail.knowledge.details.expected}</Text>}</DetailOverlay>}
 </View></UiTheme>;
}

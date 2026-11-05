import React,{useState} from 'react';
import {View} from 'react-native';
import Svg,{Circle} from 'react-native-svg';
import {Text,Pressable} from './readability';
import {UiButton} from './ui-components';
import {SectionTabs} from './section-tabs';
import {DetailOverlay} from './overlay';
import {neutral,palettes,chartColors} from './ui-palette';
import {goalStatusGroups,goalOverviewRows,type OverviewState} from './goal-overview';
import {goalStateName,levelNames,type Goal} from './foundation';
import type {Coverage} from './prd-sessions';

type Slice={id:string;value:number;color:string};
function OverviewRing({slices,total,label,centre,unit}:{slices:Slice[];total:number;label:string;centre:string;unit:string}){
 const circumference=2*Math.PI*63;let offset=0;
 return <View accessible accessibilityRole="image" accessibilityLabel={label} style={{width:172,height:172,alignSelf:'center'}}>
  <Svg width={172} height={172} viewBox="0 0 172 172" aria-hidden><Circle cx={86} cy={86} r={63} stroke={neutral.border} strokeWidth={18} fill="none"/>{total>0&&slices.filter(s=>s.value>0).map(s=>{const start=offset,length=s.value/total*circumference;offset+=length;return <Circle key={s.id} cx={86} cy={86} r={63} stroke={s.color} strokeWidth={18} strokeDasharray={`${length} ${Math.max(0,circumference-length)}`} strokeDashoffset={-start} rotation={-90} origin="86, 86" fill="none"/>;})}</Svg>
  <View pointerEvents="none" style={{position:'absolute',top:0,bottom:0,left:0,right:0,alignItems:'center',justifyContent:'center'}}><Text style={{fontSize:29,fontWeight:'700'}}>{centre}</Text><Text style={{fontSize:14,color:neutral.muted}}>{unit}</Text></View>
 </View>;
}
const statusColours={active:palettes.goal.ink,maintaining:palettes.family.ink,achieved:palettes.classroom.ink,paused:palettes.method.ink,not_started:palettes.difficulty.ink};
export function GoalOverview({goals,report,view,changeView,days,source,error,retry,openGoal}:{goals:Goal[];report:Coverage|null;view:'coverage'|'progress';changeView:(v:'coverage'|'progress')=>void;days:number;source:Coverage['source'];error:string;retry:()=>void;openGoal:(id:string)=>void}){
 const [expanded,setExpanded]=useState(false),[state,setState]=useState<OverviewState|null>(null);
 const current=report?.source===source&&report.days===days?report:null,rows=goalOverviewRows(goals,current),groups=goalStatusGroups(goals);
 const total=rows.reduce((n,r)=>n+(r.counts?.selected??0),0),known=rows.filter(r=>r.counts!==null),covered=known.filter(r=>r.counts!.selected>0),zero=known.filter(r=>r.counts!.selected===0),unknown=rows.length-known.length;
 const title=source==='family'?'家庭':source==='lesson'?'机构':'合计';
 return <View testID="goal-overview" style={{gap:12,padding:16,borderRadius:16,borderWidth:1,borderColor:palettes.goal.border,backgroundColor:neutral.surface}}>
  <View style={{flexDirection:'row',alignItems:'center',gap:10}}><Text accessibilityRole="header" style={{flex:1,fontSize:20,fontWeight:'700',color:palettes.goal.ink}}>目标总览</Text><Text style={{fontSize:14,color:neutral.muted}}>当前范围 {goals.length} 项</Text></View>
  <SectionTabs label="目标总览视角" value={view} change={changeView} items={[{value:'coverage',title:'覆盖分布',tone:'experience'},{value:'progress',title:'推进状态',tone:'experience'}]}/>
  {view==='coverage'?<>
   <Text>近{days}天 · {title}实际记录</Text>
   {error&&<><Text accessibilityRole="alert">{error}</Text><UiButton title="重新读取目标总览" press={retry}/></>}
   {!current&&!error&&<Text role="status">正在读取覆盖，尚未读到的目标不按零次显示。</Text>}
   <View style={{flexDirection:'row',alignItems:'center',justifyContent:'space-around',gap:12,flexWrap:'wrap'}}>
    <OverviewRing slices={rows.map((r,i)=>({id:r.goal.id,value:r.counts?.selected??0,color:chartColors[i%chartColors.length]}))} total={total} label={`目标覆盖环图，近${days}天${title}，${current?`共${total}次目标命中，${covered.length}个目标有记录、${zero.length}个为零次、${unknown}个尚未读到`:'数据尚未读到'}；这是记录覆盖，不是能力百分比`} centre={current?String(total):'—'} unit={current?'次目标命中':'尚未读到'}/>
    <View style={{gap:9}}><Text>有记录：{current?covered.length:'—'} 项</Text><Text>尚无记录：{current?zero.length:'—'} 项</Text>{unknown>0&&<Text>未读到：{unknown} 项</Text>}</View>
   </View>
   <Text style={{fontSize:14,color:neutral.muted}}>草稿和计划不计入。各级目标包含直接选择及下级关联，同一记录对同一目标只算一次；长期也保留原有明确关注。合计不等于干预场数，实际能力变化在下方测量中查看。</Text>
   {rows.slice(0,expanded?rows.length:4).map((r,i)=><Pressable key={r.goal.id} accessibilityRole="button" accessibilityLabel={'查看总览目标：'+r.goal.input.title} onPress={()=>openGoal(r.goal.id)} style={{gap:4,paddingVertical:9,borderTopWidth:1,borderColor:neutral.border}}><View style={{flexDirection:'row',alignItems:'center',gap:9}}><View aria-hidden style={{width:10,height:10,borderRadius:3,backgroundColor:chartColors[i%chartColors.length]}}/><Text style={{flex:1}}>{r.goal.input.title}</Text><Text style={{fontWeight:'600'}}>{r.counts?`${r.counts.selected} 次`:'未读到'}</Text><Text>›</Text></View><Text style={{fontSize:14,color:neutral.muted}}>{levelNames[r.goal.input.level]}{r.counts?` · 家庭${r.counts.family}／机构${r.counts.lesson}`:''}</Text></Pressable>)}
   {rows.length>4&&<UiButton title={expanded?'收起目标图例':`展开全部目标图例（${rows.length}项）`} variant="link" press={()=>setExpanded(v=>!v)}/>}
  </>:<>
   <Text>按家人保存的推进状态汇总，包含当前范围内各期限的目标。</Text>
   <OverviewRing slices={groups.map(g=>({id:g.id,value:g.goals.length,color:statusColours[g.id]}))} total={goals.length} label={'目标推进状态环图，'+groups.map(g=>g.label+g.goals.length+'项').join('，')} centre={String(goals.length)} unit="个目标"/>
   {groups.map(g=><Pressable key={g.id} accessibilityRole="button" accessibilityLabel={'查看状态目标：'+g.label} disabled={!g.goals.length} onPress={()=>setState(g.id)} style={{flexDirection:'row',alignItems:'center',gap:10,paddingVertical:10,borderTopWidth:1,borderColor:neutral.border}}><View aria-hidden style={{width:10,height:10,borderRadius:3,backgroundColor:statusColours[g.id]}}/><Text style={{flex:1}}>{g.label}</Text><Text style={{fontWeight:'700'}}>{g.goals.length} 项</Text>{g.goals.length>0&&<Text>›</Text>}</Pressable>)}
   <Text style={{fontSize:14,color:neutral.muted}}>状态由家人明确调整；不同能力的测量单位和帮助条件不同，逐目标看自己的变化。下方仍保留各目标实际测量及详情入口。</Text>
  </>}
  {!goals.length&&<Text>当前范围暂无目标，可新建或调整筛选。</Text>}
  {state&&<DetailOverlay title="这一状态的目标" tone="goal" close={()=>setState(null)}><Text style={{fontSize:18,fontWeight:'700'}}>{groups.find(g=>g.id===state)?.label}</Text>{groups.find(g=>g.id===state)?.goals.map(g=><UiButton key={g.id} title={'查看状态目标详情：'+g.input.title} label={`${g.input.title} · ${levelNames[g.input.level]} · ${goalStateName(g.input)}`} press={()=>openGoal(g.id)}/>)}</DetailOverlay>}
 </View>;
}

import React,{useEffect,useState} from 'react';
import {Disclosure} from './disclosure';
import {View} from 'react-native';
import Svg,{Circle} from 'react-native-svg';
import {Text,Pressable} from './readability';
import {Select} from './controls';
import {DetailOverlay} from './overlay';
import {request} from './api';
import type {Snapshot} from './foundation';
import type {Coverage} from './prd-sessions';
import {coverageItems,coverageColors,coverageLegend,type CoverageTopic,type CoverageItem} from './prd-coverage';

const palette=['#245D48','#315B82','#744C64','#775521','#4A536D','#52622F','#665285','#795047'];
const Button=({title,press,disabled=false}:{title:string;press:()=>void;disabled?:boolean})=><Pressable accessibilityRole="button" accessibilityState={{disabled}} disabled={disabled} onPress={press} style={{padding:12,borderRadius:10,backgroundColor:'#E9F2E9',opacity:disabled?.5:1}}><Text>{title}</Text></Pressable>;
function Ring({items}:{items:CoverageItem[]}){
 const total=items.reduce((n,r)=>n+r.counts.selected,0),circumference=2*Math.PI*70;let offset=0;
 return <View style={{alignItems:'center',gap:8}}>
  <View accessible accessibilityRole="image" accessibilityLabel={`当前显示项覆盖环图，合计${total}次命中；每项数量列在下方，不表示能力百分比`}>
   <Svg width={190} height={190} viewBox="0 0 190 190" aria-hidden><Circle cx={95} cy={95} r={70} stroke="#B5C1B4" strokeWidth={22} fill="none"/>{total>0&&items.map((r,i)=>{const length=r.counts.selected/total*circumference,start=offset;offset+=length;return r.counts.selected>0?<Circle key={r.id} cx={95} cy={95} r={70} stroke={palette[i%palette.length]} strokeWidth={22} strokeDasharray={`${length} ${circumference-length}`} strokeDashoffset={-start} rotation={-90} origin="95, 95" fill="none"/>:null;})}</Svg>
  </View>
  <Text>{total?`当前显示项共${total}次命中`:'本窗口当前显示项还没有记录；保留空环，不绘制假比例。'}</Text>
 </View>;
}

export function CoveragePanel({data,close,chooseGoal,selectedGoals=[],selectedFocus=[]}:{data:Snapshot;close:()=>void;chooseGoal?:(id:string,long:boolean)=>void;selectedGoals?:string[];selectedFocus?:string[]}){
 const [source,setSource]=useState<'family'|'lesson'|'combined'>('family'),[days,setDays]=useState(30),[topic,setTopic]=useState<CoverageTopic>('goals'),[level,setLevel]=useState('short'),[chart,setChart]=useState('bar'),[unrecorded,setUnrecorded]=useState(false),[retry,setRetry]=useState(0),[report,setReport]=useState<Coverage|null>(null),[error,setError]=useState('');
 useEffect(()=>{let cancel=false;setReport(null);setError('');void request<Coverage>(`/prd/coverage?days=${days}&source=${source}`).then(r=>{if(!cancel)setReport(r)}).catch(e=>{if(!cancel)setError(e instanceof Error?e.message:'暂未读到近况，请重试。')});return()=>{cancel=true};},[source,days,retry]);
 const rows=report?coverageItems(data,report,topic,level,unrecorded):[],max=Math.max(1,...rows.map(r=>r.counts.selected));
 return <DetailOverlay title="全部方向的近况" close={close}>
  <Disclosure title="调整统计范围与图表" summary={`${{family:'家庭',lesson:'机构',combined:'合计'}[source]} · 近${days}天`}>
  <Select label="统计来源" value={source} options={[{value:'family',label:'家庭干预'},{value:'lesson',label:'机构课程'},{value:'combined',label:'合计（分列两种来源）'}]} onChange={v=>setSource(v as typeof source)}/>
  <Select label="统计窗口" value={String(days)} options={[7,30,90].map(n=>({value:String(n),label:`近${n}天`}))} onChange={v=>setDays(Number(v))}/>
  <Select label="近况统计内容" value={topic} onChange={v=>setTopic(v as CoverageTopic)} options={[{value:'goals',label:'目标覆盖'},{value:'methods',label:'实际方法'},{value:'tags',label:'实际活动标签（家庭）'}]}/>
  {topic==='goals'&&<><Select label="近况目标层级" value={level} onChange={setLevel} options={[{value:'short',label:'近期本领'},{value:'medium',label:'中期目标'},{value:'long',label:'长期方向（明确关注）'}]}/><Pressable accessibilityRole="checkbox" accessibilityLabel="只看本窗口尚无记录的目标" accessibilityState={{checked:unrecorded}} aria-checked={unrecorded} onPress={()=>setUnrecorded(v=>!v)} style={{padding:12,backgroundColor:'#E9F2E9',borderRadius:10}}><Text>{unrecorded?'☑':'☐'} 只看本窗口尚无记录的目标</Text></Pressable></>}
  <Select label="近况展示方式" value={chart} onChange={setChart} options={[{value:'bar',label:'条形图'},{value:'ring',label:'环图'},{value:'table',label:'数量明细'}]}/>
  </Disclosure>
  <Text>次数表示记录覆盖，不代表能力进展；未记录不等于未安排。图表仅包含当前显示项，一条记录可以涉及多个目标、方法或标签，命中数之和可能大于记录场次。</Text>
  {error&&<><Text accessibilityRole="alert">{error}</Text><Button title="重新读取近况" press={()=>setRetry(n=>n+1)}/></>}{!report&&!error&&<Text>正在读取近况…</Text>}
  {report&&<>
   <Text>{report.start} 至 {report.end} · 家庭 {report.records.family} 次，机构 {report.records.lesson} 节</Text>
   {topic!=='tags'&&source!=='combined'&&<Text>本图只计{source==='family'?'家庭干预':'机构课程'}；另一来源的分量供参考，不合并进本图。</Text>}
   {topic==='goals'&&level!=='long'&&<Text>近{days}天固定色阶：{coverageLegend(days)}。数字与颜色一同显示；筛选不改变颜色含义。</Text>}
   {topic==='tags'&&source!=='family'&&<Text>活动标签只记录家庭实际活动，机构课程没有同口径标签；这里始终显示家庭分量。</Text>}
   {chart==='ring'&&<Ring items={rows}/>}
   {rows.map((r,i)=>{const selected=level==='long'?selectedFocus.includes(r.id):selectedGoals.includes(r.id),dark=r.band===4,text=dark?'#FFFFFF':'#243D36';return <View key={r.id} style={{gap:8,padding:12,borderRadius:10,backgroundColor:topic==='goals'&&r.band!==undefined?coverageColors[r.band]:'#FFFFFF'}}>
    <Text style={{fontWeight:'600',color:text}}>{chart==='ring'?`${i+1}. `:''}{r.label} · {r.counts.selected} 次{topic==='tags'?'标签命中':'覆盖'}</Text>
    <Text style={{color:text}}>家庭{r.counts.family}／机构{r.counts.lesson}</Text>
    {chart==='ring'&&<View aria-hidden style={{height:5,width:44,backgroundColor:palette[i%palette.length]}}/>}
    {chart==='bar'&&<View accessible accessibilityRole="image" accessibilityLabel={`${r.label}覆盖条，家庭${r.counts.family}次，机构${r.counts.lesson}次`} style={{height:20,flexDirection:'row',backgroundColor:'#F6F5EF',borderWidth:1,borderColor:'#53665A',borderRadius:4,overflow:'hidden'}}>{r.counts.family>0&&<View style={{width:`${(topic==='tags'||source!=='lesson'?r.counts.family:0)/max*100}%`,backgroundColor:'#245D48'}}/>}{r.counts.lesson>0&&<View style={{width:`${(source!=='family'?r.counts.lesson:0)/max*100}%`,backgroundColor:'#315B82'}}/>}</View>}
    <Text style={{color:text}}>{r.detail}</Text>
    {topic==='goals'&&chooseGoal&&<Button title={selected?`本次已选：${r.label}`:`${level==='long'?'加入本次长期关注':'加入本次目标'}：${r.label}`} disabled={selected} press={()=>chooseGoal(r.id,level==='long')}/>}
   </View>;})}
   {!rows.length&&<Text>{unrecorded&&topic==='goals'?'本窗口这一层级没有尚无记录的目标。':'这一范围暂无记录或可显示项目。'}</Text>}
   <Text>方法未记录：{report.method_unknown.selected} 次；明确未用库中方法：{report.no_library_method.selected} 次</Text><Text>家庭实际活动：{Object.entries(report.tags).map(([t,n])=>`${t} ${n}次`).join('、')||'尚未分类'}；未分类 {report.unclassified} 次</Text>
  </>}
 </DetailOverlay>;
}

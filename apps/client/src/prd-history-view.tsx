import React,{useMemo,useState} from 'react';
import {View,useWindowDimensions} from 'react-native';
import {HistoryDayPlot} from './history-day-plot';
import {DetailOverlay} from './overlay';
import {Select} from './controls';
import {Pressable,Text} from './readability';
import {localDate} from './draft';
import type {Snapshot} from './foundation';
import type {Session} from './prd-sessions';
import type {Lesson} from './prd-lessons';
import {historyEvidence,matchesEvidence,timeDistribution,daysAgo,type Evidence} from './prd-history';
const colors=['#F0F3ED','#DFEAD8','#BED4A9','#8BAF7D','#4E7656'];
const button=(title:string,press:()=>void)=><Pressable accessibilityRole="button" onPress={press} style={{padding:12,minHeight:46,borderRadius:10,backgroundColor:'#E9F2E9'}}><Text>{title}</Text></Pressable>;
export function HistoryPanel({data,close,object,openSession}:{data:Snapshot;close:()=>void;object?:{id:string;kind:'goal'|'method';version?:number};openSession?:(id:string)=>void}){
 const wide=useWindowDimensions().width>700;const [daily,setDaily]=useState(true);
 const [source,setSource]=useState<'family'|'lesson'|'combined'>(object?'combined':'family'),[level,setLevel]=useState('short'),[selected,setSelected]=useState<string|null>(object?.id??null),[mode,setMode]=useState(object?.version?'30':'90'),[detail,setDetail]=useState<Evidence|null>(null);
 const today=localDate(),all=useMemo(()=>historyEvidence(data,today,source),[data,today,source]);
 const matching=(id:string)=>matches(data,id,all,object?.kind??'goal').filter(e=>!object?.version||(e.kind==='family'?(e.record as Session).actual_methods:(e.record as Lesson).organization.methods).some(m=>m.id===id&&m.version===object.version));
 const rows=selected?matching(selected):[],distribution=timeDistribution(rows,today),recent=mode==='30'?rows.slice(0,30):rows.filter(r=>r.date>=distribution.start);
 const selectedName=object?.kind==='method'?data.methods.find(m=>m.id===selected)?.input.title:data.goals.find(g=>g.id===selected)?.input.title;
 return <DetailOverlay title="记录时间分布" close={close}>
  {object?.version&&<Text>只看方法第{object.version}版的实际记录。</Text>}<Text>覆盖次数与能力进展分别看。空白表示未记录，不表示没有陪伴或没有进步。</Text>
  {object&&<Select label="时间分布来源" value={source} onChange={v=>setSource(v as typeof source)} options={[{value:'family',label:'家庭干预'},{value:'lesson',label:'机构课程'},{value:'combined',label:'合计（保留来源）'}]}/>}
  {!object&&<><Select label="并列目标层级" value={level} onChange={v=>{setLevel(v);setSelected(null);}} options={[{value:'short',label:'近期本领'},{value:'medium',label:'中期目标'},{value:'long',label:'长期方向'}]}/>{wide&&<Select label="时间分布粒度" value={daily?'day':'week'} onChange={v=>setDaily(v==='day')} options={[{value:'day',label:'按日点图'},{value:'week',label:'13周小条'}]}/>}<Text>{wide&&daily?'近90天 · 日粒度仅在宽屏展示，整行打开明细。':'近90天 · 从左到右由远及近，每格一周；首格可能不足7天。颜色深浅对应0／1／2／3／4+次，数字同显。整条点击看明细。'}</Text>{data.goals.filter(g=>g.input.level===level&&!g.input.archived).map(g=>{
   const r=matching(g.id),d=timeDistribution(r,today),long=g.input.level==='long';
   return <Pressable key={g.id} accessibilityRole="button" accessibilityLabel={`查看时间分布：${g.input.title}`} onPress={()=>setSelected(g.id)} style={{gap:8,padding:12,borderRadius:10,backgroundColor:'#FFFFFF',minHeight:80}}><Text style={{fontWeight:'600'}}>{g.input.title}</Text><Text>最近一次 {d.last?`${d.last}（${daysAgo(d.last,today)}天前）`:'尚无记录'} · 近90天 {d.count} 次{long?'（明确关注或下级证据）':''}</Text>{wide&&daily?<HistoryDayPlot rows={r} today={today}/>:<View aria-hidden style={{flexDirection:'row',gap:3}}>{d.weeks.map(w=><View key={w.start} style={{flex:1,height:24,borderRadius:3,backgroundColor:colors[Math.min(w.count,4)],justifyContent:'center',alignItems:'center'}}><Text style={{fontSize:12,color:w.count>=4?'#FFFFFF':'#243D36'}}>{w.count}</Text></View>)}</View>}</Pressable>;
  })}</>}
  {selected&&<View style={{gap:10,paddingVertical:10}}><Text accessibilityRole="header" style={{fontSize:20,fontWeight:'600'}}>{selectedName??'历史对象'}的记录</Text><Select label="时间明细范围" value={mode} onChange={setMode} options={[{value:'90',label:'近90天'},{value:'30',label:'最近30次（不限时间跨度）'}]}/><Text>{mode==='30'?(recent.length?`最近${recent.length}次，实际跨度 ${recent.at(-1)!.date} 至 ${recent[0].date}`:'尚无实际记录'):`${distribution.start} 至 ${today} · ${recent.length} 次`}</Text>{recent.map(r=><View key={r.kind+':'+r.id} style={{gap:5,padding:10,borderRadius:8,backgroundColor:'#FFFFFF'}}><Text>{r.date} · {r.kind==='family'?'家庭干预':'机构课程'}{r.below.includes(selected)?' · 下级证据':''}{r.focus.includes(selected)?' · 明确长期关注':''}</Text><Text>{description(r)}</Text>{button(`查看记录依据：${r.date} · ${r.kind==='family'?'家庭':'机构'} · ${r.id.slice(0,6)}`,()=>setDetail(r))}</View>)}{!object&&button('收起目标明细',()=>setSelected(null))}</View>}
  {detail&&<DetailOverlay title="时间分布记录依据" close={()=>setDetail(null)}><Text>{detail.date} · {detail.kind==='family'?'家庭干预':'机构课程'}</Text><Text selectable>{description(detail)}</Text>{detail.kind==='family'?<><Text>实际方法：{(detail.record as Session).input.summary?.confirmation==='unknown'?'记不清':(detail.record as Session).actual_methods.map(m=>`${m.input.title}（第${m.version}版）`).join('、')||'本次没有使用库中方法'}</Text>{openSession&&button('打开这次干预记录',()=>{openSession(detail.id);setDetail(null);})}</>:<><Text>{(detail.record as Lesson).course.name} · {(detail.record as Lesson).teacher.name} · 记录者 {(detail.record as Lesson).creator_name}</Text><Text>老师的做法和建议：{(detail.record as Lesson).observation.teacher_notes||'未记录'}</Text></>}</DetailOverlay>}
 </DetailOverlay>;
}
function matches(data:Snapshot,id:string,all:Evidence[],kind:'goal'|'method'){
 const long=kind==='goal'&&data.goals.find(g=>g.id===id)?.input.level==='long';return all.filter(r=>matchesEvidence(r,id,kind,long));
}
function description(r:Evidence){return r.kind==='family'?(r.record as Session).input.summary?.targets.filter(t=>t.performed).map(t=>t.observation).filter(Boolean).join('；')||'未补充文字观察':[(r.record as Lesson).observation.notes,...(r.record as Lesson).organization.targets.map(t=>t.target.note)].filter(Boolean).join('；')||'未补充课堂观察';}

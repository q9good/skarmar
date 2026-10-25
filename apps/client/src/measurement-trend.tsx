import React,{useState} from 'react';
import {View} from 'react-native';
import Svg,{Line,Circle,Polyline,Text as SvgText} from 'react-native-svg';
import {Text,useReadability} from './readability';
import type {MeasureSeries} from './prd-progress';
export function MeasurementTrend({series}:{series:MeasureSeries}){
 const [width,setWidth]=useState(288),{large}=useReadability();
 const points=series.points,valid=points.filter(p=>p.value!==null),font=large?18:14,h=206,left=font*4+8,right=14,top=14,bottom=45;
 if(!['count','duration'].includes(series.template))return <Text>文字／步骤观察保留原文，不转换成分数或折线。</Text>;
 if(!valid.length)return <Text>待观察：尚无可绘制的实际数值。零机会或未填写的值不按0计算。</Text>;
 const max=series.template==='count'?100:Math.max(...valid.map(p=>p.value!),1),min=0,start=Date.parse(points[0].date+'T00:00:00Z'),end=Date.parse(points.at(-1)!.date+'T00:00:00Z');
 const x=(date:string)=>left+(end===start?(width-left-right)/2:(Date.parse(date+'T00:00:00Z')-start)/(end-start)*(width-left-right));
 const y=(v:number)=>h-bottom-(v-min)/(max-min)*(h-bottom-top);
 const paths:string[][]=[];let path:string[]=[];
 for(const p of points){if(p.value===null){if(path.length)paths.push(path);path=[];}else path.push(`${x(p.date)},${y(p.value)}`);}if(path.length)paths.push(path);
 return <View onLayout={e=>setWidth(Math.max(180,Math.floor(e.nativeEvent.layout.width)))} style={{gap:8}}>
  <Text>日期为横轴，{series.template==='count'?'该次机会内做到比例':'实际值'}为纵轴；只显示已记录的测量，不代表整体能力达成率。</Text>
  {!series.comparable&&<Text>帮助条件尚未记录，仅展示各次点位，不连接为可比趋势。</Text>}
  <View accessible accessibilityRole="image" accessibilityLabel={`${series.name}测量图，共${valid.length}个有效数值点，${points.length-valid.length}条未计算；${points[0].date}至${points.at(-1)!.date}`}>
   <Svg width="100%" height={h} viewBox={`0 0 ${width} ${h}`} aria-hidden>
    {[0,max/2,max].map(v=><React.Fragment key={v}><Line x1={left} x2={width-right} y1={y(v)} y2={y(v)} stroke="#B5C1B4"/><SvgText x={left-6} y={y(v)+font/3} fontSize={font} fill="#243D36" textAnchor="end">{v>=100000?v.toExponential(0):Number(v.toFixed(1))}</SvgText></React.Fragment>)}
    {series.comparable&&paths.filter(p=>p.length>1).map((p,i)=><Polyline key={i} points={p.join(' ')} fill="none" stroke="#245D48" strokeWidth={2}/>)}
    {valid.map(p=><Circle key={p.id} cx={x(p.date)} cy={y(p.value!)} r={4} fill="#245D48"/>)}
    <SvgText x={left} y={h-12} fontSize={font} fill="#243D36">{points[0].date.slice(5)}</SvgText>{points[0].date!==points.at(-1)!.date&&width-left-right>font*6&&<SvgText x={width-right} y={h-12} fontSize={font} fill="#243D36" textAnchor="end">{points.at(-1)!.date.slice(5)}</SvgText>}
   </Svg>
  </View>
  <Text>最近一次：{points.at(-1)!.date} · {points.at(-1)!.label}{points.length>1?`；本段最早：${points[0].date} · ${points[0].label}`:''}。同日多次均保留，不取平均；未记录的日期不补数值。</Text>
 </View>;
}

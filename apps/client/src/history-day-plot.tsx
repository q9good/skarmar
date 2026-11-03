import {neutral,palettes} from './ui-palette';
import React,{useState} from 'react';
import {View} from 'react-native';
import Svg,{Circle,Line} from 'react-native-svg';
import type {Evidence} from './prd-history';
import {dailyDistribution} from './prd-history';
import {Text} from './readability';
export function HistoryDayPlot({rows,today}:{rows:Evidence[];today:string}){
 const [width,setWidth]=useState(560),days=dailyDistribution(rows,today);
 return <View onLayout={e=>setWidth(e.nativeEvent.layout.width)} style={{gap:5}}>
  <View accessible accessibilityRole="image" accessibilityLabel={`近90天日分布，${days.filter(d=>d.count>0).length}天有记录，${days.reduce((n,d)=>n+d.count,0)}条记录；整行打开日期明细`}>
   <Svg width="100%" height={30} viewBox={`0 0 ${Math.max(width,90)} 30`} aria-hidden>
    <Line x1={5} y1={15} x2={Math.max(width,90)-5} y2={15} stroke={neutral.border}/>
    {days.map((d,i)=><Circle key={d.date} cx={5+i/89*(Math.max(width,90)-10)} cy={15} r={d.count?Math.min(2.2+d.count*.3,3.5):1.4} fill={d.count?palettes.family.ink:'#FFFFFF'} stroke={d.count?palettes.family.ink:neutral.muted}/>)}
   </Svg>
  </View>
  <Text style={{fontSize:14}}>{days[0].date} → {today} · 每点一天；空心为0条，实心表示已记录。整行打开可读日期和数量，不逐个点选小点。</Text>
 </View>;
}

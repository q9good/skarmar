import React from 'react';
import {View} from 'react-native';
import {Text} from './readability';
import {Disclosure} from './disclosure';
import type {Snapshot} from './foundation';
import type {Coverage} from './prd-sessions';

/** Read the same server report as the full coverage view; drafts are never counts. */
export function PlanCoveragePreview({data,report}:{data:Snapshot;report:Coverage|null}) {
 const goals=data.goals.filter(g=>!g.input.archived&&g.input.level!=='long'&&['active','maintaining'].includes(g.input.state??'active'));
 return <View style={{gap:8,padding:12,borderRadius:10,backgroundColor:'#E8EFF5'}}>
  <Text accessibilityRole="header" style={{fontSize:17,fontWeight:'600'}}>各方向覆盖 · 近30天</Text>
  {!report?<Text>近况尚未读到，请用下方“查看全部方向的近况”重试；未读到不按零次显示。</Text>:<>
   <Text style={{fontSize:14}}>{report.start} 至 {report.end} · 次数不是能力评分</Text>
   {goals.map(g=>{const row=report.goals.find(r=>r.id===g.id);return <View key={g.id} testID={`plan-coverage-${g.id}`} style={{gap:3,paddingTop:7,borderTopWidth:1,borderColor:'#BBCDD9'}}>
    <Text style={{fontWeight:'600',fontSize:15}}>{g.input.title}</Text>
    <Text style={{fontSize:14}}>{row?`家庭 ${row.counts.family} 次 · 机构 ${row.counts.lesson} 节`:'此目标近况尚未读到'}</Text>
   </View>;})}
   {!goals.length&&<Text>还没有正在推进的中短期目标，可在下方选择或新建。</Text>}
   <Disclosure title="方法使用频次（近30天）">
    {data.methods.filter(m=>m.input.state==='active').map(m=><Text key={m.id}>{m.input.title} · 家庭 {report.methods[m.id]?.family??0} 次／机构 {report.methods[m.id]?.lesson??0} 节</Text>)}
    {!data.methods.some(m=>m.input.state==='active')&&<Text>还没有使用中的方法。</Text>}
   </Disclosure>
  </>}
 </View>;
}

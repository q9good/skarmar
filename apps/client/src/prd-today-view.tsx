import React from 'react';
import {View} from 'react-native';
import {Pressable,Text} from './readability';
import type {Snapshot} from './foundation';
import type {EvidenceRecord} from './prd-observations';
import {todayRecords,type TodayRecordKind} from './prd-today';

export function TodayRecords({data,moments,today,open}:{data:Snapshot;moments:EvidenceRecord[];today:string;open:(kind:TodayRecordKind,id:string)=>void}){
 const records=todayRecords(data,moments,today);
 return <View style={{gap:10,padding:14,borderRadius:12,backgroundColor:'#FFFFFF'}}>
  <Text accessibilityRole="header" style={{fontSize:18,fontWeight:'600'}}>今天已记录</Text>
  {!records.length&&<Text>今天还没有已同步记录；普通陪伴不需要为了完整而补记。</Text>}
  {records.map(record=><View key={record.key} testID={'today-record-'+record.kind+'-'+record.id} style={{gap:5,paddingTop:records.indexOf(record)?10:0,borderTopWidth:records.indexOf(record)?1:0,borderColor:'#CED8CE'}}>
   <Text style={{fontWeight:'600'}}>{record.label} · {record.timing}</Text>
   <Text selectable numberOfLines={2}>{record.title}</Text>
   <Text>记录者：{record.author} · {record.sync}</Text>
   {record.kind!=='moment'&&<Pressable accessibilityRole="button" accessibilityLabel={(record.kind==='lesson'?'查看课记：':'查看家庭干预：')+record.title} onPress={()=>open(record.kind,record.id)} style={{padding:10,minHeight:44,borderRadius:8,backgroundColor:'#E9F2E9'}}><Text style={{color:'#34765B'}}>查看这条记录</Text></Pressable>}
  </View>)}
 </View>;
}

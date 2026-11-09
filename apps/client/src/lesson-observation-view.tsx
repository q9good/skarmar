import React from 'react';
import {View} from 'react-native';
import {Text} from './readability';
import {neutral} from './ui-palette';
import {observationItems} from './observation-items';
import type {Observation} from './prd-lessons';
export function ReadObservation({value}:{value:Observation}){
 const labels:Record<string,string>={notes:'课堂观察',teacher_notes:'老师建议',helpful_notes:'好方法',progress_notes:'进展与运用',interest_notes:'兴趣与互动',difficulty_notes:'困难与回应',cause_guess:'原因猜测（待核实）'};
 const items=observationItems(value);
 return <View style={{gap:8}}>{value.course_started&&<Text>家人明确标记：这门课程刚开始上。</Text>}{items.map((i,index)=><View key={i.id} style={{gap:4,paddingBottom:8,borderBottomWidth:1,borderColor:neutral.border}}><Text style={{fontWeight:'600'}}>{index+1}. {labels[i.kind]}</Text><Text selectable>{i.content}</Text></View>)}{!items.length&&<Text>还没有补充课堂观察。</Text>}</View>;
}

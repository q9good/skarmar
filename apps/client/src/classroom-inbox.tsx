import React from 'react';
import {View} from 'react-native';
import {Text} from './readability';
import {Disclosure} from './disclosure';
import {ReadObservation} from './lesson-observation-view';
import {UiButton} from './ui-components';
import type {Snapshot} from './foundation';
export function ClassroomInbox({data,organize,close}:{data:Snapshot;organize?:(id:string)=>void;close?:(id:string)=>void}){
 const rows=data.lessons.filter(l=>l.status==='recorded'&&l.request_generation>0&&(l.organization.state!=='done'||l.organization.processed_request_generation<l.request_generation)).sort((a,b)=>b.observation.business_date.localeCompare(a.observation.business_date)||Date.parse(b.created_at)-Date.parse(a.created_at)||a.id.localeCompare(b.id));
 const dates=[...new Set(rows.map(l=>l.observation.business_date))];
 return <View testID="unprocessed-classroom-inbox" style={{gap:12}}><Text style={{fontWeight:'700',fontSize:18}}>未处理课堂 · {rows.length} 条</Text>{dates.map(date=><View key={date} style={{gap:8}}><Text accessibilityRole="header" style={{fontWeight:'600'}}>{date}</Text>{rows.filter(l=>l.observation.business_date===date).map((l,i,list)=>{const same=list.filter(r=>r.course.name===l.course.name&&r.teacher.name===l.teacher.name).length>1;return <View key={l.id} testID={'classroom-inbox-'+l.id}><Disclosure label={`${l.course.name} · ${l.teacher.name}${same?' · 第'+(i+1)+'条':''}`} title={`待处理课堂：${date} · ${l.course.name} · ${l.teacher.name}${same?' · 第'+(i+1)+'条':''}`}><Text>记录者：{l.creator_name}{l.marked_error?' · 待核对课记':''}</Text>{l.request_name&&<Text>待回看请求：{l.request_name}</Text>}{l.request_message&&<Text>留言：{l.request_message}</Text>}{l.organization.claimed_by&&<Text>已有家人在整理，其他家人仍可查看和接手。</Text>}<ReadObservation value={l.observation}/>{l.organization.outcome&&<Text>上次整理说明：{l.organization.outcome}</Text>}<Text>看完后再选择处理方式；关闭待办会保留原课记。</Text><View testID={'classroom-inbox-actions-'+l.id} style={{gap:8}}>{organize&&<UiButton title={`整理课堂材料：${date} · ${l.course.name}`} label="整理课堂材料" tone="classroom" press={()=>organize(l.id)}/>} {close&&<UiButton title={`关闭课堂材料：${date} · ${l.course.name}`} label="关闭本次课堂待办" tone="classroom" variant="link" press={()=>close(l.id)}/>}</View></Disclosure></View>;})}</View>)}{!rows.length&&<Text>暂时没有待处理的课堂记录。</Text>}</View>;
}

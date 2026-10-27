import {DetailOverlay} from './overlay';
import { Pressable, Text, TextInput } from './readability';
import React, { useState } from 'react';
import { View } from 'react-native';
import {localDate} from './draft';
export type SelectProps = { label: string; value: string; options: { value: string; label: string }[]; onChange: (value: string) => void; disabled?: boolean; compact?:boolean };
export type DateProps = { label: string; value: string; onChange: (value: string) => void; disabled?: boolean; question?: string; emptyText?: string; help?: string };
export function Select({ label, value, options, onChange, disabled, compact=false }: SelectProps) {
 const [open,setOpen]=useState(false),[search,setSearch]=useState('');
 const choices=(options.length<=6?options:options.filter(o=>o.value===value)).map(o=><Pressable key={o.value} accessibilityRole="radio" accessibilityLabel={o.label} accessibilityState={{checked:o.value===value,disabled}} disabled={disabled} onPress={()=>onChange(o.value)} style={{padding:10,backgroundColor:'#E9F2E9',borderRadius:8}}><Text>{o.value===value?'◉':'○'} {o.label}</Text></Pressable>);
 return <View style={{gap:8}}>{!compact&&<Text>{label}</Text>}{choices}{options.length>6&&<Pressable accessibilityRole="button" disabled={disabled} onPress={()=>{setSearch('');setOpen(true);}}><Text>更多{label}</Text></Pressable>}{open&&<DetailOverlay title={'选择'+label} close={()=>setOpen(false)}>{options.length>15&&<TextInput accessibilityLabel={'搜索'+label} value={search} onChangeText={setSearch} placeholder="输入名称或关键词"/>}{options.filter(o=>o.label.includes(search)).map(o=><Pressable key={o.value} accessibilityRole="button" onPress={()=>{onChange(o.value);setOpen(false);}}><Text>{o.label}</Text></Pressable>)}</DetailOverlay>}</View>;
}
const isoDate=(value:string)=>{if(!/^\d{4}-\d{2}-\d{2}$/.test(value))return false;const date=new Date(value+'T00:00:00Z');return Number.isFinite(date.getTime())&&date.toISOString().slice(0,10)===value;};
const currentMonth=()=>localDate().slice(0,7);
const monthFor=(value:string)=>isoDate(value)?value.slice(0,7):currentMonth();
const shiftMonth=(month:string,offset:number)=>{const [year,value]=month.split('-').map(Number);return new Date(Date.UTC(year,value-1+offset,1)).toISOString().slice(0,7);};
const monthDates=(month:string)=>{const [year,value]=month.split('-').map(Number),count=new Date(Date.UTC(year,value,0)).getUTCDate();return Array.from({length:count},(_,index)=>month+'-'+String(index+1).padStart(2,'0'));};
const formatDigits=(raw:string)=>/^\d{8}$/.test(raw)?raw.slice(0,4)+'-'+raw.slice(4,6)+'-'+raw.slice(6):raw;
export function DateField({ label, value, onChange, disabled, question, emptyText, help }: DateProps) {
  const [mode,setMode]=useState<'calendar'|'digits'>(()=>value&&!isoDate(value)?'digits':'calendar'),[open,setOpen]=useState(false),[month,setMonth]=useState(()=>monthFor(value)),[helpOpen,setHelpOpen]=useState(false);
  const switchMode=()=>setMode(current=>current==='calendar'?'digits':'calendar');
  return <View style={{gap:8}}>
    <View style={{flexDirection:'row',alignItems:'center',flexWrap:'wrap',gap:8}}>
      <Text style={{fontSize:17,flexShrink:1}}>{label}</Text>
      <Pressable accessibilityRole="button" accessibilityLabel={label+'：'+(mode==='calendar'?'改用数字输入':'改用日历选择')} disabled={disabled} onPress={switchMode} style={{padding:8,borderRadius:8,backgroundColor:'#E9F2E9'}}><Text style={{fontSize:14}}>{mode==='calendar'?'数字输入':'日历选择'}</Text></Pressable>
    </View>
    {question&&<Text style={{fontSize:14,color:'#53665A'}}>{question}</Text>}
    {mode==='calendar'?<Pressable accessibilityRole="button" accessibilityLabel={label+'：打开日历'} disabled={disabled} onPress={()=>{setMonth(monthFor(value));setOpen(true);}} style={{padding:12,borderWidth:1,borderColor:'#A6B5A8',borderRadius:10,backgroundColor:'#FCFCF9'}}><Text>{isoDate(value)?value:emptyText??'请选择日期'}</Text></Pressable>:<TextInput accessibilityLabel={label} value={value} onChangeText={raw=>onChange(formatDigits(raw))} editable={!disabled} placeholder={emptyText??'例如：20261024'} keyboardType="number-pad" style={{padding:12,borderWidth:1,borderColor:'#A6B5A8',borderRadius:10,backgroundColor:'#FCFCF9'}}/>}
    {mode==='calendar'&&value&&!isoDate(value)&&<Text style={{fontSize:14,color:'#53665A'}}>数字填写尚未完成，切回数字输入可继续编辑。</Text>}
    {open&&<DetailOverlay title={'选择'+label} close={()=>setOpen(false)}>
      <View style={{flexDirection:'row',alignItems:'center',justifyContent:'space-between',gap:8}}>
        <Pressable accessibilityRole="button" accessibilityLabel="上个月" onPress={()=>setMonth(value=>shiftMonth(value,-1))} style={{padding:10,backgroundColor:'#E9F2E9',borderRadius:8}}><Text>上个月</Text></Pressable>
        <Text style={{fontSize:17}}>{month}</Text>
        <Pressable accessibilityRole="button" accessibilityLabel="下个月" onPress={()=>setMonth(value=>shiftMonth(value,1))} style={{padding:10,backgroundColor:'#E9F2E9',borderRadius:8}}><Text>下个月</Text></Pressable>
      </View>
      <View style={{flexDirection:'row',flexWrap:'wrap',gap:6}}>{monthDates(month).map(date=><Pressable key={date} accessibilityRole="button" accessibilityLabel={'选择日期：'+date} accessibilityState={{selected:value===date}} onPress={()=>{onChange(date);setOpen(false);}} style={{flexBasis:'22%',minWidth:44,minHeight:44,alignItems:'center',padding:8,borderRadius:8,backgroundColor:value===date?'#34765B':'#E9F2E9'}}><Text style={{color:value===date?'#FFFFFF':'#243D36'}}>{Number(date.slice(-2))}</Text></Pressable>)}</View>
      <Pressable accessibilityRole="button" onPress={()=>{onChange('');setOpen(false);}} style={{padding:10,backgroundColor:'#E9F2E9',borderRadius:8}}><Text>不设置日期</Text></Pressable>
    </DetailOverlay>}
    {help&&<><Pressable accessibilityRole="button" accessibilityLabel={`${helpOpen?'收起':'查看'}填写说明：${label}`} onPress={()=>setHelpOpen(value=>!value)} style={{paddingVertical:10,minHeight:44}}><Text style={{fontSize:14,color:'#34765B'}}>{helpOpen?'收起说明':'看看填写说明（可选）'}</Text></Pressable>{helpOpen&&<Text selectable style={{color:'#53665A'}}>填写说明 · {help}</Text>}</>}
  </View>;
}

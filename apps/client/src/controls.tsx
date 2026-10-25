import {DetailOverlay} from './overlay';
import { Pressable, Text, TextInput } from './readability';
import React, { useState } from 'react';
import { View } from 'react-native';
export type SelectProps = { label: string; value: string; options: { value: string; label: string }[]; onChange: (value: string) => void; disabled?: boolean };
export type DateProps = { label: string; value: string; onChange: (value: string) => void; disabled?: boolean };
export function Select({ label, value, options, onChange, disabled }: SelectProps) {
 const [open,setOpen]=useState(false),[search,setSearch]=useState('');
 const choices=(options.length<=6?options:options.filter(o=>o.value===value)).map(o=><Pressable key={o.value} accessibilityRole="radio" accessibilityLabel={o.label} accessibilityState={{checked:o.value===value,disabled}} disabled={disabled} onPress={()=>onChange(o.value)} style={{padding:10,backgroundColor:'#E9F2E9',borderRadius:8}}><Text>{o.value===value?'◉':'○'} {o.label}</Text></Pressable>);
 return <View style={{gap:8}}><Text>{label}</Text>{choices}{options.length>6&&<Pressable accessibilityRole="button" disabled={disabled} onPress={()=>{setSearch('');setOpen(true);}}><Text>更多{label}</Text></Pressable>}{open&&<DetailOverlay title={'选择'+label} close={()=>setOpen(false)}>{options.length>15&&<TextInput accessibilityLabel={'搜索'+label} value={search} onChangeText={setSearch} placeholder="输入名称或关键词"/>}{options.filter(o=>o.label.includes(search)).map(o=><Pressable key={o.value} accessibilityRole="button" onPress={()=>{onChange(o.value);setOpen(false);}}><Text>{o.label}</Text></Pressable>)}</DetailOverlay>}</View>;
}
export function DateField({ label, value, onChange, disabled }: DateProps) {
  return <View style={{ gap: 6 }}><Text>{label}</Text><TextInput accessibilityLabel={label} value={value} onChangeText={onChange} editable={!disabled} placeholder="YYYY-MM-DD" /></View>;
}

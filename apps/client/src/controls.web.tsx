import {DetailOverlay} from './overlay';
import {GuidedField} from './guided-field';
import React, { useState } from 'react';
import { View } from 'react-native';
import { Text, TextInput, Pressable, useReadability } from './readability';
import type { DateProps, SelectProps } from './controls';
const input: React.CSSProperties = { width: '100%', minWidth: 0, minHeight: 46, boxSizing: 'border-box', padding: '10px 12px', border: '1px solid #E0E6DD', borderRadius: 10, background: '#FCFCF9', color: '#243D36' };
export function Select({ label, value, options, onChange, disabled,compact=false }: SelectProps) {
  const { large } = useReadability();const [open,setOpen]=useState(false),[search,setSearch]=useState('');
  return <View style={{ gap: 6, marginTop: compact?0:8 }}>{!compact&&<Text style={{ fontSize: 17 }}>{label}</Text>}
    {options.length<=15?<select aria-label={label} value={value} disabled={disabled} onChange={event => onChange(event.target.value)} style={{ ...input, fontSize: large ? 18 : 16, minHeight: large ? 56 : 46 }}>{options.map(option => <option key={option.value} value={option.value}>{option.label}</option>)}</select>:<Pressable accessibilityRole="combobox" accessibilityLabel={label} accessibilityState={{expanded:open,disabled}} disabled={disabled} onPress={()=>{setSearch('');setOpen(true);}} style={{padding:12,borderWidth:1,borderColor:'#A6B5A8',borderRadius:10,backgroundColor:'#FCFCF9'}}><Text>{options.find(o=>o.value===value)?.label??'请选择'} ▾</Text></Pressable>}
    {open&&<DetailOverlay title={'选择'+label} close={()=>setOpen(false)}><GuidedField label={'搜索'+label} value={search} change={setSearch} multiline={false}/>{options.filter(o=>o.label.includes(search)).map(o=><Pressable key={o.value} accessibilityRole="button" accessibilityLabel={'选择：'+o.label} onPress={()=>{onChange(o.value);setOpen(false);}} style={{padding:12,backgroundColor:'#E9F2E9',borderRadius:8}}><Text>{o.label}{o.value===value?'（当前选择）':''}</Text></Pressable>)}{!options.some(o=>o.label.includes(search))&&<Text>没有匹配项，原选择仍保留。</Text>}</DetailOverlay>}
  </View>;
}
export function DateField({ label, value, onChange, disabled }: DateProps) {
  const { large } = useReadability();
  const [mode, setMode] = useState<'calendar' | 'digits'>(() => value && !/^\d{4}-\d{2}-\d{2}$/.test(value) ? 'digits' : 'calendar');
  return <View style={{ gap: 6, marginTop: 8 }}>
    <View style={{ flexDirection: 'row', alignItems: 'center', flexWrap: 'wrap', gap: 8 }}>
      <Text style={{ fontSize: 17, flexShrink: 1 }}>{label}</Text>
      <Pressable accessibilityRole="button" accessibilityLabel={`${label}：${mode === 'calendar' ? '改用数字输入' : '改用日历选择'}`}
        disabled={disabled} onPress={() => setMode(mode === 'calendar' ? 'digits' : 'calendar')} style={{ padding: 8, borderRadius: 8, backgroundColor: '#E9F2E9' }}>
        <Text style={{ fontSize: 14 }}>{mode === 'calendar' ? '数字输入' : '日历选择'}</Text>
      </Pressable>
    </View>
    {mode === 'calendar' ? <input aria-label={`${label}：选择日期`} type="date" value={/^\d{4}-\d{2}-\d{2}$/.test(value) ? value : ''}
      disabled={disabled} onChange={event => onChange(event.target.value)} style={{ ...input, fontSize: large ? 18 : 16, minHeight: large ? 56 : 46 }} /> :
      <TextInput accessibilityLabel={label} value={value} onChangeText={raw => onChange(/^\d{8}$/.test(raw) ? `${raw.slice(0, 4)}-${raw.slice(4, 6)}-${raw.slice(6)}` : raw)}
        editable={!disabled} placeholder="例如：20261024" inputMode="numeric"
        style={{ borderWidth: 1, borderColor: '#E0E6DD', padding: 12, borderRadius: 10, backgroundColor: '#FCFCF9' }} />}
    {mode === 'calendar' && value && !/^\d{4}-\d{2}-\d{2}$/.test(value) ? <Text style={{ fontSize: 14 }}>数字填写尚未完成，切回数字输入可继续编辑。</Text> : null}
  </View>;
}

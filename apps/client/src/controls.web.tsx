import React from 'react';
import { Text, TextInput, View } from 'react-native';
import type { DateProps, SelectProps } from './controls';
const input: React.CSSProperties = { width: '100%', minWidth: 0, minHeight: 46, boxSizing: 'border-box', padding: '10px 12px', border: '1px solid #E0E6DD', borderRadius: 10, background: '#FCFCF9', color: '#243D36', fontSize: 14 };
export function Select({ label, value, options, onChange, disabled }: SelectProps) {
  return <View style={{ gap: 6, marginTop: 8 }}><Text style={{ color: '#243D36', fontSize: 13 }}>{label}</Text>
    <select aria-label={label} value={value} disabled={disabled} onChange={event => onChange(event.target.value)} style={input}>
      {options.map(option => <option key={option.value} value={option.value}>{option.label}</option>)}
    </select></View>;
}
export function DateField({ label, value, onChange, disabled }: DateProps) {
  return <View style={{ gap: 6, marginTop: 8 }}><Text style={{ color: '#243D36', fontSize: 13 }}>{label}</Text>
    <input aria-label={`${label}：选择日期`} type="date" value={/^\d{4}-\d{2}-\d{2}$/.test(value) ? value : ''} disabled={disabled}
      onChange={event => onChange(event.target.value)} style={input} />
    <TextInput accessibilityLabel={label} value={value} onChangeText={onChange} editable={!disabled} placeholder="也可输入数字：YYYY-MM-DD"
      style={{ borderWidth: 1, borderColor: '#E0E6DD', padding: 12, borderRadius: 10, minHeight: 45, color: '#243D36', backgroundColor: '#FCFCF9', fontSize: 14 }} />
  </View>;
}

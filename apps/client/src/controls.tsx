import { Pressable, Text, TextInput } from './readability';
import React, { useState } from 'react';
import { View } from 'react-native';
export type SelectProps = { label: string; value: string; options: { value: string; label: string }[]; onChange: (value: string) => void; disabled?: boolean };
export type DateProps = { label: string; value: string; onChange: (value: string) => void; disabled?: boolean };
export function Select({ label, value, options, onChange, disabled }: SelectProps) {
  const [open, setOpen] = useState(false);
  return <View style={{ gap: 8 }}><Text>{label}</Text><Pressable accessibilityRole="button" disabled={disabled} onPress={() => setOpen(!open)}><Text>{options.find(option => option.value === value)?.label ?? '请选择'} ▾</Text></Pressable>
    {open && options.map(option => <Pressable key={option.value} accessibilityRole="button" disabled={disabled} onPress={() => { onChange(option.value); setOpen(false); }}><Text>{option.label}</Text></Pressable>)}</View>;
}
export function DateField({ label, value, onChange, disabled }: DateProps) {
  return <View style={{ gap: 6 }}><Text>{label}</Text><TextInput accessibilityLabel={label} value={value} onChangeText={onChange} editable={!disabled} placeholder="YYYY-MM-DD" /></View>;
}

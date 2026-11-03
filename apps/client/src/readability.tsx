import React, { createContext, forwardRef, useContext, useEffect, useState } from 'react';
import { Pressable as NativePressable, StyleSheet, Text as NativeText, TextInput as NativeTextInput, View,
  type PressableProps, type TextProps, type TextInputProps, type TextStyle } from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import {neutral} from './ui-palette';
import {useUiPalette} from './ui-theme';

const Context = createContext({ large: false, ready: false, error: '', change: (_large: boolean) => {} });
const key = 'skarmar:device:large-text';
export function ReadabilityProvider({ children }: { children: React.ReactNode }) {
  const [large, setLarge] = useState(false);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    let active = true;
    AsyncStorage.getItem(key).then(value => { if (active) setLarge(value === '1'); })
      .catch(() => { if (active) setError('无法读取本机字号设置，可以重新选择。'); })
      .finally(() => { if (active) setReady(true); });
    return () => { active = false; };
  }, []);
  const change = (next: boolean) => {
    setLarge(next); setError('');
    void AsyncStorage.setItem(key, next ? '1' : '0').catch(() => setError('字号已调整，但未能保存到本机，重新打开后请再次选择。'));
  };
  return <Context.Provider value={{ large, ready, error, change }}>{children}</Context.Provider>;
}
export const useReadability = () => useContext(Context);
function typography(style: TextProps['style'], large: boolean, input = false): TextStyle {
  const flat = StyleSheet.flatten(style) ?? {};
  const base = typeof flat.fontSize === 'number' ? flat.fontSize : 16;
  const fontSize = Math.max(input || base >= 14 ? (large ? 18 : 16) : (large ? 16 : 14), base);
  return { fontSize, lineHeight: Math.max(flat.lineHeight ?? 0, Math.ceil(fontSize * 1.45)) };
}
export const Text = forwardRef<NativeText, TextProps>(function Text(props, ref) {
  const { large } = useReadability();
  return <NativeText {...props} ref={ref} style={[{ color: neutral.ink }, props.style, typography(props.style, large)]} />;
});
export const TextInput = forwardRef<NativeTextInput, TextInputProps>(function TextInput(props, ref) {
  const { large } = useReadability();
  const c=useUiPalette(),[focused,setFocused]=useState(false);
  return <NativeTextInput placeholderTextColor="#58695E" {...props} ref={ref}
    onFocus={e=>{setFocused(true);props.onFocus?.(e);}} onBlur={e=>{setFocused(false);props.onBlur?.(e);}}
    style={[{ color: neutral.ink, minHeight: large ? 56 : 44 }, props.style, typography(props.style, large, true),focused&&{borderColor:c.ink,borderWidth:2}]} />;
});
export function Pressable({ style, ...props }: PressableProps) {
  const { large } = useReadability();
  return <NativePressable {...props} style={state => {const value=typeof style==='function'?style(state):style,flat=StyleSheet.flatten(value)??{};return [value,
    { minWidth: typeof flat.minWidth==='number'?Math.max(44,flat.minWidth):flat.minWidth??44, minHeight: typeof flat.minHeight==='number'?Math.max(large?56:44,flat.minHeight):flat.minHeight??(large?56:44), justifyContent: 'center' }];}} />;
}
export function ReadabilitySettings() {
  const { large, ready, error, change } = useReadability();
  return <View style={{ gap: 10, padding: 16, backgroundColor: '#FFFFFF', borderRadius: 14 }}>
    <Text accessibilityRole="header" style={{ fontSize: 18, fontWeight: '600' }}>本机显示</Text>
    <Pressable accessibilityRole="switch" accessibilityLabel="大字模式" aria-checked={large} accessibilityState={{ checked: large, disabled: !ready }}
      disabled={!ready} onPress={() => change(!large)} style={{ padding: 12, backgroundColor: '#E9F2E9', borderRadius: 10 }}>
      <Text style={{ fontSize: 17 }}>大字模式：{large ? '已开启' : '未开启'}</Text>
    </Pressable>
    <Text style={{ fontSize: 14, color: '#53665A' }}>只保存在这台设备。也可以使用浏览器的缩放功能。</Text>
    {error ? <Text accessibilityRole="alert">{error}</Text> : null}
  </View>;
}

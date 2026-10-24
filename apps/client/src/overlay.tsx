import React, { useEffect, useId, useRef } from 'react';
import { Modal, Platform, ScrollView, View } from 'react-native';
import { Pressable, Text } from './readability';

/** An explicitly opened detail; closing never unmounts the underlying form. */
export function DetailOverlay({ title, close, children }: { title: string; close: () => void; children: React.ReactNode }) {
  const id = useId();
  const closeRef = useRef(close);
  closeRef.current = close;
  useEffect(() => {
    if (Platform.OS !== 'web') return;
    const marker = `skarmar:${id}`;
    // Preserve the host's history state; this sentinel only belongs to this overlay.
    window.history.pushState({ ...window.history.state, skarmarOverlay: marker }, '');
    const back = () => { if (window.history.state?.skarmarOverlay !== marker) closeRef.current(); };
    window.addEventListener('popstate', back);
    return () => {
      window.removeEventListener('popstate', back);
      if (window.history.state?.skarmarOverlay === marker) window.history.back();
    };
  }, [id]);
  return <Modal visible transparent accessibilityLabel={title} animationType="none" onRequestClose={() => closeRef.current()}>
    <View style={{ flex: 1, justifyContent: 'center', alignItems: 'center', backgroundColor: '#0008', padding: 12 }}>
      <View accessibilityViewIsModal
        style={{ width: '100%', maxWidth: 700, maxHeight: '92%', backgroundColor: '#F6F5EF', borderRadius: 16, padding: 16, gap: 12 }}>
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 12 }}>
          <Text accessibilityRole="header" style={{ flex: 1, fontSize: 20, fontWeight: '600' }}>{title}</Text>
          <Pressable accessibilityRole="button" accessibilityLabel={`关闭${title}`} onPress={close} style={{ padding: 10, backgroundColor: '#E9F2E9', borderRadius: 10 }}>
            <Text style={{ fontSize: 17 }}>关闭</Text>
          </Pressable>
        </View>
        <ScrollView keyboardShouldPersistTaps="handled" contentContainerStyle={{ gap: 12, paddingBottom: 12 }}>{children}</ScrollView>
      </View>
    </View>
  </Modal>;
}

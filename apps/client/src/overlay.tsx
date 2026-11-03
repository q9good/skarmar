import {SessionRenewalContext} from './session-renewal';
import React, { useContext, useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { KeyboardAvoidingView, Modal, Platform, ScrollView, View } from 'react-native';
import { Pressable, Text } from './readability';
import {neutral,overlayColors,type UiTone} from './ui-palette';
import {UiTheme,useUiTone} from './ui-theme';

// Nested pickers and replacement details share one browser-history sentinel stack.
// A new layer waits for a closing layer's asynchronous Back navigation to finish.
type Layer = { marker: string; pushed: boolean; close: () => void; visibility: (front: boolean) => void };
const layers: Layer[] = [];
function updateVisibility() { layers.forEach((layer,index)=>layer.visibility(index===layers.length-1)); }
let installed = false;
let draining: Promise<void> | null = null;
let finishDrain: (() => void) | undefined;
function marker() { const value = window.history.state?.skarmarOverlay; return typeof value === 'string' && value.startsWith('skarmar:') ? value : undefined; }
function prune(): Promise<void> {
  if (draining) return draining;
  const current = marker();
  if (!current || layers.some(layer => layer.pushed && layer.marker === current)) return Promise.resolve();
  draining = new Promise(resolve => { finishDrain = resolve; });
  window.history.back();
  return draining;
}
function pop() {
  const current = marker();
  if (draining) {
    if (current && !layers.some(layer => layer.pushed && layer.marker === current)) { window.history.back(); return; }
    const finish = finishDrain; draining = null; finishDrain = undefined; finish?.();
  } else if (current && !layers.some(layer => layer.pushed && layer.marker === current)) { void prune(); return; }
  const index = layers.findIndex(layer => layer.pushed && layer.marker === current);
  for (const layer of [...layers].reverse()) if (layer.pushed && layers.indexOf(layer) > index) layer.close();
}

/** An explicitly opened detail; closing never unmounts the underlying form. */
export function DetailOverlay({ title, close, children, footer, colors, tone }: { title: string; close: () => void; children: React.ReactNode;footer?:React.ReactNode;colors?:{background:string;control:string;ink:string};tone?:UiTone }) {
  const inherited=useUiTone();
  const theme=colors??overlayColors(tone??inherited);
  const renewal=useContext(SessionRenewalContext);
  const [viewport,setViewport]=useState<number|null>(null);
  useEffect(()=>{
    if(Platform.OS!=='web'||!window.visualViewport)return;
    const view=window.visualViewport,update=()=>setViewport(view.height);update();view.addEventListener('resize',update);return()=>view.removeEventListener('resize',update);
  },[]);
  const compact=viewport!==null&&viewport<480;
  const id = useId();
  const frame = useRef<View>(null);
  const [front, setFront] = useState(true);
  const [historyReady,setHistoryReady]=useState(Platform.OS!=='web');
  const closeRef = useRef(close);
  closeRef.current = close;
  useEffect(() => {
    if (Platform.OS !== 'web') return;
    const dialog = (frame.current as unknown as HTMLElement|null)?.closest('[role="dialog"]');
    dialog?.setAttribute('aria-hidden', String(!front));
  }, [front]);
  useLayoutEffect(() => {
    if (Platform.OS !== 'web') return;
    if (!installed) { window.addEventListener('popstate', pop); installed = true; }
    const layer: Layer = { marker: `skarmar:${id}`, pushed: false, close: () => closeRef.current(), visibility: setFront };
    layers.push(layer);
    updateVisibility();
    let cancelled = false;
    const activate=()=>{
      if(cancelled)return;
      window.history.pushState({ ...window.history.state, skarmarOverlay: layer.marker }, '');
      layer.pushed = true;
      setHistoryReady(true);
    };
    const current=marker(),known=Boolean(current&&layers.some(item=>item.pushed&&item.marker===current));
    if(!draining&&(!current||known))activate();
    else void prune().then(activate);
    return () => {
      cancelled = true;
      const index = layers.indexOf(layer); if (index >= 0) layers.splice(index, 1);
      updateVisibility();
      if (layer.pushed) void prune();
    };
  }, [id]);
  return <UiTheme tone={tone??inherited}><Modal visible={historyReady} transparent accessibilityLabel={title} animationType="none" onRequestClose={() => closeRef.current()}>
    <KeyboardAvoidingView behavior={Platform.OS==='ios'?'padding':undefined} style={{flex:1}}><View ref={frame} aria-hidden={!front} pointerEvents={front ? 'auto' : 'none'} style={{ flex: 1, justifyContent: 'center', alignItems: 'center', backgroundColor: '#0008', padding: 12,...(viewport?{height:viewport,maxHeight:viewport,flexGrow:0,flexShrink:0,flexBasis:'auto'}:{}) }}>
      <View accessibilityViewIsModal
        style={{ width: '100%', maxWidth: 700, height:footer?(viewport?Math.floor(viewport*.92):'92%'):undefined,maxHeight: '92%', backgroundColor: theme.background, borderRadius: 18, padding: compact?12:16, gap: compact?8:14,borderWidth:1,borderColor:neutral.border }}>
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 12,flexShrink:0,paddingBottom:compact?6:10,borderBottomWidth:1,borderColor:neutral.border }}>
          <Text accessibilityRole="header" style={{ flex: 1, fontSize: 20, fontWeight: '700',color:theme.ink }}>{title}</Text>
          <Pressable accessibilityRole="button" accessibilityLabel={`关闭${title}`} onPress={close} style={{ padding: 10, backgroundColor: theme.control, borderRadius: 10,borderWidth:1,borderColor:neutral.border }}>
            <Text style={{ fontSize: 17,color:theme.ink,fontWeight:'600' }}>关闭</Text>
          </Pressable>
        </View>
        {renewal?.expiresSoon&&title!=='续期登录'&&<Pressable accessibilityRole="button" onPress={renewal.renew} style={{padding:10,backgroundColor:'#FFF5DE',borderRadius:8,flexShrink:0}}><Text>登录即将到期，先验证登录再继续填写</Text></Pressable>}
        <ScrollView keyboardShouldPersistTaps="handled" automaticallyAdjustKeyboardInsets style={{flex:footer?1:undefined,flexShrink:1,minHeight:0}} contentContainerStyle={{ gap: 12, paddingBottom: 16 }}>{children}</ScrollView>
        {footer&&<View style={{flexShrink:0,gap:8,paddingTop:8,borderTopWidth:1,borderColor:'#CED8CE',...(Platform.OS==='web'?{paddingBottom:'max(4px, env(safe-area-inset-bottom))' as unknown as number}:{paddingBottom:8})}}>{footer}</View>}
      </View>
    </View></KeyboardAvoidingView>
  </Modal></UiTheme>;
}

import React from 'react';
import {View} from 'react-native';
import {Pressable,Text} from './readability';

// Muted accents share the existing grey-green canvas. Labels and selected borders
// carry the distinction as well, so colour is never the only signal.
export const sectionTones = {
  classroom: {ink:'#315C78', pale:'#E8EFF5'},
  family: {ink:'#805039', pale:'#F4EAE3'},
  tasks: {ink:'#56516E', pale:'#EEEAF4'},
  experience: {ink:'#30674F', pale:'#E8F1E8'},
  difficulty: {ink:'#765519', pale:'#F4EEDB'},
};
export type SectionTone = keyof typeof sectionTones;
export function SectionButton({title,tone,selected=false,primary=false,disabled=false,press}:{title:string;tone:SectionTone;selected?:boolean;primary?:boolean;disabled?:boolean;press:()=>void}) {
  const c=sectionTones[tone],filled=selected||primary;
  return <Pressable accessibilityRole="button" accessibilityLabel={title} accessibilityState={{selected,disabled}} disabled={disabled} onPress={press}
    style={{padding:12,minHeight:46,borderRadius:10,borderWidth:2,borderColor:filled?c.ink:c.pale,backgroundColor:filled?c.ink:c.pale,opacity:disabled?.5:1}}>
    <Text style={{fontSize:17,fontWeight:filled?'600':'400',color:filled?'#FFFFFF':c.ink}}>{title}</Text>
  </Pressable>;
}
export function ContentGroup({title,tone='experience',children}:{title:string;tone?:SectionTone;children:React.ReactNode}) {
  const c=sectionTones[tone];
  return <View style={{gap:10,padding:12,borderRadius:10,borderLeftWidth:4,borderColor:c.ink,backgroundColor:c.pale}}>
    <Text accessibilityRole="header" style={{fontSize:18,fontWeight:'600',color:c.ink}}>{title}</Text>{children}
  </View>;
}

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
export function SectionButton({title,tone,primary=false,disabled=false,press}:{title:string;tone:SectionTone;primary?:boolean;disabled?:boolean;press:()=>void}) {
  const c=sectionTones[tone];
  return <Pressable accessibilityRole="button" accessibilityLabel={title} accessibilityState={{disabled}} disabled={disabled} onPress={press}
    style={{padding:12,minHeight:primary?56:46,justifyContent:'center',borderRadius:10,borderWidth:1,borderColor:primary?'#BCC9BD':c.pale,borderLeftWidth:primary?4:1,borderLeftColor:primary?c.ink:c.pale,backgroundColor:primary?'#FFFFFF':c.pale,opacity:disabled?.5:1}}>
    <Text style={{fontSize:17,fontWeight:primary?'600':'400',color:primary?'#243D36':c.ink}}>{title}</Text>
  </Pressable>;
}

/** A single bounded control makes sibling choices explicit. Both choices keep
 * the same surface, typography and size; selection uses one common underline. */
export function SectionTabs<T extends string>({label,items,value,change}:{label:string;items:{value:T;title:string;tone:SectionTone}[];value:T;change:(v:T)=>void}) {
  return <View role="group" accessibilityLabel={label} style={{flexDirection:'row',alignItems:'stretch',padding:4,gap:4,borderWidth:1,borderColor:'#BCC9BD',borderRadius:12,backgroundColor:'#E9EDE7'}}>
    {items.map(item=>{const selected=item.value===value;return <Pressable key={item.value} accessibilityRole="button" accessibilityLabel={item.title} accessibilityState={{selected}} aria-pressed={selected} onPress={()=>change(item.value)}
      style={{flex:1,minWidth:0,minHeight:48,paddingVertical:10,paddingHorizontal:8,borderRadius:8,borderBottomWidth:3,borderBottomColor:selected?'#34765B':'transparent',backgroundColor:'#FFFFFF',flexDirection:'row',alignItems:'center',justifyContent:'center',gap:7}}>
      <View aria-hidden style={{width:6,height:6,borderRadius:3,backgroundColor:sectionTones[item.tone].ink}}/>
      <Text style={{fontSize:17,fontWeight:'600',color:'#243D36',flexShrink:1,textAlign:'center'}}>{item.title}</Text>
    </Pressable>;})}
  </View>;
}
export function ContentGroup({title,tone='experience',children}:{title:string;tone?:SectionTone;children:React.ReactNode}) {
  const c=sectionTones[tone];
  return <View style={{gap:10,padding:12,borderRadius:10,borderLeftWidth:4,borderColor:c.ink,backgroundColor:c.pale}}>
    <Text accessibilityRole="header" style={{fontSize:18,fontWeight:'600',color:c.ink}}>{title}</Text>{children}
  </View>;
}

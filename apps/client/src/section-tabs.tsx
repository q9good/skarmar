import React from 'react';
import {View} from 'react-native';
import {Pressable,Text} from './readability';
import {UiButton} from './ui-components';
import {neutral,palettes} from './ui-palette';

// Muted accents share the existing grey-green canvas. Labels and selected borders
// carry the distinction as well, so colour is never the only signal.
export const sectionTones = {
  goal: {ink:palettes.goal.ink, pale:palettes.goal.soft},
  method: {ink:palettes.method.ink, pale:palettes.method.soft},
  classroom: {ink:palettes.classroom.ink, pale:palettes.classroom.soft},
  family: {ink:palettes.family.ink, pale:palettes.family.soft},
  tasks: {ink:palettes.tasks.ink, pale:palettes.tasks.soft},
  experience: {ink:palettes.experience.ink, pale:palettes.experience.soft},
  difficulty: {ink:palettes.difficulty.ink, pale:palettes.difficulty.soft},
};
export type SectionTone = keyof typeof sectionTones;
export const knowledgeTones = {
  experience: {...sectionTones.experience,page:palettes.experience.page,card:palettes.experience.soft,action:palettes.experience.action,border:palettes.experience.border},
  difficulty: {...sectionTones.difficulty,page:palettes.difficulty.page,card:palettes.difficulty.soft,action:palettes.difficulty.action,border:palettes.difficulty.border},
};
export function SectionButton({title,tone,primary=false,disabled=false,press}:{title:string;tone:SectionTone;primary?:boolean;disabled?:boolean;press:()=>void}) {
  return <UiButton title={title} tone={tone} variant={primary?'primary':'secondary'} chevron={primary} disabled={disabled} press={press} style={primary?{minHeight:60,paddingVertical:17}:undefined}/>;
}

/** A single bounded control makes sibling choices explicit. Both choices keep
 * the same surface, typography and size; selection uses one common underline. */
export function SectionTabs<T extends string>({label,items,value,change,categoryColors=false}:{label:string;items:{value:T;title:string;tone:SectionTone}[];value:T;change:(v:T)=>void;categoryColors?:boolean}) {
  return <View role="group" accessibilityLabel={label} style={{flexDirection:'row',alignItems:'stretch',padding:4,gap:4,borderWidth:1,borderColor:neutral.border,borderRadius:12,backgroundColor:neutral.canvas}}>
    {items.map(item=>{const selected=item.value===value;return <Pressable key={item.value} accessibilityRole="button" accessibilityLabel={item.title} accessibilityState={{selected}} aria-pressed={selected} onPress={()=>change(item.value)}
      style={{flex:1,minWidth:0,minHeight:48,paddingVertical:10,paddingHorizontal:8,borderRadius:8,borderBottomWidth:3,borderBottomColor:selected?sectionTones[item.tone].ink:'transparent',backgroundColor:categoryColors?sectionTones[item.tone].pale:'#FFFFFF',flexDirection:'row',alignItems:'center',justifyContent:'center',gap:7}}>
      <View aria-hidden style={{width:6,height:6,borderRadius:3,backgroundColor:sectionTones[item.tone].ink}}/>
      <Text style={{fontSize:17,fontWeight:'600',color:categoryColors?sectionTones[item.tone].ink:neutral.ink,flexShrink:1,textAlign:'center'}}>{item.title}</Text>
    </Pressable>;})}
  </View>;
}
export function ContentGroup({title,tone='experience',children}:{title:string;tone?:SectionTone;children:React.ReactNode}) {
  const c=sectionTones[tone];
  return <View style={{gap:10,padding:14,borderRadius:12,borderWidth:1,borderColor:palettes[tone].border,backgroundColor:c.pale}}>
    <Text accessibilityRole="header" style={{fontSize:18,fontWeight:'600',color:c.ink}}>{title}</Text>{children}
  </View>;
}

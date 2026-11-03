import React,{useState} from 'react';
import {View,type StyleProp,type ViewStyle} from 'react-native';
import {Pressable,Text} from './readability';
import {neutral,palettes,type UiTone} from './ui-palette';
import {UiTheme,useUiTone} from './ui-theme';

export function UiButton({title,label,press,disabled=false,variant='secondary',tone,chevron=false,style}:{title:string;label?:string;press:()=>void;disabled?:boolean;variant?:'primary'|'secondary'|'link';tone?:UiTone;chevron?:boolean;style?:StyleProp<ViewStyle>}){
 const inherited=useUiTone(),c=palettes[tone??inherited],[focused,setFocused]=useState(false),[hovered,setHovered]=useState(false),primary=variant==='primary',link=variant==='link';
 return <Pressable accessibilityRole="button" accessibilityLabel={title} accessibilityState={{disabled}} disabled={disabled} onPress={press} onFocus={()=>setFocused(true)} onBlur={()=>setFocused(false)} onHoverIn={()=>setHovered(true)} onHoverOut={()=>setHovered(false)}
  style={({pressed})=>[{paddingVertical:primary?14:10,paddingHorizontal:link?0:14,minHeight:primary?52:46,borderRadius:12,borderWidth:link?0:1.5,borderColor:disabled?neutral.border:focused?c.ink:primary?c.ink:c.border,
   backgroundColor:disabled?neutral.disabled:primary?pressed||hovered?c.hover:c.ink:link?'transparent':pressed||hovered?c.soft:neutral.surface,
   ...(primary&&!disabled?{shadowColor:c.ink,shadowOffset:{width:0,height:3},shadowOpacity:.13,shadowRadius:6,elevation:2}:{}),
   ...(focused?{borderWidth:2}:{}),...(pressed&&!disabled?{transform:[{scale:.992}]}:{})},style]}>
  <Text style={{fontSize:17,fontWeight:primary?'700':'600',color:disabled?neutral.disabledInk:primary?'#FFFFFF':c.ink,textAlign:link?'left':'center',...(link?{textDecorationLine:'underline'}:{}),...(chevron?{paddingHorizontal:24}:{})}}>{label??title}</Text>
  {chevron&&<View aria-hidden pointerEvents="none" style={{position:'absolute',right:14,top:0,bottom:0,justifyContent:'center'}}><Text style={{fontSize:23,fontWeight:'600',color:primary?'#FFFFFF':c.ink}}>›</Text></View>}
 </Pressable>;
}
export function UiSection({title,description,required=false,children,tone,testID}:{title:string;description?:string;required?:boolean;children:React.ReactNode;tone?:UiTone;testID?:string}){
 const inherited=useUiTone(),c=palettes[tone??inherited];
 return <UiTheme tone={tone??inherited}><View testID={testID} style={{gap:12,padding:16,borderWidth:1,borderColor:c.border,borderRadius:16,backgroundColor:neutral.surface}}>
  <View style={{flexDirection:'row',alignItems:'center',gap:10}}><Text accessibilityRole="header" style={{flex:1,fontSize:20,fontWeight:'700',color:c.ink}}>{title}</Text>{required&&<Text style={{fontSize:14,fontWeight:'600',color:c.ink,backgroundColor:c.soft,paddingHorizontal:8,paddingVertical:3,borderRadius:5}}>必填</Text>}</View>
  {description&&<Text style={{fontSize:14,color:neutral.muted}}>{description}</Text>}{children}
 </View></UiTheme>;
}
export function UiCheck({title,checked,press,disabled=false,tone}:{title:string;checked:boolean;press:()=>void;disabled?:boolean;tone?:UiTone}){
 const inherited=useUiTone(),c=palettes[tone??inherited];
 return <Pressable accessibilityRole="checkbox" accessibilityLabel={title} accessibilityState={{checked,disabled}} aria-checked={checked} disabled={disabled} onPress={press} style={{padding:12,borderRadius:10,borderWidth:1,borderColor:checked?c.ink:c.border,backgroundColor:checked?c.soft:neutral.surface,opacity:disabled?.55:1}}><Text style={{color:checked?c.ink:neutral.ink,fontWeight:checked?'600':'400'}}>{checked?'☑':'☐'} {title}</Text></Pressable>;
}

import React,{useId,useState} from 'react';
import {View} from 'react-native';
import {Pressable,Text} from './readability';

/** Keep children mounted so collapsing never clears input or a nested picker. */
export function Disclosure({title,children,summary,defaultOpen=false}:{title:string;children:React.ReactNode;summary?:string;defaultOpen?:boolean}){
 const [open,setOpen]=useState(defaultOpen),id=useId();
 return <View style={{gap:8}}>
  <Pressable accessibilityRole="button" accessibilityLabel={title} accessibilityState={{expanded:open}} aria-controls={id} onPress={()=>setOpen(v=>!v)} style={{minHeight:44,paddingVertical:8,flexDirection:'row',alignItems:'center',gap:8}}>
   <Text style={{flex:1,color:'#34765B'}}>{title}{summary?` · ${summary}`:''}</Text><Text>{open?'▴':'▾'}</Text>
  </Pressable>
  <View nativeID={id} style={{display:open?'flex':'none',gap:12}}>{children}</View>
 </View>;
}

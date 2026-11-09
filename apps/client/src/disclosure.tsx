import React,{useId,useState} from 'react';
import {View} from 'react-native';
import {Pressable,Text} from './readability';
import {useUiPalette} from './ui-theme';

/** Keep children mounted so collapsing never clears input or a nested picker. */
export function Disclosure({title,label,children,summary,defaultOpen=false,color}:{title:string;label?:string;children:React.ReactNode;summary?:string;defaultOpen?:boolean;color?:string}){
 const [open,setOpen]=useState(defaultOpen),id=useId(),theme=useUiPalette();
 return <View style={{gap:8}}>
  <Pressable accessibilityRole="button" accessibilityLabel={title} accessibilityState={{expanded:open}} aria-controls={id} onPress={()=>setOpen(v=>!v)} style={{minHeight:44,paddingVertical:8,flexDirection:'row',alignItems:'center',gap:8}}>
   <Text style={{flex:1,color:color??theme.ink}}>{label??title}{summary?` · ${summary}`:''}</Text><Text>{open?'▴':'▾'}</Text>
  </Pressable>
  <View nativeID={id} style={{display:open?'flex':'none',gap:12}}>{children}</View>
 </View>;
}

/** Optional recording step: closing hides fields without deleting entered data. */
export function OptionalSection({title,children,defaultOpen=false}:{title:string;children:React.ReactNode;defaultOpen?:boolean}) {
 const [open,setOpen]=useState(defaultOpen),id=useId(),theme=useUiPalette();
 return <View style={{gap:10}}>
  <Pressable accessibilityRole="checkbox" accessibilityLabel={title} accessibilityState={{checked:open}} aria-checked={open} aria-controls={id} onPress={()=>setOpen(v=>!v)} style={{padding:12,minHeight:46,borderRadius:10,backgroundColor:theme.soft,borderWidth:1,borderColor:theme.border}}>
   <Text>{open?'☑':'☐'} {title}</Text>
  </Pressable>
  <View nativeID={id} style={{display:open?'flex':'none',gap:12}}>{children}</View>
 </View>;
}

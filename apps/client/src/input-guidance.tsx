import {neutral,palettes} from './ui-palette';
import React,{useState} from 'react';
import {View} from 'react-native';
import {Pressable,Text} from './readability';
/** Examples remain readable after typing and never become a field value. */
export function ExampleHint({label,example}:{label:string;example?:string}){
 const [open,setOpen]=useState(false);if(!example||!example.startsWith('例如'))return null;
 return <View style={{gap:5}}><Pressable accessibilityRole="button" accessibilityLabel={`${open?'收起':'查看'}填写示例：${label}`} onPress={()=>setOpen(v=>!v)} style={{paddingVertical:10,minHeight:44}}><Text style={{fontSize:14,color:palettes.home.ink}}>{open?'收起示例':'看看怎么写（可选）'}</Text></Pressable>{open&&<Text selectable style={{color:neutral.muted}}>合成示例 · {example}</Text>}</View>;
}

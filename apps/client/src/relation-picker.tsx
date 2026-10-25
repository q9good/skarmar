import React,{useState} from 'react';
import {View} from 'react-native';
import {Text,Pressable} from './readability';
import {DetailOverlay} from './overlay';
import {GuidedField} from './guided-field';
export type RelationOption={id:string;label:string;hint?:string;disabled?:boolean;related?:boolean};
export function RelationPicker({label,options,selected,toggle,disabled=false,prefix='',buttonMode=false,selectedPrefix=prefix}:{label:string;options:RelationOption[];selected:string[];toggle:(id:string)=>void;disabled?:boolean;prefix?:string;buttonMode?:boolean;selectedPrefix?:string}){
 const [expanded,setExpanded]=useState(false),[search,setSearch]=useState(''),[panel,setPanel]=useState(false);
 const check=(o:RelationOption)=><Pressable key={o.id} accessibilityRole={buttonMode?'button':'checkbox'} accessibilityLabel={(selected.includes(o.id)?selectedPrefix:prefix)+o.label} accessibilityState={{checked:selected.includes(o.id),disabled:disabled||o.disabled}} aria-checked={selected.includes(o.id)} disabled={disabled||o.disabled} onPress={()=>toggle(o.id)} style={{padding:10,gap:4,borderRadius:8,backgroundColor:'#E9F2E9',opacity:(disabled||o.disabled)?0.5:1}}><Text>{selected.includes(o.id)?'☑':'☐'} {o.label}</Text>{o.hint&&<Text style={{fontSize:14}}>{o.hint}</Text>}</Pressable>;
 const first=options.filter(o=>selected.includes(o.id)||o.related),shown=options.length<=6||expanded?options:[...first,...options.filter(o=>!first.includes(o)).slice(0,Math.max(0,6-first.length))];
 return <View style={{gap:8}}>
  {options.length<=15?shown.map(check):options.filter(o=>selected.includes(o.id)).map(check)}
  {options.length>6&&options.length<=15&&(expanded||shown.length<options.length)&&<Pressable accessibilityRole="button" onPress={()=>setExpanded(v=>!v)} disabled={disabled} style={{padding:10,backgroundColor:'#F0F3ED',borderRadius:8}}><Text>{expanded?`收起${label}`:`更多${label}（${options.length-shown.length}）`}</Text></Pressable>}
  {options.length>15&&<Pressable accessibilityRole="button" onPress={()=>{setSearch('');setPanel(true);}} disabled={disabled} style={{padding:10,backgroundColor:'#F0F3ED',borderRadius:8}}><Text>搜索选择{label}（已选{selected.length}）</Text></Pressable>}
  {panel&&<DetailOverlay title={`选择${label}`} close={()=>setPanel(false)}><GuidedField label={`搜索${label}`} value={search} change={setSearch} multiline={false}/>{options.filter(o=>o.label.includes(search)||o.hint?.includes(search)).map(check)}{!options.some(o=>o.label.includes(search)||o.hint?.includes(search))&&<Text>没有匹配项，已有选择仍保留。</Text>}</DetailOverlay>}
 </View>;
}

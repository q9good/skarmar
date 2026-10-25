import React,{useRef,useState} from 'react';
import {View} from 'react-native';
import {Text,Pressable} from './readability';
import type {Snapshot} from './foundation';
function signature(data:Snapshot){return JSON.stringify([data.goals,data.methods,data.knowledge,data.actions,data.lessons,data.sessions,data.catalog].map(rows=>rows.map(r=>['id' in r?r.id:r.input.id,r.version])));}
export function useStableOptions(data:Snapshot,key:string|null,selected:string[]=[]){
 const [revision,redraw]=useState(0);
 const ref=useRef<{key:string|null;data:Snapshot;signature:string}>({key:null,data,signature:signature(data)});
 if(!key||ref.current.key!==key)ref.current={key,data,signature:signature(data)};
 // Explicit quick creations must remain visible immediately. Only IDs already
 // selected by the user's own form may be appended; polling never inserts options.
 let value=ref.current.data;
 for(const name of ['goals','methods','catalog','knowledge','actions','lessons','sessions'] as const){
  const ids=new Set(value[name].map(v=>'id' in v?v.id:v.input.id));
  const extra=data[name].filter(v=>selected.includes('id' in v?v.id:v.input.id)&&!ids.has('id' in v?v.id:v.input.id));
  if(extra.length)value={...value,[name]:[...value[name],...extra]};
 }
 if(value!==ref.current.data)ref.current.data=value;
 const changed=Boolean(key&&ref.current.signature!==signature(data));
 const refresh=()=>{ref.current={key,data,signature:signature(data)};redraw(n=>n+1);};
 const adopt=(id:string)=>{let next=ref.current.data;for(const name of ['goals','methods','catalog','knowledge','actions','lessons','sessions'] as const){const latest=data[name].find(v=>('id' in v?v.id:v.input.id)===id);if(latest)next={...next,[name]:next[name].map(v=>('id' in v?v.id:v.input.id)===id?latest:v)};}ref.current={...ref.current,data:next};redraw(n=>n+1);};
 return {options:value,changed,refresh,adopt,revision};
}
export function OptionsUpdate({changed,refresh}:{changed:boolean;refresh:()=>void}){
 return changed?<View style={{padding:10,gap:6,backgroundColor:'#F0F3ED',borderRadius:8}}><Text>候选项有更新，当前填写和列表位置仍保留。核对后再刷新选项。</Text><Pressable accessibilityRole="button" onPress={refresh} style={{padding:10,backgroundColor:'#E0EBDC',borderRadius:8}}><Text>核对并刷新候选项</Text></Pressable></View>:null;
}

export function useStableValue<T>(value:T,key:string|null){
 const [,redraw]=useState(0),sig=JSON.stringify(value),ref=useRef({key,value,sig});
 if(!key||ref.current.key!==key)ref.current={key,value,sig};
 return {value:ref.current.value,changed:Boolean(key&&ref.current.sig!==sig),refresh:()=>{ref.current={key,value,sig};redraw(n=>n+1);}};
}

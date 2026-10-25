import React,{useEffect,useState} from 'react';
import {View,Platform} from 'react-native';
import {Text,Pressable} from './readability';
import {DetailOverlay} from './overlay';
import {draftText,type DraftStore,type DraftBackups} from './draft-store';
import {backupKey,subscribeDrafts} from './draft-store-native';
function Button({label,press,disabled=false}:{label:string;press:()=>void;disabled?:boolean}){return <Pressable accessibilityRole="button" accessibilityState={{disabled}} disabled={disabled} onPress={press} style={{padding:12,minHeight:46,borderRadius:8,backgroundColor:'#E9F2E9',opacity:disabled?.5:1}}><Text>{label}</Text></Pressable>;}
export function DraftRecovery<T>({store,scope,label,adopted,restored}:{store:DraftStore<T>|null;scope:string;label:string;adopted:(rows:Record<string,T>)=>void;restored:(entry:string,value:T)=>void}){
 const [copies,setCopies]=useState<DraftBackups<T>|null>(null),[open,setOpen]=useState(false),[busy,setBusy]=useState(false),[error,setError]=useState('');
 useEffect(()=>{let active=true;const reload=()=>{if(store)void store.backups().then(v=>{if(active)setCopies(v)}).catch(e=>{if(active)setError(e instanceof Error?e.message:'本机保留记录未能读取');});};reload();const unsubscribe=subscribeDrafts(scope,reload),storage=(e:StorageEvent)=>{if(e.key===backupKey(scope))reload();};if(Platform.OS==='web')window.addEventListener('storage',storage);return()=>{active=false;unsubscribe();if(Platform.OS==='web')window.removeEventListener('storage',storage);};},[scope,store]);
 const act=async(fn:()=>Promise<void>)=>{if(busy)return;setBusy(true);setError('');try{await fn();}catch(e){setError(e instanceof Error?e.message:'本机核对未完成，原内容仍保留');}finally{setBusy(false);}};
 const entries=Object.entries(copies?.items??{}).filter(([,b])=>b.local!==undefined);if(!entries.length&&!error)return null;
 return <View style={{gap:6,padding:10,backgroundColor:'#FFF2DA',borderRadius:8}}><Text>本机有{entries.length}份保留的{label}填写。核对后再继续，服务器记录不会因此自动修改。</Text>{error&&<Text accessibilityRole="alert">{error}</Text>}<Button label={`核对保留的${label}填写`} press={()=>setOpen(true)}/>
 {open&&<DetailOverlay title={`核对${label}窗口填写`} close={()=>{if(!busy)setOpen(false);}}><Text>载入前会保留当前填写；已保存到服务器的记录仍需通过正常提交和版本核对。待确认的请求保持原样。</Text>{error&&<Text accessibilityRole="alert">{error}</Text>}
 <Button label="备份本窗口并载入已保存草稿" disabled={busy||!store} press={()=>void act(async()=>{const rows=await store!.adopt();adopted(rows);setOpen(false);})}/>
 {entries.map(([id,b],i)=><View key={id} style={{gap:7,padding:10,backgroundColor:'#FFFFFF'}}><Text>保留填写 {i+1} · {new Date(b.at).toLocaleString('zh-CN',{timeZone:'Asia/Shanghai'})}</Text>{draftText(b.local).map((line,n)=><Text key={n} selectable>{line}</Text>)}<Button label={`载入保留填写 ${i+1} 进行核对`} disabled={busy||!store} press={()=>void act(async()=>{const result=await store!.restore(id);restored(result.entry,result.value);setOpen(false);})}/></View>)}
 </DetailOverlay>}
 </View>;
}

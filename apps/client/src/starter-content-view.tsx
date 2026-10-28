import React,{useState} from 'react';
import {View} from 'react-native';
import {Text,Pressable} from './readability';
import {DetailOverlay} from './overlay';
import {Disclosure} from './disclosure';
import {request} from './api';
import {starterGoals,starterMethods,starterCatalog,defaultStarterKeys} from './starter-content';

function Button({title,press,disabled=false}:{title:string;press:()=>void;disabled?:boolean}) {
  return <Pressable accessibilityRole="button" accessibilityState={{disabled}} disabled={disabled} onPress={press} style={{padding:12,minHeight:44,borderRadius:10,backgroundColor:'#E9F2E9',opacity:disabled?.5:1}}><Text style={{color:'#34765B',fontSize:17}}>{title}</Text></Pressable>;
}
export function ReferencePicker({kind,disabled,fork,choose}:{kind:'goal'|'method';disabled:boolean;fork:boolean;choose:(key:string)=>void}) {
  const [open,setOpen]=useState(false),[selected,setSelected]=useState('');
  const title=kind==='goal'?'参考长期方向':'参考做法';
  const options=kind==='goal'?starterGoals:starterMethods;
  const reference=options.find(r=>r.key===selected);
  return <><Button title={title} disabled={disabled} press={()=>{setSelected('');setOpen(true);}}/>
    {open&&<DetailOverlay title={title} close={()=>setOpen(false)}>
      <Text>参考内容可修改，不代表孩子的实际表现或已验证有效。查看不会保存。</Text>
      {!reference?options.map(r=><Button key={r.key} title={'查看参考：'+r.input.title} press={()=>setSelected(r.key)}/>):<>
        <Text accessibilityRole="header">{reference.input.title}</Text>
        {Object.entries(reference.input).filter(([key,value])=>['meaning','criteria','support','steps','observe','scope','guidance'].includes(key)&&typeof value==='string'&&value).map(([key,value])=><Text key={key}>{({meaning:'生活意义',criteria:'观察提示',support:'帮助条件',steps:'具体做法',observe:'观察意图',scope:'适用条件',guidance:'参考说明'} as Record<string,string>)[key]}：{String(value)}</Text>)}
        {kind==='method'&&<Text>目标关联以表单里明确选择的目标为准，参考不会自动增加关联。</Text>}
        <Button title={fork?'按参考新建，保留当前草稿':'使用参考填写'} press={()=>{setOpen(false);choose(reference.key);}}/>
        <Button title="返回参考列表" press={()=>setSelected('')}/>
      </>}
    </DetailOverlay>}
  </>;
}
type PreviewInput={title?:string;name?:string;meaning?:string;criteria?:string;steps?:string;observe?:string;scope?:string;active?:boolean;goal_ids?:string[];state?:string;archived?:boolean};
type Row={key:string;kind:string;target_id:string;skipped:boolean;starter_status:'new'|'reuse'|'already_added'|'removed';mapped:{kind:string;value:PreviewInput|{value:{input:PreviewInput}}}};
type Plan={preview_id:string;digest:string;rows:Row[]};
type Receipt={saved:number;reused:number;skipped:number};
const labels:Record<string,string>={goal:'目标',method:'方法',course:'课程',teacher:'老师'};
export function StarterSetupButton({refresh}:{refresh:()=>Promise<void>}) {
  const [open,setOpen]=useState(false),[keys,setKeys]=useState<string[]>([...defaultStarterKeys]),[plan,setPlan]=useState<Plan|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),[receipt,setReceipt]=useState<Receipt|null>(null),[notice,setNotice]=useState('');
  const toggle=(key:string)=>{setKeys(old=>old.includes(key)?old.filter(k=>k!==key):[...old,key]);setPlan(null);setReceipt(null);setNotice('');};
  const check=(key:string,label:string)=><Pressable key={key} accessibilityRole="checkbox" accessibilityLabel={label} aria-checked={keys.includes(key)} accessibilityState={{checked:keys.includes(key),disabled:busy}} disabled={busy} onPress={()=>toggle(key)} style={{padding:12,minHeight:44,backgroundColor:'#E9F2E9',borderRadius:8}}><Text>{keys.includes(key)?'☑':'☐'} {label}</Text></Pressable>;
  const act=async(action:()=>Promise<void>)=>{if(busy)return;setBusy(true);setError('');setNotice('');try{await action();}catch(e){setError(e instanceof Error?e.message:'操作未完成，请保留预览后重试');}finally{setBusy(false);}};
  const preview=()=>void act(async()=>{setReceipt(null);const p=await request<Plan>('/data-transfer','POST',{kind:'starter_preview',value:{keys}});setPlan(p);});
  const rowInput=(r:Row):PreviewInput=>'value' in r.mapped.value?r.mapped.value.value.input:r.mapped.value;
  const names=new Map(plan?.rows.map(r=>[r.target_id,rowInput(r)?.title??rowInput(r)?.name]));
  return <><Button title="初始内容与参考" press={()=>setOpen(true)}/>
    {open&&<DetailOverlay title="初始内容与参考" close={()=>{if(!busy)setOpen(false);}}>
      <Text>选择要创建的内容，先预览再确认。方法按需勾选；不生成课记、训练结果或有效经验。已有内容不会被覆盖。</Text>
      <Text>创建后的目标和方法可在原页面修改；长期方向按原有规则周期回看。</Text>
      <Disclosure title="长期方向（6项）" defaultOpen>{starterGoals.map(r=>check(r.key,r.input.title))}</Disclosure>
      <Disclosure title="参考方法（10项，按需选择）">
        <Text>方法需同时选中其关联方向，或这些方向此前已通过本入口创建。请选择准备使用的方法，创建不表示已经验证有效。</Text>
        <Button title="选取全部参考方法及关联方向" disabled={busy} press={()=>{setKeys(old=>[...new Set([...old,...starterMethods.map(r=>r.key),...starterGoals.map(r=>r.key)])]);setPlan(null);setReceipt(null);}}/>
        {starterMethods.map(r=><View key={r.key} style={{gap:6}}>{check(r.key,r.input.title)}<Text>关联方向：{r.goal_keys.map(key=>starterGoals.find(g=>g.key===key)?.input.title).join('、')}</Text></View>)}
      </Disclosure>
      <Disclosure title="课程和老师（10项）"><Text>两个目录独立维护，历史记录只影响选择顺序，不固定配对。</Text>{starterCatalog.map(r=>check(r.key,(r.kind==='teacher'?'老师：':'课程：')+r.name))}</Disclosure>
      <Button title="生成初始内容预览" disabled={busy||!keys.length} press={preview}/>
      {error&&<Text accessibilityRole="alert">{error}</Text>}
      {plan&&<View style={{gap:12}}><Text accessibilityRole="header">初始内容写入预览</Text>
        <Text>新增 {plan.rows.filter(r=>r.starter_status==='new').length}，使用已有 {plan.rows.filter(r=>r.starter_status==='reuse').length}，此前已处理 {plan.rows.filter(r=>r.skipped).length}。同名项目只使用现有内容及关联，不改名、不恢复停用项；多条同名无法明确时会阻止预览。</Text>
        {plan.rows.map(r=>{const input=rowInput(r);return <View key={r.key} style={{gap:5,padding:12,borderWidth:1,borderColor:'#CAD8CF',borderRadius:8}}>
          <Text>{labels[r.kind]}：{input?.title??input?.name} · {({new:'新增',reuse:'使用已有，保留现有内容',already_added:'已处理，跳过',removed:'已删除，保持删除'})[r.starter_status]}</Text>
          {input?.active===false&&<Text>此选项已停用，本次保持停用。</Text>}
          {input.state&&<Text>状态：{({active:r.kind==='goal'?'关注中':'使用中',maintaining:'维持',paused:'暂停',retired:'停用',not_started:'未开始',achieved:'已达成'} as Record<string,string>)[input.state]??input.state}{input.archived?' · 已收起':''}</Text>}
          {[input?.meaning,input?.criteria,input?.steps,input?.observe,input?.scope].filter(Boolean).map((v,i)=><Text key={i}>{v}</Text>)}
          {(input.goal_ids??[]).length>0&&<Text>关联目标：{input.goal_ids?.map(id=>names.get(id)??'已有目标（保留原关联）').join('、')}</Text>}
        </View>;})}
        <Button title={busy?'正在处理初始内容…':'确认按预览创建'} disabled={busy||!plan.rows.some(r=>!r.skipped)} press={()=>void act(async()=>{
          const result=await request<Receipt>('/data-transfer','POST',{kind:'commit',value:{preview_id:plan.preview_id,digest:plan.digest}});setReceipt(result);
          try{await refresh();}catch{setNotice('初始内容已保存，页面更新失败，请稍后刷新。');}
        })}/>
        {!busy&&<Button title="重新核对初始内容预览" press={preview}/>}
      </View>}
      {receipt&&<Text role="status">已确认：新增 {receipt.saved} 项，使用已有 {receipt.reused} 项，跳过 {receipt.skipped} 项。</Text>}
      {notice&&<Text role="status">{notice}</Text>}
    </DetailOverlay>}
  </>;
}

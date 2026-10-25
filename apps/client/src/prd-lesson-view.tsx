import {authSessionGuard} from './api';
import {resetAfterFailure} from './pending-request';
import {DraftRecovery} from './draft-recovery';
import {createDraftStore} from './draft-store-native';
import type {DraftStore} from './draft-store';
import {useStableOptions,OptionsUpdate} from './stable-options';
import {RelationPicker} from './relation-picker';
import {GuidedField} from './guided-field';
import {LessonPractice} from './lesson-practice-view';
import {SaveStatus,trackLocalSave} from './save-status';
import {ExampleHint} from './input-guidance';
import {HelpButton} from './product-help';
import {useLessonOutbox} from './prd-outbox-view';
import React, { useEffect, useRef, useState } from 'react';
import { View } from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { ApiError, request } from './api';
import { DateField, Select } from './controls';
import { localDate, uuid } from './draft';
import { useAuth } from './auth';
import { DetailOverlay } from './overlay';
import { Pressable, Text, TextInput } from './readability';
import type { Catalog, Saved, Snapshot } from './foundation';
import { decodeLessonDrafts, emptyObservation, organizationOf, observationText, needsReview, monthRange, shiftMonth, teacherTotals, commonChoices, type Lesson, type LessonDraft, type LessonWrite, type Observation, type OrganizationInput, type NewKnowledge } from './prd-lessons';
const queues = new Map<string, Promise<void>>();
const localDateOf = (stamp:string) => new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date(stamp));
const labelsForFeedback=(v:string)=>({helpful:'有帮助',uncertain:'不确定',unhelpful:'没帮上'} as Record<string,string>)[v]??v;
const note = (e: unknown) => e instanceof Error ? e.message : '操作未完成，请保留填写后重试';
function Action({ title, press, disabled, secondary = true }: { title: string; press: () => void; disabled?: boolean; secondary?: boolean }) {
  return <Pressable accessibilityRole="button" disabled={disabled} accessibilityState={{ disabled }} onPress={press} style={{ padding: 12, borderRadius: 10, backgroundColor: secondary ? '#E9F2E9' : '#34765B', opacity: disabled ? .5 : 1 }}><Text style={{ color: secondary ? '#34765B' : '#FFFFFF', fontSize: 17 }}>{title}</Text></Pressable>;
}
function Field({ label, question, value, change, placeholder, disabled, multiline = true }: { label: string; question?: string; value: string; change: (v:string)=>void; placeholder?: string; disabled?: boolean; multiline?: boolean }) {return <GuidedField label={label} value={value} change={change} question={question} placeholder={placeholder} disabled={disabled} multiline={multiline}/>;}
function Check({ title, checked, press, disabled }: { title: string; checked: boolean; press: () => void; disabled?: boolean }) {
  return <Pressable accessibilityRole="checkbox" accessibilityLabel={title} aria-checked={checked} accessibilityState={{ checked, disabled }} onPress={press} disabled={disabled} style={{ padding: 10, backgroundColor: '#E9F2E9', borderRadius: 8 }}><Text>{checked ? '☑' : '☐'} {title}</Text></Pressable>;
}
function ReadObservation({ value }: { value: Observation }) {
  return <View style={{ gap: 8 }}>{value.course_started&&<Text>家人明确标记：这门课程刚开始上。</Text>}{[value.notes, value.teacher_notes && `老师的做法和建议：${value.teacher_notes}`, value.progress_notes && `进展与泛化：${value.progress_notes}`, value.interest_notes && `兴趣与互动：${value.interest_notes}`, value.difficulty_notes && `困难与成人回应：${value.difficulty_notes}`, value.cause_guess && `原因猜测（待验证）：${value.cause_guess}`].filter(Boolean).map((text,i) => <Text selectable key={i}>{text}</Text>)}{!observationText(value) && <Text>还没有补充课堂观察。</Text>}</View>;
}
function CatalogChoice({ kind, value, change, data, accepted, lesson, disabled }: { kind: 'course'|'teacher'; value: string; change: (v:string)=>void; data: Snapshot; accepted: (s:Saved)=>void; lesson?: Lesson; disabled: boolean }) {
  const outbox=useLessonOutbox();
  const [panel, setPanel] = useState(false), [search, setSearch] = useState(''), [name, setName] = useState(''), [error, setError] = useState(''), [saving, setSaving] = useState(false);
  const pending = useRef<Parameters<typeof request>[2]>(undefined);
  const label = kind === 'course' ? '课程' : '老师';
  const options = data.catalog.filter(c => c.input.kind === kind && (c.input.active || c.id === value));
  const shortcuts=useRef(commonChoices(data.lessons,data.catalog,kind,localDate())).current;
  const [pickerOptions,setPickerOptions]=useState<Catalog[]>([]);
  const optionLabel = (c:Catalog) => lesson?.[kind].id === c.id ? `${lesson[kind].name}${lesson[kind].name !== c.input.name ? '（历史名称）' : ''}` : c.input.name;
  return <View style={{ gap: 8 }}>
    <Action title={`${kind==='course'?'课程名称（必填）':'授课老师（必填）'}：${options.find(c=>c.id===value)?optionLabel(options.find(c=>c.id===value)!):`请选择${label}`}`} disabled={disabled} press={()=>{setPickerOptions(options.filter(c=>c.input.active));setPanel(true);}} />
    <View style={{flexDirection:'row',flexWrap:'wrap',gap:8}}>{shortcuts.map(c => <Action key={c.id} title={c.input.name} disabled={disabled||!data.catalog.some(row=>row.id===c.id&&row.input.active)} press={() => change(c.id)} />)}</View>
    {panel && <DetailOverlay title={`选择${label}`} close={() => { if (!saving) setPanel(false); }}>
      <Field label={`搜索${label}`} value={search} change={setSearch} multiline={false} />
      {pickerOptions.filter(c => c.input.name.includes(search)).map(c => <Action key={c.id} title={c.input.name} disabled={saving||!data.catalog.some(row=>row.id===c.id&&row.input.active)} press={() => { change(c.id); setPanel(false); }} />)}
      <Field label={`新增${label}名称`} value={name} change={setName} disabled={saving || Boolean(pending.current)} multiline={false} placeholder={kind === 'course' ? '例如：语言课' : '例如：张老师'} />
      {error && <Text accessibilityRole="alert">{error}</Text>}
      <Action title={saving ? '正在保存…' : '保存并选用'} disabled={saving || !name.trim()} secondary={false} press={() => void (async () => {
        setSaving(true); setError('');
        const body = pending.current ?? { id: uuid(), operation_id: uuid(), expected_version: null, change_note: '', mutation: { kind: 'catalog', value: { kind, name: name.trim(), active: true } } };
        pending.current = body;
        try { const payload = body as import('./foundation').Write; await outbox.queue.enqueue({kind:'catalog',value:payload}); change(payload.id); pending.current = undefined; setName(''); setPanel(false); await outbox.sync(); }
        catch(e) { if(e instanceof ApiError && [400,401,403,409,413,422].includes(e.status)) pending.current = undefined; setError(note(e)); }
        finally { setSaving(false); }
      })()} />
    </DetailOverlay>}
  </View>;
}
const themes = [
  ['teacher_notes','老师的做法和建议','老师怎样帮助或提醒？哪些建议可以带回家？','例如：老师先示范，再留时间让他尝试'],
  ['progress_notes','做得更好了／在别处也会了','和以前比有什么变化？换材料或换人时也会吗？','例如：换一本书，也会主动指给人看'],
  ['interest_notes','感兴趣、主动尝试、和人互动的','什么吸引了他？他怎样主动表达或邀请别人？','例如：拿着新玩具来找我一起玩'],
  ['difficulty_notes','遇到的困难','当时做什么？大人怎样回应，后来怎么样？','例如：等候时离开，大人陪他休息后再试'],
] as const;
function ObservationFields({ value, change, disabled }: { value: Observation; change: (v:Partial<Observation>)=>void; disabled: boolean }) {
  const [open, setOpen] = useState<string[]>(() => themes.filter(([key]) => value[key]).map(([key])=>key));
  return <>
    <Field label="今天课上怎么样？" question="记下看到的或老师说的。可以用输入法的话筒说话。" value={value.notes} change={notes => change({ notes })} placeholder="例如：今天一起玩了轮流游戏" disabled={disabled} />
    <Text>想多说一点？以下内容都可以跳过。</Text>
    {themes.map(([key,title,question,example]) => <View key={key} style={{ gap: 8 }}><Action title={`${title}${value[key] ? ' · 已写' : ''}`} disabled={disabled} press={() => setOpen(old => old.includes(key) ? old.filter(k=>k!==key) : [...old,key])} />
      {open.includes(key) && <Field label={title} question={question} value={value[key]} change={v=>change({ [key]: v })} placeholder={example} disabled={disabled} />}
    </View>)}
    {open.includes('difficulty_notes') && <Field label="原因猜测（可选）" question="这是待核实的想法，与看到的事实分开记。" value={value.cause_guess} change={cause_guess=>change({ cause_guess })} disabled={disabled} placeholder="例如：也许是在等候时需要休息" />}
    <Check title="这门课程刚开始上（家人明确标记，可选）" checked={value.course_started===true} disabled={disabled} press={()=>change({course_started:!value.course_started})}/><Text>第一次录入不等于第一次上课；只有明确标记才进入复盘的同期变化。</Text>
    <Check title="请爸爸妈妈看看" checked={value.request_review} disabled={disabled} press={()=>change({ request_review: !value.request_review })} />
    {value.request_review && <Field label="给家人的留言（可选）" value={value.review_message} change={review_message=>change({ review_message })} disabled={disabled} placeholder="不写留言也可以发出查看请求" />}
  </>;
}
function Bindings({ data, goalIds, methodIds, change, disabled }: { data:Snapshot; goalIds:string[]; methodIds:string[]; change:(g:string[],m:string[])=>void; disabled:boolean }) {return <><Text>关联目标</Text><RelationPicker label="关联目标" options={data.goals.map(g=>({id:g.id,label:g.input.title}))} selected={goalIds} disabled={disabled} toggle={id=>change(goalIds.includes(id)?goalIds.filter(v=>v!==id):[...goalIds,id],methodIds)}/><Text>关联方法</Text><RelationPicker label="关联方法" options={data.methods.map(m=>({id:m.id,label:m.input.title}))} selected={methodIds} disabled={disabled} toggle={id=>change(goalIds,methodIds.includes(id)?methodIds.filter(v=>v!==id):[...methodIds,id])}/></>;}

function OrganizationFields({ value, change, data, lesson, actorId, source, disabled,openKnowledge }: { value:OrganizationInput; change:(v:Partial<OrganizationInput>)=>void; data:Snapshot; lesson:Lesson; actorId:string; source?:Observation; disabled:boolean;openKnowledge?:(id:string)=>void }) {
  const [knowledgeEditor,setKnowledgeEditor]=useState<number|null>(null), [actionEditor,setActionEditor]=useState<number|null>(null);
  const toggle = (ids:string[],id:string)=>ids.includes(id)?ids.filter(v=>v!==id):[...ids,id];
  const selected=value.targets.map(t=>t.goal_id);
  const modifyKnowledge=(index:number,patch:Partial<NewKnowledge>)=>change({ new_knowledge:value.new_knowledge.map((k,i)=>i===index?{...k,...patch}:k) });
  return <>
    <Text style={{fontSize:18,fontWeight:'600'}}>整理时依据的课堂观察</Text><ReadObservation value={source ?? lesson.observation} />
    {value.seen_observation_version!==lesson.observation_version && <><Text>原观察后来有补充。已有经验保留原内容，可核对后使用最新观察。</Text><Action title="改用最新观察继续整理" disabled={disabled} press={()=>change({seen_observation_version:lesson.observation_version})} /></>}
    <Text style={{fontSize:18,fontWeight:'600'}}>这堂课明确涉及的目标</Text>
    <RelationPicker label="课堂目标" options={data.goals.filter(g=>g.input.level!=='long').map(g=>({id:g.id,label:g.input.title}))} selected={selected} disabled={disabled} toggle={id=>change({targets:selected.includes(id)?value.targets.filter(t=>t.goal_id!==id):[...value.targets,{goal_id:id,note:''}]})}/>
    {value.targets.map(t=><Field key={t.goal_id} label={`${data.goals.find(g=>g.id===t.goal_id)?.input.title}的一句观察（可选）`} value={t.note} change={note=>change({targets:value.targets.map(v=>v.goal_id===t.goal_id?{...v,note}:v)})} disabled={disabled}/>)}
    <Text>实际使用的方法（没有整理时可不选）</Text><RelationPicker label="课堂实际方法" prefix="实际方法：" options={data.methods.map(m=>({id:m.id,label:m.input.title,related:m.input.goal_ids.some(id=>selected.includes(id))}))} selected={value.method_ids} disabled={disabled} toggle={id=>change({method_ids:toggle(value.method_ids,id)})}/>
    <Text>明确关注的长期方向（可选）</Text><RelationPicker label="课堂长期关注" prefix="长期关注：" options={data.goals.filter(g=>g.input.level==='long').map(g=>({id:g.id,label:g.input.title}))} selected={value.focus_goal_ids} disabled={disabled} toggle={id=>change({focus_goal_ids:toggle(value.focus_goal_ids,id)})}/>
    <LessonPractice lesson={lesson} data={data} value={value} change={change} disabled={disabled} openKnowledge={openKnowledge}/>
    <Text style={{fontSize:18,fontWeight:'600'}}>需要留下什么？</Text>
    {value.new_knowledge.map((k,i)=><View key={k.id} style={{gap:8,padding:12,backgroundColor:'#FFFFFF',borderRadius:10}}>
      <Field label={k.kind==='difficulty'?`困难内容 ${i+1}`:`经验内容 ${i+1}`} value={k.content} change={content=>modifyKnowledge(i,{content})} disabled={disabled} placeholder={k.kind==='difficulty'?'写下实际与预期的差别，以及成人的回应':'写下有帮助的做法或待试的想法'} />
      {k.kind==='experience' ? <Select label={`经验阶段 ${i+1}`} value={k.stage ?? ''} disabled={disabled} onChange={stage=>modifyKnowledge(i,{stage:stage?stage as NewKnowledge['stage']:null})} options={[{value:'',label:'请选择当前阶段'},{value:'idea',label:'想法'},{value:'try',label:'可尝试'},{value:'helpful',label:'已有多次观察到帮助'}]} /> : <DateField label={`下次回看日期 ${i+1}（可选）`} value={k.next_review_date??''} disabled={disabled} onChange={date=>modifyKnowledge(i,{next_review_date:date||null})} />}
      <Text>关联：{k.goal_ids.map(id=>data.goals.find(g=>g.id===id)?.input.title).concat(k.method_ids.map(id=>data.methods.find(m=>m.id===id)?.input.title)).filter(Boolean).join('、')||'尚未选择'}</Text>
      <Action title={`调整关联 ${i+1}`} disabled={disabled} press={()=>setKnowledgeEditor(i)} />
      <Action title={`取消新增条目 ${i+1}`} disabled={disabled} press={()=>change({new_knowledge:value.new_knowledge.filter((_,j)=>j!==i)})} />
    </View>)}
    {(['difficulty','experience'] as const).map(kind=><Action key={kind} title={kind==='difficulty'?'记一条困难':'记一条经验'} disabled={disabled} press={()=>change({ new_knowledge:[...value.new_knowledge,{id:uuid(),kind,content:'',goal_ids:[...selected,...value.focus_goal_ids],method_ids:[...value.method_ids],stage:null,next_review_date:null}] })} />)}
    {value.new_actions.map((a,i)=><View key={a.id} style={{gap:8}}><DateField label={`下一步建议日期 ${i+1}（可选）`} value={a.suggested_date??''} disabled={disabled} onChange={v=>change({new_actions:value.new_actions.map((a,j)=>i===j?{...a,suggested_date:v||null}:a)})}/><Field label={`下一步 ${i+1}`} value={a.content} disabled={disabled} placeholder="例如：请老师示范后在家尝试" change={content=>change({new_actions:value.new_actions.map((v,j)=>j===i?{...v,content}:v)})} />
      <Check title={`下一步 ${i+1} 由我负责`} checked={a.assignee_id===actorId} disabled={disabled} press={()=>change({new_actions:value.new_actions.map((v,j)=>j===i?{...v,assignee_id:v.assignee_id===actorId?null:actorId}:v)})} />
      <Text>关联：{a.goal_ids.map(id=>data.goals.find(g=>g.id===id)?.input.title).concat(a.method_ids.map(id=>data.methods.find(m=>m.id===id)?.input.title)).filter(Boolean).join('、')||'尚未选择'}</Text><Action title={`调整下一步关联 ${i+1}`} disabled={disabled} press={()=>setActionEditor(i)}/><Text>取消“由我负责”后由家人待认领。</Text><Action title={`取消下一步 ${i+1}`} disabled={disabled} press={()=>change({new_actions:value.new_actions.filter((_,j)=>j!==i)})} /></View>)}
    <Action title="写下下一步" disabled={disabled} press={()=>change({new_actions:[...value.new_actions,{id:uuid(),content:'',goal_ids:[...selected,...value.focus_goal_ids],method_ids:[...value.method_ids],assignee_id:actorId}]})} />
    <Select label="这条记录如何处理" value={value.processing} disabled={disabled} onChange={processing=>change({processing:processing as OrganizationInput['processing'],request_generation:lesson.request_generation})} options={[{value:'keep',label:'只保存整理，不改变待办'},{value:'claim',label:'我来整理'},{value:'take_over',label:'从其他家人接手'},{value:'complete',label:'已处理'},{value:'reopen',label:'重新留待处理'}]} />
    <Field label="处理说明（可选）" value={value.outcome} change={outcome=>change({outcome})} disabled={disabled} placeholder="例如：已记下困难，下次复盘再看看" />
    {actionEditor!==null&&value.new_actions[actionEditor]&&<DetailOverlay title="调整下一步的关联" close={()=>setActionEditor(null)}><Bindings data={data} goalIds={value.new_actions[actionEditor].goal_ids} methodIds={value.new_actions[actionEditor].method_ids} disabled={disabled} change={(goal_ids,method_ids)=>change({new_actions:value.new_actions.map((a,i)=>i===actionEditor?{...a,goal_ids,method_ids}:a)})}/></DetailOverlay>}
    {knowledgeEditor!==null&&value.new_knowledge[knowledgeEditor]&&<DetailOverlay title="调整经验或困难的关联" close={()=>setKnowledgeEditor(null)}><Bindings data={data} goalIds={value.new_knowledge[knowledgeEditor].goal_ids} methodIds={value.new_knowledge[knowledgeEditor].method_ids} disabled={disabled} change={(goal_ids,method_ids)=>modifyKnowledge(knowledgeEditor,{goal_ids,method_ids})} /></DetailOverlay>}
  </>;
}

export function LessonHub({ data:remoteData, actorId, actorName, admin, mode, refresh, savedLesson, catalogSaved, openGoal, openMethod, scrollTop, interventionEntry, openAction,requested,onRequested,openKnowledge,inboxManaged=false }: { data:Snapshot; actorId:string; actorName:string; admin:boolean; mode:'today'|'records'|'drafts'; refresh:()=>Promise<void>; savedLesson:(l:Lesson)=>void; catalogSaved:(s:Saved)=>void; openGoal:(id:string)=>void; openMethod:(id:string)=>void; scrollTop:()=>void; interventionEntry?:React.ReactNode;openAction?:(id:string)=>void;requested?:{id:string;nonce:string;view?:'detail'|'organize'}|null;onRequested?:()=>void;inboxManaged?:boolean;openKnowledge?:(id:string)=>void }) {
  const draftStore=useRef<DraftStore<LessonDraft>|null>(null);
 const outbox=useLessonOutbox();
  const data={...remoteData,catalog:[...remoteData.catalog,...outbox.catalog.filter(c=>!remoteData.catalog.some(r=>r.id===c.id))]};
  const scope = `${actorId}`;
  // Space is supplied by the authenticated identity, never by the form.
  const authScope = useAuth().identity.space;
  const storageKey = `skarmar:${authScope}:${scope}:lesson-drafts:v1`;
  const [drafts,setDrafts]=useState<Record<string,LessonDraft>>({}), [editor,setEditor]=useState<string|null>(null), [ready,setReady]=useState(false), [error,setError]=useState(''), [notice,setNotice]=useState(''), [busy,setBusy]=useState(false), [receipt,setReceipt]=useState<string|null>(null), [detail,setDetail]=useState<string|null>(null);
  const [historyPage,setHistoryPage]=useState(0),[historyMore,setHistoryMore]=useState(false),[historyLoading,setHistoryLoading]=useState(false);const historySequence=useRef(0);
  const [customRange,setCustomRange]=useState(false),[rangeFrom,setRangeFrom]=useState(''),[rangeTo,setRangeTo]=useState('');
  const [otherTasks,setOtherTasks]=useState(false),[intro,setIntro]=useState(false);
  useEffect(()=>{void AsyncStorage.getItem(storageKey+':introduced').then(v=>{if(!v)setIntro(true);}).catch(()=>undefined);},[storageKey]);
  const [discard,setDiscard]=useState(false), [historyOpen,setHistoryOpen]=useState(false), [history,setHistory]=useState<import('./prd-lessons').LessonRevision[]>([]), [historyError,setHistoryError]=useState('');
  const [section,setSection]=useState<'classroom'|'totals'>('classroom'), [month,setMonth]=useState(()=>localDate().slice(0,7)), [countMonth,setCountMonth]=useState(()=>shiftMonth(localDate().slice(0,7),-1)), [selectedTeacher,setSelectedTeacher]=useState<string|null>(null), [search,setSearch]=useState(''), [teachersOnly,setTeachersOnly]=useState(false);
  const refs=useRef(drafts), alive=useRef(true), inFlight=useRef(false);
  useEffect(()=>{
    alive.current=true;
    void (async()=>{ await queues.get(storageKey)?.catch(()=>undefined);const parsed=decodeLessonDrafts(await AsyncStorage.getItem(storageKey));
      if(alive.current){draftStore.current=createDraftStore(storageKey,decodeLessonDrafts,parsed);refs.current=parsed;setDrafts(parsed);setReady(true);}
    })().catch(e=>setError(note(e)));
    return()=>{alive.current=false;};
  },[storageKey]);
  const persist=(next:Record<string,LessonDraft>)=>{
    refs.current=next;setDrafts(next);
    const task=draftStore.current!.save(next);queues.set(storageKey,task);trackLocalSave(storageKey,task);
    void task.catch(e=>{if(alive.current)setError(note(e));});return task;
  };
 const draftRecovery=()=> <DraftRecovery store={draftStore.current} scope={storageKey} label="课记" adopted={rows=>{refs.current=rows;setDrafts(rows);setError('');}} restored={(entry,value)=>{const rows={...refs.current,[entry]:value};refs.current=rows;setDrafts(rows);setEditor(entry);setError('');}}/>;
  const open=(lesson?:Lesson,phase:LessonDraft['phase']='basic')=>{
    if(!ready||!outbox.ready)return;
    const id=lesson?.id??uuid(), key=`${id}:${phase==='organization'?'organization':'observation'}`;
    if(!refs.current[key]){
      const draft:LessonDraft={phase,source:lesson?.observation,request:{id,operation_id:uuid(),expected_version:lesson?(phase==='organization'?lesson.organization_version:lesson.observation_version):null,action:phase==='organization'&&lesson?{kind:'organization',value:organizationOf(lesson)}:{kind:'observation',value:lesson?{...lesson.observation,request_review:false,review_message:''}:emptyObservation(localDate())}}};
      void persist({...refs.current,[key]:draft});
    }
    setEditor(key);setDiscard(false);setError('');setNotice('');setDetail(null);
  };
  useEffect(()=>{if(!requested||!ready)return;const lesson=data.lessons.find(l=>l.id===requested.id);if(!lesson)return;onRequested?.();if(lesson.status==='recorded'){if(requested.view==='detail')setDetail(lesson.id);else open(lesson,'organization');}else setError('这条课记已撤销或删除，待办将刷新。');},[requested?.nonce,ready,data.lessons]);
  const draft=editor?drafts[editor]:undefined;
  const candidate=useStableOptions(data,editor,draft?.request.action.kind==='organization'?[...draft.request.action.value.targets.map(t=>t.goal_id),...draft.request.action.value.method_ids]:[]),options=candidate.options;
  const remote=draft?data.lessons.find(l=>l.id===draft.request.id):undefined;
  const expected=remote?(draft?.request.action.kind==='observation'?remote.observation_version:draft?.request.action.kind==='organization'?remote.organization_version:remote.version):null;
  const conflict=Boolean(draft&&remote&&draft.request.expected_version!==expected);
  const patch=(patch:Partial<Observation>|Partial<OrganizationInput>)=>{
    if(!draft||draft.pending||busy||!editor)return;
    const action=draft.request.action;
    if(action.kind!=='observation'&&action.kind!=='organization')return;
    const next={...draft,request:{...draft.request,operation_id:uuid(),action:{...action,value:{...action.value,...patch}} as LessonWrite['action']}};
    if(action.kind==='organization'&&'seen_observation_version' in patch&&remote&&remote.observation_version===patch.seen_observation_version)next.source=remote.observation;
    void persist({...refs.current,[editor]:next});
  };
  const perform=(lesson:Lesson,kind:'withdraw'|'delete'|'acknowledge'|'mark_error',marked=true)=>{
    if(!ready)return;
    const key=`${lesson.id}:administration`;
    const next:LessonDraft=refs.current[key]??{phase:'administration',source:lesson.observation,request:{id:lesson.id,operation_id:uuid(),expected_version:lesson.version,action:kind==='mark_error'?{kind,value:{marked}}:{kind}}};
    void persist({...refs.current,[key]:next});setEditor(key);setDetail(null);setError('');
  };
  const submit=async()=>{
    if(!draft||!editor||inFlight.current||(conflict&&!draft.pending))return;
    const key=editor,payload=draft.pending??draft.request;
    setBusy(true);inFlight.current=true;setError('');const assertSession=authSessionGuard();let sent=false;
    try{
      if(payload.expected_version===null&&payload.action.kind==='observation'){
        await outbox.queue.enqueue({kind:'lesson',value:payload},Boolean(draft.pending));
        const remaining={...refs.current};delete remaining[key];
        try{await persist(remaining);}catch{ /* Queue is durable and still hides this original draft. */ }
        setEditor(null);setError('');scrollTop();setNotice('课记已保存在本机待上传，服务器确认后才计节数。');
        await outbox.sync();const entry=outbox.queue.value.entries[payload.id];
        if(entry?.status==='confirmed'){
          setReceipt(payload.id);setNotice('已记下，家人刷新即可看到。');await refresh().catch(()=>undefined);
        }
        return;
      }
      await persist({...refs.current,[key]:{...draft,pending:payload}});assertSession();sent=true;
      const lesson=await request<Lesson>(`/prd/lessons/${payload.id}`,'PUT',payload);savedLesson(lesson);
      const next={...refs.current};delete next[key];
      let cleanupFailed=false;try{await persist(next);}catch{cleanupFailed=true;}
      setEditor(null);setReceipt(lesson.status==='recorded'?lesson.id:null);setError('');
      setNotice(cleanupFailed?'记录已保存，本机草稿暂未清除；再次提交同一次操作不会重复记录。':lesson.status==='withdrawn'?'课记已撤销，不再计节数。':lesson.status==='deleted'?'课记已删除。':'已记下，家人刷新即可看到。');
      scrollTop();try{await refresh();}catch{setNotice(v=>v+' 列表暂未刷新。');}
    }catch(e){
      if(resetAfterFailure(Boolean(draft.pending),sent,e)){void persist({...refs.current,[key]:{...draft,pending:undefined,request:{...draft.request,operation_id:uuid()}}});if(e instanceof ApiError&&e.status===409)await refresh().catch(()=>undefined);}
      setError(sent||e instanceof ApiError?note(e):draft.pending?'本机存储未完成；上次提交结果仍待确认，请保留填写后重试。':'本机草稿未保存，尚未发送，请保留页面后重试。');
    }finally{setBusy(false);inFlight.current=false;}
  };
  const selected=detail?data.lessons.find(l=>l.id===detail):undefined;
  useEffect(()=>{++historySequence.current;setHistoryLoading(false);setHistoryMore(false);setHistoryOpen(false);setHistory([]);setHistoryError('');},[detail]);
  useEffect(()=>{
    if(!historyOpen||!selected)return;
    void loadLessonHistory(selected.id);
    return()=>{++historySequence.current;};
  },[historyOpen,selected?.id,selected?.version]);
  async function loadLessonHistory(id:string,page=0){const seq=++historySequence.current;setHistoryLoading(true);setHistoryError('');try{const v=await request<{items:import('./prd-lessons').LessonRevision[];has_more:boolean}>(`/prd/process?view=lesson_history&id=${id}&page=${page}`);if(seq!==historySequence.current)return;setHistory(old=>page?[...old,...v.items].filter((r,i,all)=>all.findIndex(v=>v.lesson.version===r.lesson.version)===i):v.items);setHistoryPage(page);setHistoryMore(v.has_more);}catch(e){if(seq===historySequence.current)setHistoryError(note(e));}finally{if(seq===historySequence.current)setHistoryLoading(false);}}
  const confirmed=receipt?data.lessons.find(l=>l.id===receipt):undefined;
  const today=localDate(), active=data.lessons.filter(l=>l.status==='recorded');
  const monthBounds=monthRange(section==='totals'?countMonth:month);
  const {start,end}=section==='totals'&&customRange?{start:rangeFrom,end:rangeTo}:monthBounds;
  const validRange=[start,end].every(v=>/^\d{4}-\d{2}-\d{2}$/.test(v)&&Number.isFinite(Date.parse(v+'T00:00:00Z'))&&new Date(v+'T00:00:00Z').toISOString().slice(0,10)===v)&&start<=end;
  const records=active.filter(l=>l.observation.business_date>=start&&l.observation.business_date<=end&&(!teachersOnly||Boolean(l.observation.teacher_notes))&&(!search||[l.course.name,l.teacher.name,observationText(l.observation),...data.knowledge.filter(k=>!k.deleted&&k.source_lesson_id===l.id).map(k=>k.input.content)].join('\n').includes(search))).sort((a,b)=>b.observation.business_date.localeCompare(a.observation.business_date)||b.created_at.localeCompare(a.created_at));
  const totals=validRange?teacherTotals(data.lessons,start,end,today):[];
  const draftName=(d:LessonDraft)=>{const a=d.request.action;if(a.kind!=='observation')return '整理或核对';const courseId=a.value.course_id;return data.catalog.find(c=>c.id===courseId)?.input.name??'未选课程';};
  const observe=draft?.request.action.kind==='observation'?draft.request.action.value:undefined;
  return <>
    <View style={{gap:14}}>
      {mode!=='drafts'&&<Action title="记录上课" secondary={false} disabled={!ready||!outbox.ready||busy} press={()=>open()} />}
      {interventionEntry}
      {error&&!editor&&<Text accessibilityRole="alert">{error}</Text>}{notice&&<Text role="status">{notice}</Text>}
      {confirmed&&<View style={{gap:10,padding:16,backgroundColor:'#FFFFFF',borderRadius:12}}><Text style={{fontSize:18,fontWeight:'600'}}>已记下：{confirmed.observation.business_date} · {confirmed.course.name} · {confirmed.teacher.name}</Text>
        <Action title="补充课堂观察" press={()=>open(confirmed,'observation')} /><Action title="完成" press={()=>setReceipt(null)} />
      </View>}
      {draftRecovery()}{Object.entries(drafts).filter(([,d])=>!outbox.value.entries[d.request.id]).map(([key,d])=><Action key={key} title={`继续课记：${draftName(d)}`} press={()=>{setEditor(key);setDiscard(false);setError('');}} />)}
      {mode==='today'?<>
        {!inboxManaged&&<>
        {data.lessons.filter(needsReview).length>0&&<><Text style={{fontSize:18,fontWeight:'600'}}>请家人查看</Text>{data.lessons.filter(needsReview).map(l=><View key={l.id} style={{padding:12,backgroundColor:'#FFFFFF',gap:8,borderRadius:10}}><Text>{l.request_name}请查看：{l.course.name} · {l.observation.business_date}</Text><Text>{l.request_message||'无留言，也可以直接看看观察。'}</Text><Action title={`整理：${l.course.name}`} press={()=>open(l,'organization')} /></View>)}</>}
        {data.actions.filter(a=>a.status==='pending'&&(!a.input.assignee_id||a.input.assignee_id===actorId)).map(a=><View key={a.input.id} style={{padding:12,backgroundColor:'#FFFFFF',gap:6}}><Text>待做：{a.input.content}{a.input.suggested_date?` · 建议${a.input.suggested_date}`:''}</Text><Text>负责人：{a.assignee_name??'家人待认领'}</Text>{a.source_deleted&&<Text>来源记录已删除或不可用，保留已保存的下一步。</Text>}{openAction&&<Action title={`维护下一步：${a.input.content}`} press={()=>openAction(a.input.id)}/>}</View>)}
        {data.actions.some(a=>a.status==='pending'&&a.input.assignee_id&&a.input.assignee_id!==actorId)&&<Action title={`其他家人的待做（${data.actions.filter(a=>a.status==='pending'&&a.input.assignee_id&&a.input.assignee_id!==actorId).length}）`} press={()=>setOtherTasks(v=>!v)}/>}
        {otherTasks&&data.actions.filter(a=>a.status==='pending'&&a.input.assignee_id&&a.input.assignee_id!==actorId).map(a=><View key={a.input.id} style={{gap:5}}><Text>{a.assignee_name}的下一步：{a.input.content}</Text>{openAction&&<Action title={`维护下一步：${a.input.content}`} press={()=>openAction(a.input.id)}/>}</View>)}
        </>}

      </>:mode==='records'?<>
        <View style={{flexDirection:'row',flexWrap:'wrap',gap:8}}><Action title="课堂记录" secondary={section!=='classroom'} press={()=>{setSection('classroom');setSelectedTeacher(null);}}/><Action title="课时核对" secondary={section!=='totals'} press={()=>{setSection('totals');setSelectedTeacher(null);}}/></View>
        <View style={{flexDirection:'row',justifyContent:'space-between',alignItems:'center',gap:8}}><Action title="上个月" press={()=>{setCustomRange(false);section==='totals'?setCountMonth(v=>shiftMonth(v,-1)):setMonth(v=>shiftMonth(v,-1));}}/><Text>{section==='totals'?countMonth:month}</Text><Action title="下个月" press={()=>{setCustomRange(false);section==='totals'?setCountMonth(v=>shiftMonth(v,1)):setMonth(v=>shiftMonth(v,1));}}/></View>
        {section==='totals'?<>
          <Action title={customRange?'收起自定义日期，按整月核对':'自定义课时核对日期'} press={()=>{if(!customRange){setRangeFrom(monthBounds.start);setRangeTo(monthBounds.end);}setCustomRange(v=>!v);setSelectedTeacher(null);}}/>
          {customRange&&<><DateField label="课时起始日期" value={rangeFrom} onChange={v=>{setRangeFrom(v);setSelectedTeacher(null);}}/><DateField label="课时截止日期" value={rangeTo} onChange={v=>{setRangeTo(v);setSelectedTeacher(null);}}/>{!validRange&&<Text>请完整填写有效起止日期，起始日期不能晚于截止日期。</Text>}</>}
          <Text>每条未撤销／删除的实际课记计一节，待核对仍计入。金额线下计算。</Text>
          {totals.map(group=><Action key={group.id} title={`${data.catalog.find(c=>c.id===group.id)?.input.name??group.name}：${group.count} 节${group.marked?` · 待核对 ${group.marked}`:''}`} press={()=>setSelectedTeacher(selectedTeacher===group.id?null:group.id)} />)}
          {validRange&&!totals.length&&<Text>{customRange?'这个时间段还没有上课记录。':'这个月还没有上课记录。'}</Text>}
          {totals.find(g=>g.id===selectedTeacher)?.records.map(l=><LessonCard key={l.id} lesson={l} open={()=>setDetail(l.id)}/>)}
        </>:<>
          <Field label="搜索课堂观察和老师建议" value={search} change={setSearch} multiline={false} placeholder="输入课程、老师或观察中的词"/><Check title="只看老师建议" checked={teachersOnly} press={()=>setTeachersOnly(!teachersOnly)}/>
          {[...new Set(records.map(l=>l.course.id))].map(courseId=><View key={courseId} style={{gap:10}}><Text style={{fontSize:18,fontWeight:'600'}}>{data.catalog.find(c=>c.id===courseId)?.input.name??records.find(l=>l.course.id===courseId)?.course.name}</Text>{records.filter(l=>l.course.id===courseId).map(l=><LessonCard key={l.id} lesson={l} matchSource={search||teachersOnly?(l.observation.teacher_notes.includes(search)&&Boolean(l.observation.teacher_notes)?'老师建议':observationText(l.observation).includes(search)?'课堂观察':'课程／老师或关联知识'):undefined} open={()=>setDetail(l.id)}/>)}</View>)}
          {!records.length&&<Text>这个月没有匹配的课记。</Text>}
        </>}
      </>:null}
    </View>
    {draft&&<DetailOverlay title={draft.phase==='organization'?'整理课堂记录':draft.phase==='administration'?'核对课记操作':'记录上课'} close={()=>{if(!busy)setEditor(null);}} footer={<><SaveStatus storageKey={storageKey} sharing={busy} pending={Boolean(draft.pending)}/><Action title={busy?'正在提交…':draft.pending?'重试同一次课记提交':draft.phase==='organization'?'保存整理':draft.phase==='administration'?'确认操作':'提交记录'} secondary={false} disabled={busy||(conflict&&!draft.pending)||Boolean(observe&&(!observe.course_id||!observe.teacher_id))} press={()=>void submit()}/></>}>
      {error&&<Text accessibilityRole="alert">{error}</Text>}
      {draftRecovery()}<OptionsUpdate changed={candidate.changed} refresh={candidate.refresh}/><Text style={{fontSize:14,color:'#53665A'}}>未提交内容留在本机，保存成功后家人才可查看。</Text>
      {conflict&&remote&&!draft.pending&&<View style={{padding:12,backgroundColor:'#FFFFFF',gap:10}}><Text>这一部分已有新版本，当前填写仍保留。</Text><ReadObservation value={remote.observation}/>
        <Action title="备份填写并载入最新课记" disabled={busy} press={()=>void(async()=>{try{await AsyncStorage.setItem(`${storageKey}:backup:${Date.now()}`,JSON.stringify(draft));const kind=draft.request.action.kind;const nextAction:LessonWrite['action']=kind==='observation'?{kind,value:{...remote.observation,request_review:false,review_message:''}}:kind==='organization'?{kind,value:organizationOf(remote)}:draft.request.action;await persist({...refs.current,[editor!]:{...draft,backup:draft.request,source:remote.observation,request:{...draft.request,expected_version:expected,operation_id:uuid(),action:nextAction}}});setError('');}catch(e){setError(note(e));}})()}/></View>}
      {draft.backup&&<View style={{gap:8}}><Text style={{fontWeight:'600'}}>此前填写的备份，可长按复制</Text>{draft.backup.action.kind==='observation'?<ReadObservation value={draft.backup.action.value}/>:draft.backup.action.kind==='organization'?<>{draft.backup.action.value.new_knowledge.map(k=><Text selectable key={k.id}>{k.content}</Text>)}{draft.backup.action.value.new_actions.map(a=><Text selectable key={a.id}>{a.content}</Text>)}{draft.backup.action.value.feedback?.map(f=><Text selectable key={f.id}>{labelsForFeedback(f.result)}：{f.observation}</Text>)}</>:null}</View>}
      {observe&&<>
        {intro&&draft.request.expected_version===null&&<View style={{gap:6}}><Text>先选课程和老师。</Text><Text>可以直接提交，也可以补充观察。</Text><Text>保存成功后家人就能看到。</Text><Action title="知道了，不再提示" press={()=>void AsyncStorage.setItem(storageKey+':introduced','1').then(()=>setIntro(false))}/></View>}
        <HelpButton/>
        <LessonDate value={observe.business_date} today={today} disabled={busy||Boolean(draft.pending)} change={business_date=>patch({business_date})}/>
        <CatalogChoice kind="course" value={observe.course_id} change={course_id=>patch({course_id})} data={data} accepted={catalogSaved} lesson={remote} disabled={busy||Boolean(draft.pending)}/>
        <CatalogChoice kind="teacher" value={observe.teacher_id} change={teacher_id=>patch({teacher_id})} data={data} accepted={catalogSaved} lesson={remote} disabled={busy||Boolean(draft.pending)}/>
        {active.filter(l=>l.id!==draft.request.id&&l.observation.business_date===observe.business_date&&l.course.id===observe.course_id&&l.teacher.id===observe.teacher_id).map(l=><Text key={l.id}>同日已有记录：{l.creator_name}，{new Date(l.created_at).toLocaleTimeString('zh-CN',{timeZone:'Asia/Shanghai'})}；确实是另一节仍可提交。</Text>)}
        {draft.phase==='basic'?<Action title="补充观察" disabled={busy||Boolean(draft.pending)} press={()=>void persist({...refs.current,[editor!]:{...draft,phase:'observation'}})}/>:<ObservationFields value={observe} change={patch} disabled={busy||Boolean(draft.pending)}/>}
      </>}
      {draft.request.action.kind==='organization'&&remote&&<OrganizationFields openKnowledge={openKnowledge} value={draft.request.action.value} change={patch} data={options} lesson={remote} source={draft.source} actorId={actorId} disabled={busy||Boolean(draft.pending)}/>}
      {draft.phase==='administration'&&<Text>{draft.request.action.kind==='withdraw'?'确认撤销这条课记？撤销后不计节数。':draft.request.action.kind==='delete'?'确认删除课记？需要管理员权限，已有派生引用时会阻止操作。':draft.request.action.kind==='acknowledge'?'告诉记录者“看到了，谢谢”，不会将待办标为已处理。':'标记记错会保留节数，待管理员核对。'}</Text>}

      {!draft.pending&&!busy&&<Action title="放弃这份本机草稿" press={()=>setDiscard(true)}/>}
      {discard&&!draft.pending&&!busy&&<View style={{gap:8}}><Text>只清除此处未提交的填写，已保存的课记保持原样。</Text><Action title="确认放弃未提交填写" press={()=>void(async()=>{const next={...refs.current};delete next[editor!];try{await persist(next);setEditor(null);setDiscard(false);}catch(e){setError(note(e));}})()}/><Action title="继续填写" press={()=>setDiscard(false)}/></View>}

    </DetailOverlay>}
    {selected&&<DetailOverlay title="课记详情" close={()=>setDetail(null)}>
      <Text style={{fontSize:18,fontWeight:'600'}}>{selected.observation.business_date} · {selected.course.name} · {selected.teacher.name}</Text><Text>记录者：{selected.creator_name}{selected.marked_error?' · 待核对':''}</Text><ReadObservation value={selected.observation}/><LessonPractice lesson={selected} data={data} openKnowledge={openKnowledge}/>
      {selected.organization.seen_observation_version>0&&selected.organization.seen_observation_version!==selected.observation_version&&<Text>原观察有后续补充，请核对先前整理内容。</Text>}
      <Action title="补充／修改课堂观察" press={()=>open(selected,'observation')}/><Action title="整理这条记录" press={()=>open(selected,'organization')}/>
      {selected.organization.claimed_name&&<Text>整理人：{selected.organization.claimed_name} · {selected.organization.state==='done'?'已处理':'整理中'}</Text>}{selected.organization.outcome&&<Text>处理说明：{selected.organization.outcome}</Text>}
      {selected.organization.targets.map(t=><View key={t.goal.id} style={{gap:6}}><Text>关联目标：{t.goal.input.title} · {t.target.note}</Text><Action title={`查看关联目标：${t.goal.input.title}`} press={()=>{setDetail(null);openGoal(t.goal.id);}}/></View>)}
      {selected.organization.methods.map(m=><Action key={m.id} title={`查看实际方法：${m.input.title}`} press={()=>{setDetail(null);openMethod(m.id);}}/>)}
      {data.knowledge.filter(k=>!k.deleted&&k.source_lesson_id===selected.id).map(k=><Text key={k.input.id}>{k.input.kind==='difficulty'?'已记困难':'已记经验'}：{k.input.content} · 依据观察第 {k.source_observation_version} 版</Text>)}
      {data.actions.filter(a=>a.source_lesson_id===selected.id).map(a=><Text key={a.input.id}>下一步：{a.input.content}（{({pending:'待做',in_plan:'已纳入计划',completed:'已完成',cancelled:'取消'} as Record<string,string>)[a.status]??a.status}）</Text>)}
      <Action title={historyOpen?'收起课记修改记录':'查看课记修改记录'} press={()=>setHistoryOpen(v=>!v)}/>
      {historyOpen&&<View style={{gap:10}}>{historyError&&<Text accessibilityRole="alert">{historyError}</Text>}{history.map(h=><View key={h.lesson.version} style={{gap:6}}><Text>{new Date(h.recorded_at).toLocaleString('zh-CN',{timeZone:'Asia/Shanghai'})} · {h.actor_name} · {({observation:'基础与观察',organization:'整理与关联',withdrawn:'撤销',deleted:'管理员删除',marked_error:'待核对标记',acknowledged:'回应'} as Record<string,string>)[h.event]??h.event}</Text><Text>观察第 {h.lesson.observation_version} 版 · 整理第 {h.lesson.organization_version} 版</Text><ReadObservation value={h.lesson.observation}/></View>)}{historyMore&&<Action title='更早的课记版本' disabled={historyLoading} press={()=>void loadLessonHistory(selected.id,historyPage+1)}/>}</View>}
      <Text>看到了：{selected.acknowledgements.length} 位家人。轻量回应不等于已整理。</Text>
      <Action title={selected.acknowledgements.includes(actorId)?'已回应：看到了，谢谢':'看到了，谢谢'} disabled={selected.acknowledgements.includes(actorId)} press={()=>perform(selected,'acknowledge')}/>
      <Action title="标记记错，请管理员核对" press={()=>perform(selected,'mark_error')}/>
      {selected.creator_id===actorId&&localDateOf(selected.created_at)===today&&<Action title="撤销这条课记" press={()=>perform(selected,'withdraw')}/>}
      {admin&&<><Action title="删除课记（管理员）" press={()=>perform(selected,'delete')}/>{selected.marked_error&&<Action title="取消待核对标记（管理员）" press={()=>perform(selected,'mark_error',false)}/>}</>}
    </DetailOverlay>}
  </>;
}
function LessonDate({value,today,disabled,change}:{value:string;today:string;disabled:boolean;change:(v:string)=>void}) {
  const yesterday=new Date(Date.parse(today+'T00:00:00Z')-86400000).toISOString().slice(0,10);
  const [other,setOther]=useState(value!==today&&value!==yesterday);
  return <View style={{gap:8}}><Text>上课日期：{value}</Text><View style={{flexDirection:'row',flexWrap:'wrap',gap:8}}>
    {[['今天',today],['昨天',yesterday]].map(([label,date])=><Action key={label} title={label} disabled={disabled} secondary={value!==date||other} press={()=>{setOther(false);change(date);}}/>)}
    <Action title="其他日期" disabled={disabled} secondary={!other} press={()=>setOther(true)}/></View>
    {other&&<DateField label="上课日期" value={value} disabled={disabled} onChange={change}/>}</View>;
}
function LessonCard({lesson:l,open,matchSource}:{lesson:Lesson;open:()=>void;matchSource?:string}) {return <View style={{gap:8,padding:14,borderRadius:12,backgroundColor:'#FFFFFF'}}><Text>{l.observation.business_date} · {l.course.name} · {l.teacher.name}</Text>{matchSource&&<Text>匹配来源：{matchSource}</Text>}<Text>记录者：{l.creator_name}{l.marked_error?' · 待核对':''}</Text><Text>{l.observation.notes||l.observation.teacher_notes||'尚未补充观察'}</Text><Action title={`查看课记：${l.course.name} · ${l.observation.business_date}`} press={open}/></View>;}

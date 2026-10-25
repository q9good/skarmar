import {authSessionGuard} from './api';
import {resetAfterFailure} from './pending-request';
import {DraftRecovery} from './draft-recovery';
import {createDraftStore} from './draft-store-native';
import type {DraftStore} from './draft-store';
import {ObjectContext} from './object-context-view';
import {MethodUsage} from './method-usage-view';
import {useStableOptions,OptionsUpdate} from './stable-options';
import {RelationPicker} from './relation-picker';
import {GuidedField} from './guided-field';
import {knowledgeForGoal,knowledgeRelation,knowledgeSource} from './knowledge-relations';
import {coverageLegend} from './prd-coverage';
import {SaveStatus,trackLocalSave} from './save-status';
import {ExampleHint} from './input-guidance';
import {HelpButton} from './product-help';
import {ObservationHub} from './prd-observation-view';
import {prioritizeMethods} from './prd-history';
import type {Coverage} from './prd-sessions';
import {HistoryPanel} from './prd-history-view';
import {measurementSeries} from './prd-progress';
import type {EvidenceRecord} from './prd-observations';
import {TodayRecords} from './prd-today-view';
import {businessDateLabel} from './prd-today';
import {LessonOutboxProvider,LessonQueueStatus} from './prd-outbox-view';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { ActivityIndicator, AppState, Platform, ScrollView, StyleSheet, View } from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { AccountPanel, useAuth } from './auth';
import { ApiError, request } from './api';
import { DateField, Select } from './controls';
import {localDate, uuid } from './draft';
import { DetailOverlay } from './overlay';
import {FollowupHub} from './prd-followup-view';
import {SessionHub} from './prd-session-view';
import { LessonHub } from './prd-lesson-view';
import { Pressable, Text, TextInput } from './readability';
import { acceptSaved, decodeDrafts, emptySnapshot, examples, findSaved, goalStateName, levelNames, mergeSnapshot, mutationOf,
  nameOf, newGoal, newMethod, type Draft, type GoalInput, type MethodInput, type Mutation, type Revision, type Saved, type Snapshot, type Write } from './foundation';
const messageOf = (e: unknown) => e instanceof Error ? e.message : '保存未完成，请保留填写后重试。';
function Action({ title, press, disabled = false, secondary = false }: { title: string; press: () => void; disabled?: boolean; secondary?: boolean }) {
  return <Pressable accessibilityRole="button" accessibilityState={{ disabled }} disabled={disabled} onPress={press} style={[s.action, secondary && s.secondary, disabled && { opacity: 0.5 }]}>
    <Text style={{ color: secondary ? '#34765B' : '#FFFFFF', fontSize: 17, textAlign: 'center' }}>{title}</Text></Pressable>;
}
function Field({ label, hint, value, change, placeholder, disabled, multiline = false }: { label: string; hint?: string; value: string; change: (v: string) => void; placeholder?: string; disabled?: boolean; multiline?: boolean }) {return <GuidedField label={label} value={value} change={change} question={hint} placeholder={placeholder} disabled={disabled} multiline={multiline}/>;}
function Check({ label, checked, change, disabled }: { label: string; checked: boolean; change: () => void; disabled?: boolean }) {
  return <Pressable accessibilityRole="checkbox" aria-checked={checked} accessibilityState={{ checked, disabled }} accessibilityLabel={label} onPress={change} disabled={disabled} style={[s.secondary, { padding: 10, borderRadius: 8 }]}>
    <Text>{checked ? '☑' : '☐'} {label}</Text></Pressable>;
}
function GoalFields({ value, change, data, id, disabled, existing }: { value: GoalInput; change: (patch: Partial<GoalInput>) => void; data: Snapshot; id: string; disabled: boolean; existing: boolean }) {
  const [more, setMore] = useState(existing);
  const rank = { long: 0, medium: 1, short: 2 };
  return <>
    <Field label="本领名称" hint="想帮助孩子做到什么？" value={value.title} change={title => change({ title })} placeholder="例如：自己用筷子" disabled={disabled} />
    <Select label="目标类型" value={value.level} disabled={disabled || existing} options={Object.entries(levelNames).map(([value, label]) => ({ value, label }))}
      onChange={level => change({ level: level as GoalInput['level'], parent_id: null, state: 'active' })} />
    <Action title={more ? '收起目标补充信息' : '补充目标说明（可选）'} secondary press={() => setMore(!more)} />
    {more && <>
      <Field label="生活意义" hint="这会帮助哪方面生活？" value={value.meaning} change={meaning => change({ meaning })} placeholder="例如：更独立地吃饭" disabled={disabled} multiline />
      <Select label="观察方式" value={value.template ?? 'text'} disabled={disabled} options={[
        { value: 'text', label: '直接写文字' }, { value: 'count', label: '次数／机会' }, { value: 'duration', label: '时长／数量' }, { value: 'steps', label: '步骤／描述' }]}
        onChange={template => change({ template: template as GoalInput['template'] })} />
      <Field label="观察标准" hint="怎样知道有进步？" value={value.criteria} change={criteria => change({ criteria })} placeholder={examples[value.template ?? 'text']} disabled={disabled} multiline />
      <Select label="上级目标" value={value.parent_id ?? ''} disabled={disabled} options={[{ value: '', label: '不关联上级' }, ...data.goals.filter(g => g.id !== id && rank[g.input.level] < rank[value.level]).map(g => ({ value: g.id, label: g.input.title }))]}
        onChange={parent_id => change({ parent_id: parent_id || null })} />
      <Select label="目标状态" value={value.state ?? 'active'} disabled={disabled} options={(value.level === 'long' ? ['active', 'maintaining', 'paused'] : ['not_started', 'active', 'maintaining', 'paused', 'achieved']).map(state => ({ value: state, label: goalStateName({ ...value, state: state as GoalInput['state'] }) }))}
        onChange={state => change({ state: state as GoalInput['state'],resume_on:state==='paused'?value.resume_on??null:null })} />
      {value.state==='paused'&&<DateField label="确认目标是否恢复（留空不排期）" value={value.resume_on??''} onChange={v=>change({resume_on:v||null})} disabled={disabled}/>}
      <Select label="关注节奏" value={value.frequency ?? ''} disabled={disabled} options={[{ value: '', label: '继承状态默认' }, { value: 'weekly', label: '每周' }, { value: 'fortnightly', label: '每两周' }, { value: 'monthly', label: '每月' }, { value: 'none', label: '不设节奏' }]}
        onChange={frequency => change({ frequency: frequency ? frequency as GoalInput['frequency'] : null })} />
      <DateField label="开始关注日期（可选）" value={value.attention_start ?? ''} disabled={disabled} onChange={date => change({ attention_start: date || null })} />
      <Check label="从默认列表收起" checked={value.archived} disabled={disabled} change={() => change({ archived: !value.archived })} />
    </>}
  </>;
}
function MethodFields({ value, change, data, disabled }: { value: MethodInput; change: (patch: Partial<MethodInput>) => void; data: Snapshot; disabled: boolean }) {
  return <>
    <Field label="方法名称" hint="准备用什么做法支持孩子？" value={value.title} change={title => change({ title })} placeholder="例如：先示范，再留出等待时间" disabled={disabled} />
    <Text style={s.label}>关联目标</Text>
    {data.goals.length?<RelationPicker label="方法适用目标" options={data.goals.map(g=>({id:g.id,label:g.input.title}))} selected={value.goal_ids} disabled={disabled} toggle={id=>change({goal_ids:value.goal_ids.includes(id)?value.goal_ids.filter(v=>v!==id):[...value.goal_ids,id]})}/>:<Text>先在目标页写下一个近期本领，再关联方法。</Text>}

    <Field label="具体做法（可后补）" value={value.description} change={description => change({ description })} placeholder="例如：先跟随孩子的玩法，再增加一个小步骤" multiline disabled={disabled} />
    <Field label="适用条件与步骤（可选）" value={value.steps} change={steps => change({ steps })} placeholder="例如：在熟悉的游戏中，孩子愿意继续时尝试" multiline disabled={disabled} />
    <Field label="指导来源（可选）" value={value.guidance} change={guidance => change({ guidance })} placeholder="例如：老师示范，家里试过几次" disabled={disabled} />
    <Select label="方法状态" value={value.state ?? 'active'} disabled={disabled} options={[{ value: 'active', label: '使用中' }, { value: 'paused', label: '暂停' }, { value: 'retired', label: '停用' }]}
      onChange={state => change({ state: state as MethodInput['state'],resume_on:state==='paused'?value.resume_on??null:null })} />
    {value.state==='paused'&&<DateField label="确认方法是否恢复（留空不排期）" value={value.resume_on??''} onChange={v=>change({resume_on:v||null})} disabled={disabled}/>}
    <Check label="手动置顶" checked={value.pinned} disabled={disabled} change={() => change({ pinned: !value.pinned })} />
    <Check label="需要注意" checked={value.needs_attention} disabled={disabled} change={() => change({ needs_attention: !value.needs_attention })} />
    {value.needs_attention && <Field label="需要注意的原因" value={value.attention_reason} change={attention_reason => change({ attention_reason })} multiline disabled={disabled} />}
  </>;
}
function ReadOnlyValues({ mutation, data }: { mutation: Mutation; data: Snapshot }) {
  const values: [string, string][] = mutation.kind === 'goal' ? [
    ['名称', mutation.value.title], ['生活意义', mutation.value.meaning], ['观察标准', mutation.value.criteria],
    ['观察方式', ({count:'次数／机会',duration:'时长／数量',steps:'步骤／描述',text:'直接写文字'}[mutation.value.template ?? 'text'])],
    ['关注节奏', mutation.value.frequency ? ({weekly:'每周',fortnightly:'每两周',monthly:'每月',none:'不设节奏'}[mutation.value.frequency]) : '继承状态默认'],
    ['状态', goalStateName(mutation.value)], ['收起', mutation.value.archived ? '是' : '否'],
    ['上级目标', data.goals.find(g => g.id === mutation.value.parent_id)?.input.title ?? '未关联'],
    ['开始关注', mutation.value.attention_start ?? '未设置'],
  ] : mutation.kind === 'method' ? [
    ['名称', mutation.value.title], ['具体做法', mutation.value.description], ['适用条件与步骤', mutation.value.steps],
    ['指导来源', mutation.value.guidance], ['关联目标', mutation.value.goal_ids.map(id => data.goals.find(g => g.id === id)?.input.title ?? '待核对目标').join('、')],
    ['方法状态', ({active:'使用中',paused:'暂停',retired:'停用'}[mutation.value.state ?? 'active'])],
    ['手动置顶', mutation.value.pinned ? '是' : '否'], ['需要注意', mutation.value.needs_attention ? '是' : '否'],
    ['需要注意的原因', mutation.value.attention_reason],
  ] : [['名称', mutation.value.name], ['可供选择', mutation.value.active ? '是' : '否']];
  return <View style={{ gap: 8 }}>{values.map(([label, value]) => <Text selectable key={label}>{label}：{value || '未填写'}</Text>)}</View>;
}
export function FoundationWorkspace() {
 const draftStore=useRef<DraftStore<Draft>|null>(null);
  const auth = useAuth();
  const scope = `${auth.identity.space}:${auth.identity.user.id}`;
  const key = `skarmar:${scope}:prd-drafts:v1`;
  const [data, setData] = useState<Snapshot>(emptySnapshot);
  const [todayMoments,setTodayMoments]=useState<EvidenceRecord[]>([]);
  const reportTodayMoments=useCallback((records:EvidenceRecord[])=>setTodayMoments(records),[]);
  const [goalLevel,setGoalLevel]=useState('all'),[goalView,setGoalView]=useState('coverage'),[progressNonce,setProgressNonce]=useState(0),[progress,setProgress]=useState<{as_of:string;goals:{id:string;measurement_count:number;segment_count:number;latest:EvidenceRecord[]}[]}|null>(null),[progressError,setProgressError]=useState('');
  const progressSequence=useRef(0);
  const [goalSearch,setGoalSearch]=useState(''),[methodSearch,setMethodSearch]=useState(''),[goalSource,setGoalSource]=useState<'family'|'lesson'|'combined'>('family'),[goalDays,setGoalDays]=useState(30),[goalReport,setGoalReport]=useState<Coverage|null>(null);
  const goalSequence=useRef(0);
  const rankingData=useRef(data);rankingData.current=data;
  const [methodOrder,setMethodOrder]=useState<ReturnType<typeof prioritizeMethods>>([]),[rankingBusy,setRankingBusy]=useState(false),[rankingError,setRankingError]=useState('');
  const rankingSequence=useRef(0);
  const loadMethodOrder=useCallback(async()=>{
    const seq=++rankingSequence.current;setRankingBusy(true);setRankingError('');
    try{const report=await request<Coverage>('/prd/coverage?days=30&source=family');
      if(seq===rankingSequence.current)setMethodOrder(prioritizeMethods(rankingData.current,report.goals.filter(g=>g.ratio!==null&&g.ratio>1).map(g=>g.id)));
    }catch{if(seq===rankingSequence.current){setRankingError('近况暂未读到，先按置顶、困难及状态排列。');setMethodOrder(prioritizeMethods(rankingData.current,[]));}}
    finally{if(seq===rankingSequence.current)setRankingBusy(false);}
  },[]);
  const [tab, setTab] = useState<'today' | 'goals' | 'methods' | 'records' | 'followups' | 'catalog'>('today');
  useEffect(()=>{if(tab==='methods')void loadMethodOrder();return()=>{++rankingSequence.current;};},[tab,loadMethodOrder]);
  useEffect(()=>{if(tab!=='goals')return;const seq=++goalSequence.current;setGoalReport(null);void request<Coverage>(`/prd/coverage?days=${goalDays}&source=${goalSource}`).then(r=>{if(seq===goalSequence.current)setGoalReport(r);}).catch(()=>undefined);return()=>{++goalSequence.current;};},[tab,goalSource,goalDays,progressNonce]);
  useEffect(()=>{if(tab!=='goals'||goalView!=='progress')return;const seq=++progressSequence.current;setProgressError('');void request<typeof progress>('/prd/process?view=goal_progress').then(v=>{if(seq===progressSequence.current)setProgress(v);}).catch(()=>{if(seq===progressSequence.current)setProgressError('实际测量更新失败，请刷新重试；已显示的是此前读取的内容，未读到不表示没有测量。');});return()=>{++progressSequence.current;};},[tab,goalView,progressNonce]);
  const [recordKind,setRecordKind]=useState<'lessons'|'family'>('lessons');
  const [requestedObservation,setRequestedObservation]=useState<{kind:'goal'|'needs'|'moment';id:string;nonce:string}|null>(null);
  const [requestedFollowup,setRequestedFollowup]=useState<{kind:'knowledge'|'review'|'action'|'new_experience';id:string;object_kind?:'goal'|'method'|'knowledge';nonce:string}|null>(null);
  const [requestedLesson,setRequestedLesson]=useState<{id:string;nonce:string;view?:'detail'|'organize'}|null>(null);
  const [requestedPlanActions,setRequestedPlanActions]=useState<{ids:string[];nonce:string}|null>(null);
  const [requestedSession,setRequestedSession]=useState<{id:string;nonce:string}|null>(null);
  const [account, setAccount] = useState(false);
  const [editor, setEditor] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<Record<string, Draft>>({});
  const [ready, setReady] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [syncError, setSyncError] = useState('');
  const [busy, setBusy] = useState(false);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [history, setHistory] = useState<Revision[]>([]);
  const [historyPage,setHistoryPage]=useState(0),[historyMore,setHistoryMore]=useState(false),[historyLoading,setHistoryLoading]=useState(false);
  const [distribution,setDistribution]=useState<{id:string;kind:'goal'|'method';version?:number}|null>(null);
  const [showDirections, setShowDirections] = useState(false);
  const [showHidden, setShowHidden] = useState(false);
  const detailSequence = useRef(0);
  const scroll = useRef<ScrollView>(null);
  useEffect(() => { scroll.current?.scrollTo({ y: 0, animated: false }); }, [tab, account]);
  const draftsRef = useRef(drafts), persistence = useRef(Promise.resolve()), alive = useRef(true), submitting = useRef(false);
  const refresh = useCallback(async () => {
    const next = await request<Snapshot>('/prd/state');
    if (next.schema !== 'skarmar.prd.v1') throw new Error('页面与服务版本不匹配，请保留填写后刷新。');
    if (alive.current) { setData(current => mergeSnapshot(current, next)); setSyncError(''); }
  }, []);
  useEffect(() => {
    alive.current = true;
    void refresh().catch(e => setSyncError(messageOf(e)));
    void AsyncStorage.getItem(key).then(raw => {
      const parsed = decodeDrafts(raw);
      if (alive.current) { draftStore.current=createDraftStore(key,decodeDrafts,parsed); draftsRef.current = parsed as Record<string, Draft>; setDrafts(draftsRef.current); setReady(true); }
    }).catch(e => setError(messageOf(e)));
    const sync = () => { if (Platform.OS !== 'web' || document.visibilityState === 'visible') void refresh().catch(e => { if (alive.current) setSyncError(messageOf(e)); }); };
    const timer = setInterval(sync, 20000);
    const native = AppState.addEventListener('change', value => { if (value === 'active') sync(); });
    if (Platform.OS === 'web') { window.addEventListener('focus', sync); document.addEventListener('visibilitychange', sync); }
    return () => { alive.current = false; clearInterval(timer); native.remove(); if (Platform.OS === 'web') { window.removeEventListener('focus', sync); document.removeEventListener('visibilitychange', sync); } };
  }, [key, refresh]);
  const persist = (next: Record<string, Draft>) => {
    draftsRef.current = next; setDrafts(next);
    const task = draftStore.current!.save(next);
    persistence.current = task;trackLocalSave(key,task);
    void task.catch(e => {if(alive.current)setError(messageOf(e));});
    return task;
  };
 const draftRecovery=()=> <DraftRecovery store={draftStore.current} scope={key} label="目标方法" adopted={rows=>{draftsRef.current=rows;setDrafts(rows);setError('');}} restored={(entry,value)=>{const rows={...draftsRef.current,[entry]:value};draftsRef.current=rows;setDrafts(rows);setEditor(entry);setError('');}}/>;
  const begin = (mutation: Mutation, saved?: Saved) => {
    if (!ready) return;
    const id = saved?.value.id ?? uuid();
    if (!draftsRef.current[id]) void persist({ ...draftsRef.current, [id]: { id, operation_id: uuid(), expected_version: saved?.value.version ?? null, change_note: '', mutation } });
    setEditor(id); setError(''); setNotice('');
  };
  const draft = editor ? drafts[editor] : undefined;
  const latest = draft ? findSaved(data, draft.id) : undefined;
  const conflict = Boolean(draft && latest && draft.expected_version !== latest.value.version);
  const candidate=useStableOptions(data,editor,draft?.mutation.kind==='method'?draft.mutation.value.goal_ids:[]);
  const patch = (value: Record<string, unknown>) => {
    if (!draft || draft.pending || busy) return;
    void persist({ ...draftsRef.current, [draft.id]: { ...draft, operation_id: uuid(), mutation: { ...draft.mutation, value: { ...draft.mutation.value, ...value } } as Mutation } });
  };
  const submit = async () => {
    if (!draft || (conflict && !draft.pending) || submitting.current) return;
    submitting.current = true; setBusy(true); setError('');
    const { pending: _pending, conflict_backup: _backup, ...body } = draft;
    const payload = draft.pending ?? body;
    const assertSession=authSessionGuard();
    let sent = false;
    try {
      await persist({ ...draftsRef.current, [draft.id]: { ...draft, pending: payload } });
      assertSession();sent = true;
      const saved = await request<Saved>(`/prd/records/${draft.id}`, 'PUT', payload);
      setData(current => acceptSaved(current, saved));
      const next = { ...draftsRef.current }; delete next[draft.id];
      try { await persist(next); } catch { setError('记录已保存，但本机草稿暂未清理；请保留浏览器数据后重试。'); }
      setEditor(null); setNotice('已保存，其他家人刷新即可看到。');
      try { await refresh(); } catch { setSyncError('记录已保存，列表暂未刷新。'); }
    } catch (e) {
      if (resetAfterFailure(Boolean(draft.pending),sent,e)) {
        void persist({ ...draftsRef.current, [draft.id]: { ...draft, operation_id: uuid(), pending: undefined } });
        if (e instanceof ApiError && e.status === 409) await refresh().catch(() => undefined);
      }
      setError(sent || e instanceof ApiError ? messageOf(e) : draft.pending ? '本机存储未完成；上次提交结果仍待确认，请保留填写后重试。' : '本机存储未完成，尚未发送，请保留页面后重试。');
    } finally { submitting.current = false; setBusy(false); }
  };
  const loadObjectHistory=async(id:string,page=0)=>{const sequence=++detailSequence.current;setHistoryLoading(true);try{const result=await request<{items:Revision[];has_more:boolean}>(`/prd/process?view=entity_history&id=${id}&page=${page}`);if(alive.current&&sequence===detailSequence.current){setHistory(old=>page?[...old,...result.items].filter((r,i,all)=>all.findIndex(v=>v.entity.value.version===r.entity.value.version)===i):result.items);setHistoryPage(page);setHistoryMore(result.has_more);}}catch(e){setError(messageOf(e));}finally{if(sequence===detailSequence.current)setHistoryLoading(false);}};
  const openDetail = async (id:string)=>{setDetailId(id);setHistory([]);setHistoryMore(false);await loadObjectHistory(id);};
  const detail = detailId ? findSaved(data, detailId) : undefined;
  const relatedKnowledge = (goalId?: string, methodId?: string) => goalId?knowledgeForGoal(data,goalId).map(r=>r.knowledge):data.knowledge.filter(k=>!k.deleted&&!!methodId&&k.input.method_ids.includes(methodId));
  const knowledgeCard=(k:Snapshot['knowledge'][number],goalId?:string,methodId?:string)=><View key={k.input.id} style={{gap:6}}><Text>{k.input.kind==='difficulty'?'关联困难':'关联经验'}：{k.input.content}</Text><Text style={s.hint}>{knowledgeRelation(data,k,goalId,methodId)}</Text><Text style={s.hint}>{knowledgeSource(data,k)}</Text><Action title={`查看关联知识：${k.details?.title||k.input.content}`} secondary press={()=>setRequestedFollowup({kind:'knowledge',id:k.input.id,nonce:uuid()})}/></View>;
  const today=localDate();
  const goals = data.goals.filter(g => (goalLevel==='all'||g.input.level===goalLevel)&&(showHidden || !g.input.archived)&&[g.input.title,g.input.meaning,g.input.criteria].some(v=>v.includes(goalSearch)));
  const longDirections = data.goals.filter(g => g.input.level === 'long' && !g.input.archived);
  const methods = [...data.methods].filter(m => (showHidden || m.input.state === 'active')&&[m.input.title,m.input.description,m.input.steps].some(v=>v.includes(methodSearch))).sort((a,b)=>(methodOrder.findIndex(r=>r.id===a.id)<0?999:methodOrder.findIndex(r=>r.id===a.id))-(methodOrder.findIndex(r=>r.id===b.id)<0?999:methodOrder.findIndex(r=>r.id===b.id)));
  const followupHub=(mode:'today'|'full'|'hidden')=><FollowupHub planActions={ids=>setRequestedPlanActions({ids,nonce:uuid()})} data={data} mode={mode} openLesson={id=>void refresh().then(()=>setRequestedLesson({id,nonce:uuid()})).catch(e=>setSyncError(messageOf(e)))} refresh={refresh} adjustMethod={(mid,kid,version)=>{const method=data.methods.find(m=>m.id===mid);if(!method)return;if(draftsRef.current[mid]){setEditor(mid);setError('先核对已有方法填写，再根据这条知识调整。');return;}begin({kind:'method',value:{...method.input,basis:[...(method.input.basis??[]).filter(b=>b.id!==kid),{id:kid,version}]}},{kind:'method',value:method});}} requested={requestedFollowup} onRequested={()=>setRequestedFollowup(null)}/>;
  const todayRecords=<TodayRecords data={data} moments={todayMoments} today={today} open={(kind,id)=>kind==='lesson'?setRequestedLesson({id,nonce:uuid(),view:'detail'}):setRequestedSession({id,nonce:uuid()})}/>;
  const sessionHub=(mode:'today'|'records'|'drafts')=><SessionHub data={data} mode={mode} afterPrimary={mode==='today'?todayRecords:undefined} refresh={refresh} accepted={session=>setData(current=>mergeSnapshot(current,{...emptySnapshot,sessions:[session]}))} acceptMethod={saved=>setData(current=>acceptSaved(current,saved))} openGoal={id=>void openDetail(id)} openAction={id=>setRequestedFollowup({kind:'action',id,nonce:uuid()})} requested={requestedSession} onRequested={()=>setRequestedSession(null)} requestedActions={requestedPlanActions} onRequestedActions={()=>setRequestedPlanActions(null)}/>;
  return <LessonOutboxProvider data={data} cacheLoaded={snapshot=>setData(current=>mergeSnapshot(snapshot,current))} accepted={receipt=>setData(current=>'kind' in receipt?acceptSaved(current,receipt):mergeSnapshot(current,{...emptySnapshot,lessons:[receipt]}))}><View style={s.root}>
    <View style={s.header}><Text style={s.brand}>✦ skarmar</Text><Action title="我的账号" secondary press={() => setAccount(true)} /></View>
    <ScrollView ref={scroll} contentContainerStyle={s.content} keyboardShouldPersistTaps="handled">
      {account ? <><AccountPanel businessTools={false} prdExport back={() => setAccount(false)} onImported={refresh} logout={async () => { await persistence.current; await auth.logout(); }} /><HelpButton title="使用说明与手机桌面入口"/><Action title="共享需要与帮助说明" secondary press={()=>setRequestedObservation({kind:'needs',id:'',nonce:uuid()})}/><Action title="维护课程和老师选项" secondary press={() => { setAccount(false); setTab('catalog'); }} /></> : <>
        <View style={s.header}><Text accessibilityRole="header" style={s.title}>{{ today: '今天', goals: '目标', methods: '方法', records: '记录', followups: '跟进', catalog: '课程和老师' }[tab]}</Text>{tab==='today'&&<Text accessibilityLabel="家庭业务日期" style={s.hint}>{businessDateLabel(today)}</Text>}</View>
        <LessonQueueStatus/>
        {syncError ? <View style={s.card}><Text accessibilityRole="alert">{syncError}</Text><Action title="重试同步" secondary press={() => {setProgressNonce(n=>n+1);void refresh().catch(e => setSyncError(messageOf(e)));}} /></View> : null}
        {notice ? <Text role="status">{notice}</Text> : null}
        {!ready ? <ActivityIndicator accessibilityLabel="正在读取本机草稿" /> : <>
          {draftRecovery()}
          {Object.values(drafts).map(d => <View key={d.id} style={s.card}><Text>继续填写：{nameOf(d.mutation) || '尚未命名'}</Text><Action title={`继续填写：${nameOf(d.mutation) || '尚未命名'}`} secondary press={() => setEditor(d.id)} /></View>)}
          {tab === 'today' || tab === 'records' ? <>
            {tab === 'today' && <Pressable accessibilityRole="button" accessibilityLabel="查看长期关注" onPress={() => setShowDirections(true)} style={{padding:10,borderRadius:10,backgroundColor:'#E9F2E9',flexDirection:'row',alignItems:'center',gap:8}}>
              <Text numberOfLines={1} style={{flex:1,fontSize:14}}>{longDirections.length ? `长期关注 · ${longDirections.slice(0,3).map(g=>g.input.title).join(' · ')}` : '先写下想长期关注的方向'}</Text><Text style={{fontSize:14}}>{longDirections.length>3?`另有 ${longDirections.length-3} 项 ▾`:'▾'}</Text>
            </Pressable>}
            {tab==='records'&&<View style={{flexDirection:'row',gap:8}}><Action title="机构课程" secondary={recordKind!=='lessons'} press={()=>setRecordKind('lessons')}/><Action title="家庭干预" secondary={recordKind!=='family'} press={()=>setRecordKind('family')}/></View>}
            {tab==='records'&&recordKind==='family'?sessionHub('records'):<LessonHub openKnowledge={id=>setRequestedFollowup({kind:'knowledge',id,nonce:uuid()})} inboxManaged requested={requestedLesson} onRequested={()=>setRequestedLesson(null)} openAction={id=>setRequestedFollowup({kind:'action',id,nonce:uuid()})} interventionEntry={tab==='today'?<>{sessionHub('today')}{followupHub('today')}</>:undefined} data={data} actorId={auth.identity.user.id} actorName={auth.identity.user.display_name} admin={auth.identity.user.role === 'admin'} mode={tab === 'today' ? 'today' : 'records'} refresh={refresh}
              savedLesson={lesson => setData(current => mergeSnapshot(current, { ...emptySnapshot, lessons: [lesson] }))}
              catalogSaved={saved => setData(current => acceptSaved(current, saved))} openGoal={id => void openDetail(id)} openMethod={id => void openDetail(id)} scrollTop={() => scroll.current?.scrollTo({y:0,animated:false})} />}
          </> : tab === 'followups' ? <>
            {followupHub('full')}
          </> : tab === 'goals' ? <>
            <Field label="搜索目标" value={goalSearch} change={setGoalSearch}/><Select label="目标期限筛选" value={goalLevel} onChange={setGoalLevel} options={[{value:'all',label:'全部期限'},...Object.entries(levelNames).map(([value,label])=>({value,label}))]}/><Select label="目标展示" value={goalView} onChange={setGoalView} options={[{value:'coverage',label:'记录覆盖'},{value:'progress',label:'实际进展（测量）'}]}/>{goalView==='coverage'?<><Select label="目标覆盖来源" value={goalSource} onChange={v=>setGoalSource(v as typeof goalSource)} options={[{value:'family',label:'家庭干预'},{value:'lesson',label:'机构课程'},{value:'combined',label:'合计（分列来源）'}]}/><Select label="目标覆盖窗口" value={String(goalDays)} onChange={v=>setGoalDays(Number(v))} options={[7,30,90].map(n=>({value:String(n),label:`近${n}天`}))}/><Text>近{goalDays}天固定色阶：{coverageLegend(goalDays)}；不代表能力进步。{goalReport?`${goalReport.start} 至 ${goalReport.end}`:'正在读取覆盖'}，数字和来源同时显示。</Text></>:<><Text>实际进展来自各指标测量，不按覆盖次数计算能力百分比。这里显示各条件下最近测量；完整趋势在目标详情中查看。</Text>{progressError&&<Text accessibilityRole="alert">{progressError}</Text>}{!progress&&!progressError&&<Text>正在读取实际测量…</Text>}</>}
            <Action title="新建目标" press={() => begin({ kind: 'goal', value: newGoal() })} />
            <Check label="显示已收起目标" checked={showHidden} change={() => setShowHidden(!showHidden)} />
            {!goals.length && <Text>写下想帮助孩子学会的本领，或者希望长期支持的方向。</Text>}
            {goals.map(g => <View key={g.id} style={s.card} testID={`prd-goal-${g.id}`}>
              <Text style={s.heading}>{g.input.title}</Text>{goalView==='coverage'&&g.input.level!=='long'&&goalReport&&(()=>{const row=goalReport.goals.find(r=>r.id===g.id);return row?<View style={{padding:10,borderRadius:8,backgroundColor:['#F3F6EF','#E4EEDC','#CADDB9','#94B681','#4E7656'][row.band]}}><Text style={{color:row.band===4?'#FFFFFF':'#243D36'}}>近{goalDays}天 {row.counts.selected}次覆盖 · 家庭{row.counts.family}／机构{row.counts.lesson}</Text></View>:null;})()}<Text>{levelNames[g.input.level]} · {goalStateName(g.input)}{g.input.archived ? ' · 已收起' : ''}</Text>
              <Text>{g.input.meaning || '尚未补充生活意义'}</Text>
              {goalView==='progress'&&(()=>{const p=progress?.goals.find(row=>row.id===g.id);return <View style={{gap:7,padding:10,backgroundColor:'#F3F6EF'}}>{p?<><Text>{p.measurement_count}条实际测量 · {p.segment_count}组指标与条件；截至{progress?.as_of}</Text>{measurementSeries(p.latest,localDate()).map(series=><Text key={series.key}>{series.name} · 指标第{series.metricVersion}版／目标第{series.goalVersion}版 · {series.sourceName} · {series.methodBasis} · {series.support||'帮助条件未记录'}：{series.points.at(-1)!.date}，{series.points.at(-1)!.label}{p.latest.find(r=>r.id===series.points.at(-1)!.id)?.source_deleted?'（来源已撤销或删除，保留当时测量）':''}</Text>)}{p.segment_count>3&&<Text>这里只列最近3组；其他组请打开完整趋势。</Text>}</>:<Text>{progressError?'实际测量暂未读到，请刷新重试。':!progress?'正在读取实际测量…':'待观察：尚无实际测量，不用训练次数代替进步。'}</Text>}<Action title={`查看测量趋势：${g.input.title}`} secondary press={()=>setRequestedObservation({kind:'goal',id:g.id,nonce:uuid()})}/></View>;})()}

              {relatedKnowledge(g.id).map(k=>knowledgeCard(k,g.id))}
              {data.methods.filter(m => m.input.goal_ids.includes(g.id)).map(m => <Action key={m.id} title={`关联方法：${m.input.title}`} secondary press={() => void openDetail(m.id)} />)}
              <Action title={`查看目标：${g.input.title}`} secondary press={() => void openDetail(g.id)} />
              <Action title={`编辑目标：${g.input.title}`} secondary press={() => begin({ kind: 'goal', value: g.input }, { kind: 'goal', value: g })} />
            </View>)}
          </> : tab === 'methods' ? <>
            <Field label="搜索方法" value={methodSearch} change={setMethodSearch}/>
            <Action title="新建方法" press={() => begin({ kind: 'method', value: newMethod() })} />
            <Check label="显示暂停和停用方法" checked={showHidden} change={() => setShowHidden(!showHidden)} />
            <HelpButton/>
            {!methods.length && <Text>方法是支持目标的具体做法。可以先写名称并关联目标，以后再补充细节。</Text>}
            <Action title={rankingBusy?'正在整理方法顺序…':'按最新近况重新排序'} secondary disabled={rankingBusy} press={()=>void loadMethodOrder()}/><Text>有困难意味着先检查和调整，不表示应该增加使用次数。列表不会随着后台刷新自动换位。</Text>{rankingError&&<Text>{rankingError}</Text>}
            {!rankingBusy&&methods.map(m => <View key={m.id} style={s.card} testID={`prd-method-${m.id}`}><Text style={s.heading}>{m.input.title}</Text>
              <Text>{methodOrder.find(r=>r.id===m.id)?.reason??'新加入的方法，稍后可按近况重排'}</Text><Text>关联目标：{m.input.goal_ids.map(id => data.goals.find(g => g.id === id)?.input.title ?? '待核对目标').join('、')}</Text>
              <Text>{m.input.description || '具体做法可后补'}</Text>
              {relatedKnowledge(undefined,m.id).map(k => <View key={k.input.id} style={{gap:6}}><Text>{k.input.kind === 'difficulty' ? '关联困难' : '关联经验'}：{k.input.content}</Text><Action title={`查看关联知识：${k.details?.title||k.input.content}`} secondary press={()=>setRequestedFollowup({kind:'knowledge',id:k.input.id,nonce:uuid()})}/></View>)}
              <Action title={`查看方法：${m.input.title}`} secondary press={() => void openDetail(m.id)} />
              <Action title={`编辑方法：${m.input.title}`} secondary press={() => begin({ kind: 'method', value: m.input }, { kind: 'method', value: m })} /></View>)}
          </> : <>
            <Text>选项可新增、改名、停用和恢复，历史记录保留当时名称。</Text>
            {(['course', 'teacher'] as const).map(kind => <View key={kind} style={{ gap: 12 }}><Action title={kind === 'course' ? '新增课程' : '新增老师'} press={() => begin({ kind: 'catalog', value: { kind, name: '', active: true } })} />
              {data.catalog.filter(c => c.input.kind === kind).map(c => <View key={c.id} style={s.card}><Text>{c.input.name} · {c.input.active ? '使用中' : '已停用'}</Text>
                <Action title={`维护选项：${c.input.name}`} secondary press={() => begin({ kind: 'catalog', value: c.input }, { kind: 'catalog', value: c })} />
                <Action title={`查看改名历史：${c.input.name}`} secondary press={() => void openDetail(c.id)} /></View>)}
            </View>)}
          </>}
        </>}
      </>}
      <ObservationHub changed={()=>setProgressNonce(n=>n+1)} reported={reportTodayMoments} data={data} mode={account?'hidden':tab==='today'?'today':tab==='records'&&recordKind==='family'?'records':'hidden'} requested={requestedObservation} onRequested={()=>setRequestedObservation(null)}/>
      {tab!=='today'&&!(tab==='records'&&recordKind==='family')&&sessionHub('drafts')}
      {tab!=='today'&&!(tab==='records'&&recordKind==='lessons')&&<LessonHub openKnowledge={id=>setRequestedFollowup({kind:'knowledge',id,nonce:uuid()})} inboxManaged requested={requestedLesson} onRequested={()=>setRequestedLesson(null)} data={data} actorId={auth.identity.user.id} actorName={auth.identity.user.display_name} admin={auth.identity.user.role==='admin'} mode="drafts" refresh={refresh} savedLesson={lesson=>setData(current=>mergeSnapshot(current,{...emptySnapshot,lessons:[lesson]}))} catalogSaved={saved=>setData(current=>acceptSaved(current,saved))} openGoal={id=>void openDetail(id)} openMethod={id=>void openDetail(id)} scrollTop={()=>scroll.current?.scrollTo({y:0,animated:false})}/>}

      {tab!=='today'&&tab!=='followups'&&followupHub('hidden')}
      {error && !editor ? <Text accessibilityRole="alert">{error}</Text> : null}
    </ScrollView>
    {!account && <View style={s.navigation}>{(['today', 'goals', 'methods', 'records', 'followups'] as const).map(kind => <Action key={kind} title={{ today: '今天', goals: '目标', methods: '方法', records: '记录', followups: '跟进' }[kind]} secondary={tab !== kind} press={() => { setTab(kind); setShowHidden(false); }} />)}</View>}
    {showDirections && <DetailOverlay title="长期关注" close={()=>setShowDirections(false)}>
      {longDirections.map(g=><View key={g.id} style={s.card}><Text style={s.heading}>{g.input.title}</Text><Text>{g.input.meaning||g.input.criteria||'可以在目标页补充想支持的方向。'}</Text>
        {data.sessions.filter(s=>s.status==='completed'&&s.input.business_date<=localDate()&&(s.focus_goals.some(f=>f.id===g.id)||s.actual_targets.some(t=>t.ancestors.some(a=>a.id===g.id)))).sort((a,b)=>b.input.business_date.localeCompare(a.input.business_date)).slice(0,3).map(s=><View key={s.id} style={{gap:5}}><Text>最近家庭观察：{s.input.business_date} · {s.input.summary?.targets.filter(t=>t.performed).map(t=>t.observation).join('；')}</Text>{s.input.summary?.signals.filter(signal=>signal.goal_ids.includes(g.id)).map(signal=><Text key={signal.kind}>明确关联的参与信号：{signal.kind}</Text>)}</View>)}
        {data.lessons.filter(l=>l.status==='recorded'&&l.observation.business_date<=localDate()&&(l.organization.focus_goals.some(f=>f.id===g.id)||l.organization.targets.some(t=>t.ancestors.some(a=>a.id===g.id)))).sort((a,b)=>b.observation.business_date.localeCompare(a.observation.business_date)).slice(0,2).map(l=><Text key={l.id}>最近课堂观察：{l.observation.business_date} · {l.observation.notes||l.observation.teacher_notes||'未补充文字'}</Text>)}
        {relatedKnowledge(g.id).map(k=>knowledgeCard(k,g.id))}
        <Action title={`查看目标：${g.input.title}`} secondary press={()=>{setShowDirections(false);void openDetail(g.id);}}/>
      </View>)}
      {!longDirections.length&&<Action title="新建长期方向" press={()=>{setShowDirections(false);setTab('goals');begin({kind:'goal',value:{...newGoal(),level:'long'}});}}/>}
    </DetailOverlay>}
    {draft && <DetailOverlay title={draft.mutation.kind === 'goal' ? '填写目标' : draft.mutation.kind === 'method' ? '填写方法' : '维护选项'} close={() => { if (!busy) setEditor(null); }} footer={<><SaveStatus storageKey={key} sharing={busy} pending={Boolean(draft.pending)}/><Action title={busy ? '正在保存…' : draft.pending ? '重试同一次保存' : '保存'} disabled={busy || (conflict && !draft.pending)} press={() => void submit()} /></>}>
      {draftRecovery()}<OptionsUpdate changed={candidate.changed} refresh={candidate.refresh}/><Text style={s.hint}>填写内容会留在本机，可关闭后继续。保存成功后家人才能看到。</Text>
      {error ? <Text accessibilityRole="alert">{error}</Text> : null}
      {conflict && latest && <View style={s.card}><Text>另一位家人已修改这条记录，你的填写仍保留。</Text><Text style={s.heading}>家人保存的最新内容</Text><ReadOnlyValues mutation={mutationOf(latest)} data={data} />
        <Action title="备份我的填写并采用最新版本" secondary disabled={busy} press={() => void (async () => {
          try { await AsyncStorage.setItem(`${key}:conflict:${draft.id}:${Date.now()}`, JSON.stringify(draft));
            await persist({ ...draftsRef.current, [draft.id]: { id: draft.id, operation_id: uuid(), expected_version: latest.value.version, change_note: '', mutation: mutationOf(latest), conflict_backup: draft.mutation } }); setError(''); }
          catch (e) { setError(messageOf(e)); }
        })()} /></View>}
      {draft.conflict_backup && <View style={s.card}><Text style={s.heading}>你此前填写的备份</Text><Text style={s.hint}>对照下方最新表单，按需补回；长按文字可复制。</Text><ReadOnlyValues mutation={draft.conflict_backup} data={data} /></View>}
      {draft.mutation.kind === 'goal' ? <GoalFields id={draft.id} value={draft.mutation.value} data={candidate.options} disabled={busy || Boolean(draft.pending)} existing={draft.expected_version !== null} change={patch} /> : draft.mutation.kind === 'method' ?
        <MethodFields value={draft.mutation.value} data={candidate.options} disabled={busy || Boolean(draft.pending)} change={patch} /> : <>
          <Field label={draft.mutation.value.kind === 'course' ? '课程名称' : '老师名称'} value={draft.mutation.value.name} change={name => patch({ name })} placeholder={draft.mutation.value.kind === 'course' ? '例如：语言课' : '例如：张老师'} disabled={busy || Boolean(draft.pending)} />
          <Check label="可在后续记录中选择" checked={draft.mutation.value.active} disabled={busy || Boolean(draft.pending)} change={() => patch({ active: !(draft.mutation.value as { active: boolean }).active })} />
        </>}
      {draft.mutation.kind==='method'&&latest?.kind==='method'&&(draft.method_change||draft.mutation.value.description!==latest.value.input.description||draft.mutation.value.steps!==latest.value.input.steps)&&<><Select label="这次方法修改的性质" value={draft.method_change??'adjustment'} disabled={busy||Boolean(draft.pending)} options={[{value:'adjustment',label:'调整实际做法，分开回看前后观察'},{value:'correction',label:'只更正文案，实际做法没有改变'}]} onChange={v=>void persist({...draftsRef.current,[draft.id]:{...draft,operation_id:uuid(),method_change:v as 'adjustment'|'correction'}})}/><Text>旧观察和方法版本都会保留；仅更正文案不会重新开始比较期。</Text></>}
      {draft.expected_version !== null && <Field label="修改说明" hint="改变状态时，请说明依据或原因。" value={draft.change_note} disabled={busy || Boolean(draft.pending)} change={change_note => void persist({ ...draftsRef.current, [draft.id]: { ...draft, operation_id: uuid(), change_note } })} placeholder="例如：先暂停，等调整好活动再试" />}


    </DetailOverlay>}
    {distribution&&<HistoryPanel data={data} object={distribution} close={()=>setDistribution(null)} openSession={id=>setRequestedSession({id,nonce:uuid()})}/>}
    {detail && <DetailOverlay title="查看详情与修改记录" close={() => { ++detailSequence.current; setDetailId(null); setHistory([]); }}>
      <Text style={s.heading}>{nameOf(mutationOf(detail))}</Text>
      {detail.kind === 'goal' ? <><Text>{detail.value.input.meaning || '尚未补充生活意义'}</Text><Text>{detail.value.input.criteria || '尚未设置观察标准'}</Text>
        {data.methods.filter(m => m.input.goal_ids.includes(detail.value.id)).map(m => <Text key={m.id}>关联方法：{m.input.title}</Text>)}</> : detail.kind === 'method' ? <>
        <Text>{detail.value.input.description || '具体做法可后补'}</Text><Text>当前说明第{detail.value.version}版；适用条件与步骤：{detail.value.input.steps||'未补充'}</Text><Text>指导来源：{detail.value.input.guidance||'未补充'}</Text><MethodUsage data={data} id={detail.value.id} openVersion={version=>setDistribution({id:detail.value.id,kind:'method',version})}/>{detail.value.basis_snapshots?.map(b=><Text key={`${b.id}:${b.version}`}>调整依据：{b.content}（知识第{b.version}版）{b.conditions?`；当时条件：${b.conditions}`:''}</Text>)}
        {detail.value.input.goal_ids.map(id => <Text key={id}>关联目标：{data.goals.find(g => g.id === id)?.input.title}</Text>)}
      </> : null}
      {relatedKnowledge(detail.kind==='goal'?detail.value.id:undefined,detail.kind==='method'?detail.value.id:undefined).map(k=>knowledgeCard(k,detail.kind==='goal'?detail.value.id:undefined,detail.kind==='method'?detail.value.id:undefined))}
      {(detail.kind==='goal'||detail.kind==='method')&&<ObjectContext key={detail.value.id} data={data} object={{id:detail.value.id,kind:detail.kind}} openAction={id=>setRequestedFollowup({kind:'action',id,nonce:uuid()})} openSession={id=>setRequestedSession({id,nonce:uuid()})} openLesson={id=>setRequestedLesson({id,nonce:uuid(),view:'detail'})}/>}
      {(detail.kind==='goal'||detail.kind==='method')&&<Action title="记一条心得" secondary press={()=>setRequestedFollowup({kind:'new_experience',id:detail.value.id,object_kind:detail.kind as 'goal'|'method',nonce:uuid()})}/>}
      {(detail.kind==='goal'||detail.kind==='method')&&<Action title="现在复盘这个对象" secondary press={()=>setRequestedFollowup({kind:'review',id:detail.value.id,object_kind:detail.kind as 'goal'|'method',nonce:uuid()})}/>}
      {(detail.kind==='goal'||detail.kind==='method')&&<Action title="查看覆盖时间分布" secondary press={()=>setDistribution({id:detail.value.id,kind:detail.kind as 'goal'|'method'})}/>}
      {detail.kind==='goal'&&<Action title="查看观察与实际测量" secondary press={()=>setRequestedObservation({kind:'goal',id:detail.value.id,nonce:uuid()})}/>}
      <Text style={s.heading}>修改记录</Text>
      {history.map(rev => <View key={rev.entity.value.version} style={s.card}><Text>{new Date(rev.recorded_at).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })}</Text>
        <Text>{nameOf(mutationOf(rev.entity))}</Text><Text>{rev.change_note || (rev.event === 'created' ? '首次记录' : '内容修改')}</Text>
        {rev.entity.kind === 'method' && <Text>{rev.entity.value.input.description}</Text>}
      </View>)}
      {historyMore&&<Action title='更早的修改记录' disabled={historyLoading} secondary press={()=>void loadObjectHistory(detail.value.id,historyPage+1)}/>}
    </DetailOverlay>}
  </View></LessonOutboxProvider>;
}
const s = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#F6F5EF' }, header: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', padding: 12, gap: 12 },
  brand: { fontSize: 22, fontWeight: '700' }, title: { fontSize: 24, fontWeight: '600' }, heading: { fontSize: 18, fontWeight: '600' },
  content: { padding: 16, gap: 14 }, card: { padding: 16, borderRadius: 12, backgroundColor: '#FFFFFF', gap: 10 },
  action: { padding: 12, backgroundColor: '#34765B', borderRadius: 10 }, secondary: { backgroundColor: '#E9F2E9' },
  label: { fontSize: 17, fontWeight: '500' }, hint: { color: '#53665A', fontSize: 14 },
  input: { padding: 12, borderWidth: 1, borderColor: '#A6B5A8', borderRadius: 10, backgroundColor: '#FFFFFF', textAlignVertical: 'top' },
  navigation: { flexDirection: 'row', flexWrap: 'wrap', justifyContent: 'space-around', padding: 10, gap: 8, backgroundColor: '#FFFFFF' },
});

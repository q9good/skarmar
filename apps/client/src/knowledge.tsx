import React, { useEffect, useState } from 'react';
import { Pressable, StyleSheet, Text, TextInput, View } from 'react-native';
import { ApiError, request, type AppState, type FollowUp, type Goal, type Pathway, type Lesson } from './api';
import { uuid } from './draft';
import { DeleteRecordButton, type DeletedHandler } from './delete-record';

const errorMessage = (error: unknown) => error instanceof Error ? error.message : '操作未完成，请重试';
const toggle = (ids: string[], id: string) => ids.includes(id) ? ids.filter(value => value !== id) : [...ids, id];
const goalName = (goal: Goal) => `${goal.title} · ${{ long: '长期', medium: '中期', short: '短期' }[goal.level]}${goal.status === 'archived' ? ' · 已归档' : ''}`;
function Action({ title, press, disabled }: { title: string; press: () => void; disabled?: boolean }) {
  return <Pressable accessibilityRole="button" disabled={disabled} accessibilityState={{ disabled }} onPress={press} style={[s.action, disabled && { opacity: 0.5 }]}><Text style={s.actionText}>{title}</Text></Pressable>;
}
function Input({ label, value, change, disabled }: { label: string; value: string; change: (value: string) => void; disabled?: boolean }) {
  return <View style={s.field}><Text style={s.label}>{label}</Text><TextInput accessibilityLabel={label} value={value} onChangeText={change} editable={!disabled} multiline style={s.input} /></View>;
}
function Choices({ label, items, selected, change, disabled }: {
  label: string; items: { id: string; title: string }[]; selected: string[]; change: (ids: string[]) => void; disabled?: boolean;
}) {
  return <View style={s.field}><Text style={s.label}>{label}</Text><View style={s.row}>
    {items.map(item => <Pressable key={item.id} accessibilityRole="checkbox" accessibilityLabel={item.title}
      accessibilityState={{ checked: selected.includes(item.id), disabled }} aria-checked={selected.includes(item.id)} disabled={disabled}
      onPress={() => change(toggle(selected, item.id))} style={[s.choice, selected.includes(item.id) && s.checked]}>
      <Text style={s.body}>{selected.includes(item.id) ? '✓ ' : ''}{item.title}</Text>
    </Pressable>)}
    {!items.length && <Text style={s.hint}>暂无可选项，可先创建后再关联。</Text>}
  </View></View>;
}

export function ExperienceBindings({ goals, pathways, goalIds, pathwayIds, changeGoals, changePathways, disabled }: {
  goals: Goal[]; pathways: Pathway[]; goalIds: string[]; pathwayIds: string[];
  changeGoals: (ids: string[]) => void; changePathways: (ids: string[]) => void; disabled?: boolean;
}) {
  return <>
    <Choices label="关联目标 · 可多选" items={goals.map(goal => ({ id: goal.id, title: goalName(goal) }))} selected={goalIds} change={changeGoals} disabled={disabled} />
    <Choices label="关联方法 · 可多选" items={pathways.map(pathway => ({ id: pathway.id, title: pathway.title + (pathway.status === 'archived' ? ' · 已归档' : '') }))} selected={pathwayIds} change={changePathways} disabled={disabled} />
    <Text style={s.hint}>可以只选目标、只选方法，也可以同时选择。方法服务哪些目标，不会自动替当前记录增加绑定。</Text>
  </>;
}

export function ExperienceCreator({ data, saved, cancel }: { data: AppState; saved: (item: FollowUp) => void; cancel: () => void }) {
  const [content, setContent] = useState('');
  const [goalIds, setGoalIds] = useState<string[]>([]);
  const [pathwayIds, setPathwayIds] = useState<string[]>([]);
  const [id, setId] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const submit = async () => {
    if (busy) return;
    setBusy(true); setError('');
    try {
      const nextId = id || uuid(); setId(nextId);
      const item = await request<FollowUp>('/experiences', 'POST', { id: nextId, content, goal_ids: goalIds, pathway_ids: pathwayIds });
      saved(item);
    } catch (error) { setError(errorMessage(error)); } finally { setBusy(false); }
  };
  return <View style={s.card}><Text style={s.title}>新建经验</Text><Text style={s.hint}>直接整理已有认识，不需要虚构一条训练记录。新经验先记为待验证。</Text>
    {error && <Text accessibilityRole="alert" style={s.error}>{error}</Text>}
    <Input label="经验内容" value={content} change={setContent} disabled={busy} />
    <ExperienceBindings goals={data.goals} pathways={data.pathways} goalIds={goalIds} pathwayIds={pathwayIds} changeGoals={setGoalIds} changePathways={setPathwayIds} disabled={busy} />
    <Text style={s.hint}>至少选择一个目标或方法。填写后点击保存；此表单尚无本机自动保存。</Text>
    <Action title={busy ? '正在保存…' : '保存独立经验'} press={() => void submit()} disabled={busy} /><Action title="取消新建经验" press={cancel} disabled={busy} />
  </View>;
}

export function RelatedKnowledge({ data, goalId, pathwayId, openPathway, openLesson }: { data: AppState; goalId?: string; pathwayId?: string; openPathway?: (pathway: Pathway) => void; openLesson?: (lesson: Lesson) => void }) {
  const related = data.pathways.filter(pathway => pathway.id === pathwayId || (goalId && pathway.goal_ids.includes(goalId)));
  const pathwayIds = new Set(related.map(pathway => pathway.id));
  const experiences = data.experiences.filter(item => (goalId && item.goal_ids.includes(goalId)) || item.pathway_ids.some(id => pathwayIds.has(id)));
  const difficulties = data.difficulties.filter(item => (goalId && item.goal_ids.includes(goalId)) || item.pathway_ids.some(id => pathwayIds.has(id)));
  return <View style={s.card}>
    {goalId && <><Text style={s.title}>实现这个目标的方法</Text>
      {related.length ? related.map(pathway => <View key={pathway.id} style={s.field}><Text style={s.label}>{pathway.title}{pathway.status === 'archived' ? ' · 已归档' : ''}</Text><Text style={s.body}>{pathway.description || pathway.steps || '尚未填写具体说明'}</Text>{openPathway && <Action title={`查看方法：${pathway.title}`} press={() => openPathway(pathway)} />}</View>) : <Text style={s.hint}>可在“方法”页创建方式并关联这个目标。</Text>}
    </>}
    {difficulties.length > 0 && <><Text style={s.title}>关联困难 · {difficulties.length}</Text>
      {difficulties.map(item => <View key={item.id} style={s.field}><Text style={s.hint}>{item.status === 'resolved' ? '已解决' : '待跟进'}</Text><Text style={s.body}>{item.content}</Text>{item.conclusion && <Text style={s.hint}>跟进结论：{item.conclusion}</Text>}</View>)}
    </>}
    <Text style={s.title}>相关机构课程</Text>
    {data.lessons.filter(lesson=>(goalId && lesson.links?.targets.some(target=>target.goal_id===goalId)) || (pathwayId && lesson.links?.pathway_ids.includes(pathwayId))).map(lesson=><View key={lesson.id} style={s.field}>
      <Text style={s.label}>{lesson.business_date} · {lesson.course} · {lesson.teacher}</Text>
      <Text style={s.body}>{lesson.content}</Text>
      {lesson.links?.targets.filter(target=>!goalId||target.goal_id===goalId).map(target=><Text key={target.goal_id} style={s.hint}>{target.plan}{target.progress ? ` · 观察：${target.progress}` : ''}</Text>)}
      {openLesson && <Action title={`查看上课记录：${lesson.course} · ${lesson.business_date}`} press={()=>openLesson(lesson)} />}
    </View>)}
    <Text style={s.title}>关联经验</Text>
    {experiences.length ? experiences.map(item => <View key={item.id} style={s.field}>
      <Text style={s.hint}>{[goalId && item.goal_ids.includes(goalId) ? '关联目标' : '', item.pathway_ids.some(id => pathwayIds.has(id)) ? '方法细节' : ''].filter(Boolean).join(' · ')} · {item.status === 'verified' ? '已验证' : item.status === 'stopped' ? '已停用' : '待验证'}</Text>
      <Text style={s.body}>{item.content}</Text>
      {item.conclusion && <Text style={s.hint}>验证依据：{item.conclusion}</Text>}
    </View>) : <Text style={s.hint}>在“跟进”页新增经验或为已有经验补充绑定。</Text>}
  </View>;
}

export function PathwayLibrary({ data, saved, initialPathway, openGoal, removed, openLesson }: { data: AppState; saved: (pathway: Pathway) => void; initialPathway?: Pathway; openGoal: (goal: Goal) => void; removed: DeletedHandler; openLesson?: (lesson: Lesson) => void }) {
  const [editing, setEditing] = useState<Pathway | 'new' | null>(initialPathway ?? null);
  useEffect(() => { if (editing && editing !== 'new' && data.deleted?.some(item => item.kind === 'pathways' && item.id === editing.id)) setEditing(null); }, [data.deleted, editing]);
  if (editing) return <>
    {editing !== 'new' && <View style={s.card}><Text style={s.label}>关联目标</Text>{(data.pathways.find(item => item.id === editing.id) ?? editing).goal_ids.map(id => {
      const goal = data.goals.find(item => item.id === id);
      return goal ? <Action key={id} title={`查看目标：${goal.title}`} press={() => openGoal(goal)} /> : null;
    })}</View>}
    <PathwayForm key={editing === 'new' ? 'new' : editing.id} initial={editing === 'new' ? undefined : data.pathways.find(item => item.id === editing.id) ?? editing} goals={data.goals}
      saved={pathway => { saved(pathway); setEditing(null); }} cancel={() => setEditing(null)} />
    {editing !== 'new' && <RelatedKnowledge data={data} openLesson={openLesson} pathwayId={editing.id} />}
    {editing !== 'new' && <DeleteRecordButton kind="pathways" id={editing.id} version={(data.pathways.find(item => item.id === editing.id) ?? editing).version} label="方法" removed={removed} hint="上课记录、困难与经验会保留，并解除与此方法的关联。" />}
  </>;
  return <>
    <Text style={s.hint}>目标回答“要达到什么”，方法回答“怎样实现”。同一个方法可以服务多个目标，每个目标也可以尝试多种方法。</Text>
    <Action title="＋ 新建方法" press={() => setEditing('new')} />
    {data.pathways.length ? data.pathways.map(pathway => <View key={pathway.id} style={s.card}>
      <Text style={s.label}>关联目标：{pathway.goal_ids.map(id => data.goals.find(goal => goal.id === id)?.title ?? '目标待核对').join(' · ') || '尚未关联目标'}</Text>
      <Text style={s.title}>{pathway.title}</Text><Text style={s.hint}>{pathway.status === 'archived' ? '已归档' : '使用中'} · {pathway.goal_ids.length} 个目标</Text>
      <Text style={s.body}>{pathway.description || '尚未填写作用说明'}</Text>
      <Action title={`查看方法：${pathway.title}`} press={() => setEditing(pathway)} />
    </View>) : <View style={s.card}><Text style={s.title}>从一种熟悉的方式开始</Text><Text style={s.hint}>例如共同阅读、视觉提示或分步练习，写清怎样做，再关联想实现的目标。</Text></View>}
  </>;
}

function PathwayForm({ initial, goals, saved, cancel }: { initial?: Pathway; goals: Goal[]; saved: (pathway: Pathway) => void; cancel: () => void }) {
  const [id, setId] = useState(initial?.id ?? '');
  const [version, setVersion] = useState(initial?.version);
  const [title, setTitle] = useState(initial?.title ?? '');
  const [description, setDescription] = useState(initial?.description ?? '');
  const [steps, setSteps] = useState(initial?.steps ?? '');
  const [status, setStatus] = useState<Pathway['status']>(initial?.status ?? 'active');
  const [goalIds, setGoalIds] = useState(initial?.goal_ids ?? []);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [latest, setLatest] = useState<Pathway | null>(null);
  useEffect(() => { if (initial && initial.version > (version ?? 0)) setLatest(initial); }, [initial, version]);
  const submit = async () => {
    if (busy) return;
    setBusy(true); setError(''); setLatest(null);
    try {
      const nextId = id || uuid(); setId(nextId);
      const body = { title, description, steps, goal_ids: goalIds };
      const value = await request<Pathway>(initial ? `/pathways/${nextId}` : '/pathways', initial ? 'PUT' : 'POST', initial ? { ...body, expected_version: version, status } : { ...body, id: nextId });
      saved(value);
    } catch (error) {
      setError(errorMessage(error));
      if (error instanceof ApiError && error.status === 409 && initial) {
        try { const current = await request<AppState>('/state'); setLatest(current.pathways.find(pathway => pathway.id === id) ?? null); } catch {}
      }
    } finally { setBusy(false); }
  };
  return <View style={s.card}><Text style={s.title}>{initial ? '编辑方法' : '新建方法'}</Text>
    {error && <Text accessibilityRole="alert" style={s.error}>{error}</Text>}
    {latest && <View style={s.field}><Text style={s.hint}>最新版本：{latest.title} · {latest.description}</Text>
      <Action title="载入最新方法内容" disabled={busy} press={() => { setTitle(latest.title); setDescription(latest.description); setSteps(latest.steps); setStatus(latest.status); setGoalIds(latest.goal_ids); setVersion(latest.version); setLatest(null); setError(''); }} />
      <Action title="保留填写，按最新版本继续" disabled={busy} press={() => { setVersion(latest.version); setLatest(null); setError('已保留填写，请核对后再保存。'); }} />
    </View>}
    <Choices label="此方法服务的目标 · 可多选" items={goals.map(goal => ({ id: goal.id, title: goalName(goal) }))} selected={goalIds} change={setGoalIds} disabled={busy} />
    <Input label="方法名称" value={title} change={setTitle} disabled={busy} />
    <Input label="作用说明" value={description} change={setDescription} disabled={busy} />
    <Input label="具体步骤" value={steps} change={setSteps} disabled={busy} />

    {initial && <><Text style={s.label}>方法状态</Text><View style={s.row}>
      {(['active', 'archived'] as const).map(value => <Pressable key={value} accessibilityRole="radio" accessibilityState={{ checked: status === value, disabled: busy }}
        disabled={busy} onPress={() => setStatus(value)} style={[s.choice, status === value && s.checked]}><Text style={s.body}>{value === 'active' ? '使用中' : '已归档'}</Text></Pressable>)}
    </View></>}
    <Text style={s.hint}>可以暂不关联目标。归档仍保留关联与经验。填写后点击保存；此表单尚无本机自动保存。</Text>
    <Action title={busy ? '正在保存…' : '保存方法'} press={() => void submit()} disabled={busy} /><Action title="返回方法列表" press={cancel} disabled={busy} />
  </View>;
}

const s = StyleSheet.create({
  card: { padding: 18, borderRadius: 18, borderWidth: 1, borderColor: '#E0E6DD', backgroundColor: '#FFFFFF', gap: 13 },
  title: { color: '#243D36', fontWeight: '600', fontSize: 17 }, label: { color: '#243D36', fontWeight: '500', fontSize: 13 },
  body: { color: '#243D36', fontSize: 14, lineHeight: 23 }, hint: { color: '#6C7B73', fontSize: 12, lineHeight: 21 },
  field: { gap: 8 }, row: { flexDirection: 'row', flexWrap: 'wrap', gap: 8 },
  choice: { padding: 10, borderRadius: 10, backgroundColor: '#F6F5EF', borderWidth: 1, borderColor: '#E0E6DD', maxWidth: '100%' }, checked: { borderColor: '#34765B', backgroundColor: '#E9F2E9' },
  action: { minHeight: 46, padding: 12, borderRadius: 12, alignItems: 'center', justifyContent: 'center', backgroundColor: '#E9F2E9' }, actionText: { color: '#34765B', fontSize: 14, fontWeight: '600' },
  input: { minHeight: 72, padding: 12, borderRadius: 10, borderWidth: 1, borderColor: '#E0E6DD', color: '#243D36', backgroundColor: '#FCFCF9', textAlignVertical: 'top' },
  error: { padding: 12, borderRadius: 10, backgroundColor: '#FBECE2', color: '#925231', lineHeight: 21 },
});

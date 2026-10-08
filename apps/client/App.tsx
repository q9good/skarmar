import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  ActivityIndicator, Pressable, ScrollView, StyleSheet, Text, TextInput,
  View, useWindowDimensions, StatusBar,
} from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import {
  ApiError, request, type AppState, type FollowUp, type Goal, type Level,
  type Role, type SaveRequest, type SaveResult, type Session, type SessionInput,
} from './src/api';
import { clearDraft, emptyInput, localDate, readDraft, saveDraft, uuid, type Draft } from './src/draft';

const colors = { ink: '#243D36', muted: '#6C7B73', green: '#34765B', light: '#E9F2E9',
  cream: '#F6F5EF', white: '#FFFFFF', border: '#E0E6DD', orange: '#AA713B' };
const levels: Record<Level, string> = { long: '长期', medium: '中期', short: '短期' };
const roles: Record<Role, string> = { primary: '主目标', secondary: '副目标', review: '复习' };
const activities = ['桌面教学', '感统活动', '游戏', '家务劳动', '生活自理', '户外活动', '阅读'];
const tabs = [ ['today', '☀', '今天'], ['goals', '◎', '目标'], ['history', '▤', '记录'], ['followups', '◇', '跟进'] ] as const;
type Tab = typeof tabs[number][0];
const emptyState: AppState = { goals: [], sessions: [], difficulties: [], experiences: [] };

function Button({ title, onPress, secondary = false, disabled = false }: {
  title: string; onPress: () => void; secondary?: boolean; disabled?: boolean;
}) {
  return <Pressable accessibilityRole="button" accessibilityState={{ disabled }} onPress={onPress}
    disabled={disabled} style={[styles.button, secondary && styles.secondaryButton, disabled && { opacity: 0.45 }]}>
    <Text style={[styles.buttonText, secondary && { color: colors.green }]}>{title}</Text>
  </Pressable>;
}

function Chip({ title, selected, onPress, disabled }: {
  title: string; selected?: boolean; onPress: () => void; disabled?: boolean;
}) {
  return <Pressable accessibilityRole="button" accessibilityState={{ selected, disabled }}
    onPress={onPress} disabled={disabled} style={[styles.chip, selected && styles.selectedChip]}>
    <Text style={[styles.chipText, selected && { color: colors.green, fontWeight: '600' }]}>{title}</Text>
  </Pressable>;
}

function Field({ label, value, onChange, multiline = false, placeholder = '', disabled = false }: {
  label: string; value: string; onChange: (value: string) => void; multiline?: boolean; placeholder?: string; disabled?: boolean;
}) {
  return <View style={styles.field}>
    <Text style={styles.label}>{label}</Text>
    <TextInput accessibilityLabel={label} value={value} onChangeText={onChange}
      editable={!disabled} multiline={multiline} placeholder={placeholder} placeholderTextColor="#93A097"
      style={[styles.input, multiline && styles.textarea]} />
  </View>;
}

function Card({ children }: { children: React.ReactNode }) { return <View style={styles.card}>{children}</View>; }
function Hint({ children }: { children: React.ReactNode }) { return <Text style={styles.hint}>{children}</Text>; }

function inputOf(session: Session): SessionInput {
  const { id, version, status, target_snapshots, ...input } = session;
  return input;
}

export default function App() {
  const [data, setData] = useState<AppState>(emptyState);
  const [tab, setTab] = useState<Tab>('today');
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [draft, setDraft] = useState<Draft | null>(null);
  const [editing, setEditing] = useState(false);
  const [localSaved, setLocalSaved] = useState(false);
  const [storageReady, setStorageReady] = useState(false);
  const [detail, setDetail] = useState<Session | null>(null);
  const [selectedGoal, setSelectedGoal] = useState<Goal | null>(null);
  const [showGoalForm, setShowGoalForm] = useState(false);
  const [conflict, setConflict] = useState<Session | null>(null);
  const persistence = useRef<Promise<void>>(Promise.resolve());
  const { width } = useWindowDimensions();

  const persist = useCallback((next: Draft) => {
    const task = persistence.current.catch(() => undefined).then(() => saveDraft(next));
    persistence.current = task;
    return task;
  }, []);

  const refresh = useCallback(async () => {
    const next = await request<AppState>('/state');
    setData(next);
    return next;
  }, []);

  useEffect(() => {
    let active = true;
    Promise.allSettled([refresh(), readDraft()]).then(results => {
      if (!active) return;
      const [remote, local] = results;
      if (remote.status === 'rejected') setError('暂时无法连接，请重试。已有本机草稿仍可继续编辑。');
      if (local.status === 'fulfilled') { setDraft(local.value); setStorageReady(true); }
      else setError('无法读取本机草稿，请保留浏览器数据后重试。');
      setLoading(false);
    });
    return () => { active = false; };
  }, [refresh]);

  useEffect(() => {
    if (!draft) return;
    let active = true;
    setLocalSaved(false);
    persist(draft).then(() => { if (active) setLocalSaved(true); })
      .catch(() => { if (active) setError('本机草稿保存失败，请勿关闭页面。'); });
    return () => { active = false; };
  }, [draft, persist]);

  const safely = async (action: () => Promise<void>) => {
    setBusy(true); setError(''); setMessage('');
    try { await action(); }
    catch (e) { setError(e instanceof Error ? e.message : '操作未完成，请重试'); }
    finally { setBusy(false); }
  };

  const dirty = () => draft && (draft.pending || !data.sessions.some(s =>
    s.id === draft.id && s.version === draft.version && JSON.stringify(inputOf(s)) === JSON.stringify(draft.input)));

  const startNew = () => {
    if (!storageReady) { setError('无法读取已有本机草稿，请先恢复存储访问后继续。'); return; }
    if (dirty()) { setEditing(true); setError('请先保存当前草稿，再创建另一条训练。'); return; }
    try {
      setDraft({ id: uuid(), version: null, input: emptyInput() });
      setEditing(true); setDetail(null); setConflict(null); setError(''); setMessage('');
    } catch (e) { setError((e as Error).message); }
  };

  const openSession = (session: Session) => {
    if (session.status === 'completed') { setDetail(session); setEditing(false); setSelectedGoal(null); return; }
    if (!storageReady) { setError('无法读取已有本机草稿，请先恢复存储访问后继续。'); return; }
    if (draft?.id !== session.id && dirty()) { setError('当前本机草稿尚未提交，请先保存后切换。'); return; }
    if (draft?.id !== session.id) setDraft({ id: session.id, version: session.version, input: inputOf(session) });
    setEditing(true); setDetail(null); setConflict(null); setError(''); setSelectedGoal(null);
  };

  const change = (value: Partial<SessionInput>) => {
    if (!draft || busy || draft.pending) return;
    setDraft({ ...draft, input: { ...draft.input, ...value } }); setMessage('');
  };

  const submit = (action: SaveRequest['action']) => safely(async () => {
    if (!draft) return;
    // Retain the exact request before sending. An uncertain response reuses the same operation ID.
    const pending = draft.pending ?? { operation_id: uuid(), expected_version: draft.version, action, session: draft.input };
    const queued = { ...draft, pending };
    await persist(queued);
    setDraft(queued);
    let result: SaveResult;
    try { result = await request<SaveResult>(`/sessions/${draft.id}`, 'PUT', pending); }
    catch (e) {
      if (e instanceof ApiError && e.status < 500) {
        const retryable = { ...draft, pending: undefined };
        await persist(retryable); setDraft(retryable);
        if (e.status === 409) {
          const latest = await refresh();
          setConflict(latest.sessions.find(s => s.id === draft.id) ?? null);
        }
      }
      throw e;
    }
    const nextState = { ...data,
      sessions: [result.session, ...data.sessions.filter(s => s.id !== result.session.id)] };
    setData(nextState);
    if (result.session.status === 'completed') {
      setDraft(null); setEditing(false); setDetail(result.session);
      // Await prior writes before clearing, so a delayed autosave cannot resurrect the draft.
      await persistence.current;
      await clearDraft();
      setMessage('这次训练已完成，困难与经验已按填写内容保存。');
    } else {
      const saved = { id: result.session.id, version: result.session.version, input: inputOf(result.session) };
      await persist(saved); setDraft(saved);
      setMessage('计划已保存。训练后继续回填这条记录。');
    }
    // A refresh failure must not turn a confirmed save into an uncertain mutation.
    try { await refresh(); } catch { setError('记录已保存，列表暂未刷新。请稍后刷新查看。'); }
  });

  const resolveConflict = (useRemote: boolean) => safely(async () => {
    if (!draft || !conflict) return;
    await AsyncStorage.setItem(`skarma:conflict-backup:${draft.id}`, JSON.stringify(draft));
    if (conflict.status === 'completed') {
      setDraft(null); await persistence.current; await clearDraft();
      setDetail(conflict); setEditing(false);
      setMessage('已查看完成版本，本机修改另存为备份。');
    } else {
      const next = { id: draft.id, version: conflict.version,
        input: useRemote ? inputOf(conflict) : draft.input };
      await persist(next); setDraft(next);
      setMessage(useRemote ? '已采用最新内容，本机旧草稿另存为备份。' : '已保留本机内容，请核对后再保存。');
    }
    setConflict(null); setError('');
  });

  const today = localDate();
  const todaySessions = data.sessions.filter(s => s.business_date === today);
  const due = data.difficulties.filter(d => d.status !== 'resolved' && d.next_review_date && d.next_review_date <= today);
  const pageTitle = { today: '今天', goals: '目标', history: '训练记录', followups: '困难与经验' }[tab];

  return <View style={styles.root}>
    <StatusBar barStyle="dark-content" />
    <View style={[styles.shell, width > 700 && { maxWidth: 960 }]}>
      <View style={styles.header}>
        <View style={styles.brand}><Text style={styles.star}>✦</Text><Text style={styles.brandName}>skarma</Text></View>
        <Text style={styles.prototype}>体验版 · 示例空间</Text>
      </View>
      <ScrollView style={{ flex: 1 }} contentContainerStyle={styles.content} keyboardShouldPersistTaps="handled">
        {error ? <View accessibilityRole="alert" style={styles.error}><Text style={styles.errorText}>{error}</Text></View> : null}
        {message ? <View accessibilityRole="alert" style={styles.notice}><Text style={styles.noticeText}>{message}</Text></View> : null}
        {loading ? <ActivityIndicator color={colors.green} style={{ marginVertical: 40 }} /> :
          editing && draft ? <>
            <View style={styles.sectionHeader}><Text style={styles.title}>一次训练</Text>
              <Button title="返回" secondary onPress={() => setEditing(false)} disabled={busy} /></View>
            <Hint>{draft.pending ? '上次提交结果尚未确认，请重试同一次提交。' :
              localSaved ? '草稿已保存在本机 · 正式保存后可在其他设备继续' : '正在保存本机草稿…'}</Hint>
            {conflict ? <Card>
              <Text style={styles.cardTitle}>发现另一个版本</Text>
              <Text style={styles.body}>最新安排：{conflict.plan || '未填写'}</Text>
              <Text style={styles.body}>最新观察：{conflict.observation || '未填写'}</Text>
              <Hint>请核对双方修改。操作前会另存当前草稿为本机备份。</Hint>
              <Button title="采用最新记录" secondary onPress={() => void resolveConflict(true)} disabled={busy} />
              {conflict.status !== 'completed' && <Button title="保留我的内容，核对后继续" secondary onPress={() => void resolveConflict(false)} disabled={busy} />}
            </Card> : null}
            <TrainingEditor draft={draft} goals={data.goals} sessions={data.sessions} difficulties={data.difficulties}
              disabled={busy || Boolean(draft.pending)} change={change} />
            {draft.pending ? <Button title="重试同一次提交" onPress={() => void submit(draft.pending!.action)} disabled={busy} /> :
              <View style={styles.row}><Button title="保存计划／草稿" secondary onPress={() => void submit('save_draft')} disabled={busy} />
                <Button title={busy ? '正在保存…' : '完成训练'} onPress={() => void submit('complete')} disabled={busy} /></View>}
          </> : detail ? <SessionDetail session={detail} data={data} back={() => setDetail(null)} goFollowups={() => { setDetail(null); setTab('followups'); }} />
          : selectedGoal ? <>
            <View style={styles.sectionHeader}><Text style={styles.title}>{selectedGoal.title}</Text><Button title="返回" secondary onPress={() => setSelectedGoal(null)} /></View>
            <Card><Text style={styles.badge}>{levels[selectedGoal.level]} · {selectedGoal.status === 'archived' ? '归档复习' : '当前目标'}</Text>
              <Text style={styles.body}>{selectedGoal.criteria || '尚未填写达成标准'}</Text>
              <Hint>上级：{data.goals.find(g => g.id === selectedGoal.parent_id)?.title ?? '无'}</Hint></Card>
            <Text style={styles.subtitle}>相关训练</Text>
            {data.sessions.filter(s => s.targets.some(t => t.goal_id === selectedGoal.id)).map(s => <SessionCard key={s.id} session={s} onPress={() => openSession(s)} />)}
            {!data.sessions.some(s => s.targets.some(t => t.goal_id === selectedGoal.id)) && <Hint>开始记录后，这里会呈现相关训练与逐目标结果。</Hint>}
          </> : <>
            <View style={styles.sectionHeader}><Text style={styles.title}>{pageTitle}</Text>
              <Button title="刷新" secondary disabled={busy} onPress={() => void safely(async () => { await refresh(); })} /></View>
            {tab === 'today' && <>
              <View style={styles.hero}>
                <Text style={styles.eyebrow}>{today.replaceAll('-', '.')} · 每一步都值得记录</Text>
                <Text style={styles.heroTitle}>把今天的小进步，{width < 500 ? '\n' : ''}留下来。</Text>
                <Text style={styles.heroText}>课前安排，课后回填。{width < 500 ? '\n' : ' '}让每一次观察都能成为下一步的依据。</Text>
                <Button title="＋ 新建训练计划" onPress={startNew} disabled={busy} />
              </View>
              {draft ? <Card><Text style={styles.cardTitle}>继续你的草稿</Text><Hint>{draft.input.business_date} · {draft.version ? '已有计划，可继续回填' : '本机尚未提交'}</Hint>
                <Button title="继续填写" secondary onPress={() => setEditing(true)} /></Card> : null}
              <View style={styles.stats}>
                <Stat number={todaySessions.length} label="今天的训练" />
                <Stat number={todaySessions.filter(s => s.status === 'completed').length} label="已完成记录" />
                <Stat number={due.length} label="到期困难" />
              </View>
              <Text style={styles.subtitle}>今天的安排</Text>
              {todaySessions.length ? todaySessions.map(s => <SessionCard key={s.id} session={s} onPress={() => openSession(s)} />) :
                <Card><Text style={styles.cardTitle}>从一次熟悉的活动开始</Text><Hint>游戏、阅读或生活中的小事，都可以成为一次有计划的训练。</Hint></Card>}
              {due.length > 0 && <Card><Text style={styles.cardTitle}>有 {due.length} 条困难到了回看时间</Text>
                <Button title="查看跟进" secondary onPress={() => setTab('followups')} /></Card>}
              <Text style={styles.subtitle}>长期关注</Text>
              {data.goals.filter(g => g.level === 'long').map(g => <GoalCard key={g.id} goal={g} onPress={() => setSelectedGoal(g)} />)}
            </>}
            {tab === 'goals' && <>
              <Hint>长期方向持续关注，中短期目标进入训练，达成后保留归档与复习。</Hint>
              <Button title={showGoalForm ? '收起新目标' : '＋ 新建目标'} secondary onPress={() => setShowGoalForm(v => !v)} />
              {showGoalForm && <GoalForm goals={data.goals} busy={busy} create={input => void safely(async () => {
                await request<Goal>('/goals', 'POST', { id: uuid(), ...input });
                setShowGoalForm(false); await refresh(); setMessage('新目标已保存。');
              })} />}
              {(['long', 'medium', 'short'] as Level[]).map(level => <View key={level}>
                <Text style={styles.subtitle}>{levels[level]}目标</Text>
                {data.goals.filter(g => g.level === level).map(g => <GoalCard key={g.id} goal={g} onPress={() => setSelectedGoal(g)} />)}
              </View>)}
            </>}
            {tab === 'history' && <>
              <Hint>计划和课后结果始终属于同一次训练。每次记录都能回到当时的目标与观察。</Hint>
              {data.sessions.length ? [...data.sessions].sort((a, b) => b.business_date.localeCompare(a.business_date))
                .map(s => <SessionCard key={s.id} session={s} onPress={() => openSession(s)} />) :
                <Card><Text style={styles.cardTitle}>还没有训练记录</Text><Hint>从“今天”新建一条计划，完成后可以在这里回看。</Hint></Card>}
            </>}
            {tab === 'followups' && <>
              <Hint>训练中的困难进入后续跟进；成功片段先保存为待验证经验。</Hint>
              <Text style={styles.subtitle}>困难跟进 · {data.difficulties.length}</Text>
              {data.difficulties.length ? data.difficulties.map(item => <FollowUpCard key={item.id} item={item} kind="difficulties" busy={busy}
                source={() => { const s = data.sessions.find(s => s.id === item.source_session_id); if (s) openSession(s); }}
                update={body => void safely(async () => { await request(`/difficulties/${item.id}`, 'PUT', body); await refresh(); setMessage('跟进已保存。'); })} />) : <Hint>完成训练时填写困难，这里会保留来源与跟进过程。</Hint>}
              <Text style={styles.subtitle}>成功经验 · {data.experiences.length}</Text>
              {data.experiences.length ? data.experiences.map(item => <FollowUpCard key={item.id} item={item} kind="experiences" busy={busy}
                source={() => { const s = data.sessions.find(s => s.id === item.source_session_id); if (s) openSession(s); }}
                update={body => void safely(async () => { await request(`/experiences/${item.id}`, 'PUT', body); await refresh(); setMessage('验证记录已保存。'); })} />) : <Hint>已保存不等于已验证。复用前，请补充适用情况和验证依据。</Hint>}
            </>}
          </>}
        <Text style={styles.footer}>Every star shines at its own pace.</Text>
      </ScrollView>
      <View style={styles.navigation}>
        {tabs.map(([key, icon, label]) => <Pressable key={key} accessibilityRole="button" accessibilityLabel={label}
          accessibilityState={{ selected: tab === key && !editing }} style={styles.navItem} disabled={busy}
          onPress={() => { setTab(key); setEditing(false); setDetail(null); setSelectedGoal(null); setError(''); setMessage(''); }}>
          <Text style={[styles.navIcon, tab === key && { color: colors.green }]}>{icon}</Text>
          <Text style={[styles.navLabel, tab === key && { color: colors.green, fontWeight: '600' }]}>{label}</Text>
        </Pressable>)}
      </View>
    </View>
  </View>;
}

function Stat({ number, label }: { number: number; label: string }) {
  return <View style={styles.stat}><Text style={styles.statNumber}>{number}</Text><Text style={styles.hint}>{label}</Text></View>;
}

function GoalCard({ goal, onPress }: { goal: Goal; onPress: () => void }) {
  return <Pressable accessibilityRole="button" onPress={onPress} style={styles.card}>
    <View style={styles.sectionHeader}><Text style={styles.cardTitle}>{goal.title}</Text><Text style={styles.badge}>{goal.status === 'archived' ? '归档' : levels[goal.level]}</Text></View>
    <Hint>{goal.criteria || '尚未填写达成标准'}</Hint>
  </Pressable>;
}

function SessionCard({ session, onPress }: { session: Session; onPress: () => void }) {
  return <Pressable accessibilityRole="button" onPress={onPress} style={styles.card}>
    <View style={styles.sectionHeader}><Text style={styles.badge}>{session.business_date} · {session.time_slot}</Text>
      <Text style={styles.badge}>{session.status === 'completed' ? '已完成' : '计划中'}</Text></View>
    <Text style={styles.cardTitle}>{session.activities.join(' · ') || '待安排训练活动'}</Text>
    <Text style={styles.body} numberOfLines={2}>{session.plan || '点击继续完善这次训练'}</Text>
    <Hint>{session.organization} · {session.targets.length} 个目标</Hint>
  </Pressable>;
}

function TrainingEditor({ draft, goals, sessions, difficulties, disabled, change }: {
  draft: Draft; goals: Goal[]; sessions: Session[]; difficulties: FollowUp[];
  disabled: boolean; change: (input: Partial<SessionInput>) => void;
}) {
  const input = draft.input;
  const [adding, setAdding] = useState<Role | null>(null);
  const updateTarget = (index: number, patch: Partial<SessionInput['targets'][number]>) =>
    change({ targets: input.targets.map((t, i) => i === index ? { ...t, ...patch } : t) });
  const candidates = goals.filter(g => g.level !== 'long' &&
    g.status === (adding === 'review' ? 'archived' : 'active') && !input.targets.some(t => t.goal_id === g.id));
  return <>
    <Card><Text style={styles.cardTitle}>1 · 课前安排</Text>
      <Field label="训练日期" value={input.business_date} onChange={v => change({ business_date: v })} placeholder="YYYY-MM-DD" disabled={disabled} />
      <Text style={styles.label}>时段</Text><View style={styles.row}>{['上午', '下午', '晚上'].map(v => <Chip key={v} title={v} selected={v === input.time_slot} onPress={() => change({ time_slot: v })} disabled={disabled} />)}</View>
      <Text style={styles.label}>训练活动 · 可多选</Text><View style={styles.row}>{activities.map(v => <Chip key={v} title={v} selected={input.activities.includes(v)} disabled={disabled}
        onPress={() => change({ activities: input.activities.includes(v) ? input.activities.filter(a => a !== v) : [...input.activities, v] })} />)}</View>
      <Text style={styles.label}>组织方式</Text><View style={styles.row}>{['居家', '一对一', '小组课'].map(v => <Chip key={v} title={v} selected={v === input.organization} onPress={() => change({ organization: v })} disabled={disabled} />)}</View>
      <Field label="整次训练安排" value={input.plan} onChange={v => change({ plan: v })} multiline placeholder="准备做什么？在哪里、和谁一起？" disabled={disabled} />
    </Card>
    <Card><Text style={styles.cardTitle}>2 · 本次目标</Text><Hint>正式完成时需要一个主目标、至少一个副目标。复习目标单独添加。</Hint>
      {input.targets.map((target, index) => {
        const goal = goals.find(g => g.id === target.goal_id);
        const last = sessions.filter(s => s.id !== draft.id && s.status === 'completed')
          .sort((a, b) => b.business_date.localeCompare(a.business_date))
          .flatMap(s => s.targets.filter(t => t.goal_id === target.goal_id).map(t => ({ date: s.business_date, progress: t.progress })))[0];
        const unresolved = difficulties.filter(d => d.status !== 'resolved' && sessions.some(s => s.id === d.source_session_id && s.targets.some(t => t.goal_id === target.goal_id)));
        return <View style={styles.target} key={target.goal_id}>
          <View style={styles.sectionHeader}><Text style={styles.cardTitle}>{roles[target.role]} · {goal?.title ?? '目标已变更'}</Text>
            <Pressable accessibilityRole="button" accessibilityLabel={`移除${goal?.title}`} disabled={disabled} onPress={() => change({ targets: input.targets.filter((_, i) => i !== index) })}><Text style={styles.link}>移除</Text></Pressable></View>
          <Hint>{goal?.criteria}</Hint>
          {last && <Text style={styles.context}>上次（{last.date}）：{last.progress || '未填写逐目标观察'}</Text>}
          {unresolved.length > 0 && <Hint>相关训练中有 {unresolved.length} 条未解决困难，可在跟进页查看。</Hint>}
          <Field label={`${roles[target.role]}计划`} value={target.plan} onChange={v => updateTarget(index, { plan: v })} multiline placeholder="这次准备如何围绕该目标开展？" disabled={disabled} />
        </View>;
      })}
      <View style={styles.row}>
        {!input.targets.some(t => t.role === 'primary') && <Button title="＋ 主目标" secondary onPress={() => setAdding('primary')} disabled={disabled} />}
        <Button title="＋ 副目标" secondary onPress={() => setAdding('secondary')} disabled={disabled} />
        <Button title="＋ 复习" secondary onPress={() => setAdding('review')} disabled={disabled} />
      </View>
      {adding && <View style={styles.candidates}><Text style={styles.label}>选择{roles[adding]}</Text>
        {candidates.length ? candidates.map(g => <Chip key={g.id} title={g.title} onPress={() => {
          change({ targets: [...input.targets, { goal_id: g.id, role: adding, plan: '', progress: '', outcome: null, next_step: '' }] }); setAdding(null);
        }} disabled={disabled} />) : <Hint>暂无可选目标，可以先保存草稿，再到目标页新建。</Hint>}
        <Button title="收起选择" secondary onPress={() => setAdding(null)} />
      </View>}
    </Card>
    <Card><Text style={styles.cardTitle}>3 · 课后回填</Text><Hint>训练前可以先保存计划，训练后回来填写以下内容。</Hint>
      {input.targets.map((target, index) => <View key={target.goal_id} style={styles.target}>
        <Text style={styles.label}>{roles[target.role]} · {goals.find(g => g.id === target.goal_id)?.title}</Text>
        <Field label={`${roles[target.role]}实际进度`} value={target.progress} onChange={v => updateTarget(index, { progress: v })} multiline placeholder="实际发生了什么？接受了哪些帮助？未观察可以留空。" disabled={disabled} />
        <View style={styles.row}>{([['continue', '继续'], ['archive', '达成归档'], ['adjust', '调整']] as const).map(([value, label]) =>
          <Chip key={value} title={label} selected={target.outcome === value} onPress={() => updateTarget(index, { outcome: value })} disabled={disabled} />)}</View>
        <Field label={`${roles[target.role]}下一步`} value={target.next_step} onChange={v => updateTarget(index, { next_step: v })} placeholder={target.outcome === 'adjust' ? '调整时请填写具体说明' : '可选：下次准备怎样继续'} disabled={disabled} />
      </View>)}
      <Field label="整次训练观察" value={input.observation} onChange={v => change({ observation: v })} multiline placeholder="补充整次活动的情况、环境或值得留意的片段" disabled={disabled} />
      <Text style={styles.label}>本次是否遇到困难？</Text><View style={styles.row}>{[false, true].map(v => <Chip key={String(v)} title={v ? '是' : '否'} selected={input.has_difficulty === v} onPress={() => change({ has_difficulty: v })} disabled={disabled} />)}</View>
      {input.has_difficulty && <Field label="困难详情" value={input.difficulty} onChange={v => change({ difficulty: v })} multiline placeholder="描述观察到的困难，完成后进入待确认跟进" disabled={disabled} />}
      <Text style={styles.label}>本次是否有成功经验？</Text><View style={styles.row}>{[false, true].map(v => <Chip key={String(v)} title={v ? '是' : '否'} selected={input.has_experience === v} onPress={() => change({ has_experience: v })} disabled={disabled} />)}</View>
      {input.has_experience && <>
        <Field label="成功经验详情" value={input.experience} onChange={v => change({ experience: v })} multiline placeholder="做了什么、在什么条件下有帮助？完成后保存为待验证经验" disabled={disabled} />
        <View style={styles.row}>{([['task', '中短期任务'], ['general', '长期普适']] as const).map(([v, title]) => <Chip key={v} title={title} selected={input.experience_kind === v} onPress={() => change({ experience_kind: v })} disabled={disabled} />)}</View>
      </>}
    </Card>
  </>;
}

function GoalForm({ goals, busy, create }: {
  goals: Goal[]; busy: boolean;
  create: (input: { title: string; level: Level; parent_id: string | null; area: string; criteria: string }) => void;
}) {
  const [title, setTitle] = useState('');
  const [level, setLevel] = useState<Level>('short');
  const [parent, setParent] = useState<string | null>(null);
  const [area, setArea] = useState('');
  const [criteria, setCriteria] = useState('');
  return <Card><Text style={styles.cardTitle}>新目标</Text>
    <Field label="目标名称" value={title} onChange={setTitle} disabled={busy} />
    <View style={styles.row}>{(['long', 'medium', 'short'] as Level[]).map(v => <Chip key={v} title={levels[v]} selected={v === level} onPress={() => { setLevel(v); setParent(null); }} disabled={busy} />)}</View>
    {level !== 'long' && <><Text style={styles.label}>上级目标</Text><View style={styles.row}>
      {goals.filter(g => g.status === 'active' && g.level === (level === 'medium' ? 'long' : 'medium')).map(g => <Chip key={g.id} title={g.title} selected={parent === g.id} onPress={() => setParent(g.id)} disabled={busy} />)}
    </View></>}
    <Field label="训练领域" value={area} onChange={setArea} disabled={busy} />
    <Field label="达成标准" value={criteria} onChange={setCriteria} multiline placeholder="什么观察可以作为达成依据？" disabled={busy} />
    <Button title="保存目标" onPress={() => create({ title, level, parent_id: parent, area, criteria })} disabled={busy} />
  </Card>;
}

function SessionDetail({ session, data, back, goFollowups }: { session: Session; data: AppState; back: () => void; goFollowups: () => void }) {
  return <>
    <View style={styles.sectionHeader}><Text style={styles.title}>训练回看</Text><Button title="返回" secondary onPress={back} /></View>
    <Card><Text style={styles.badge}>{session.business_date} · {session.time_slot} · 已完成</Text>
      <Text style={styles.cardTitle}>{session.activities.join(' · ')}</Text><Hint>{session.organization}</Hint>
      <Text style={styles.label}>课前安排</Text><Text style={styles.body}>{session.plan || '未填写'}</Text>
      <Text style={styles.label}>课后观察</Text><Text style={styles.body}>{session.observation || '未填写'}</Text></Card>
    {session.target_snapshots.map(target => <Card key={target.goal_id}>
      <Text style={styles.cardTitle}>{roles[target.role]} · {target.goal_snapshot.title}</Text>
      <Hint>记录时：{target.goal_snapshot.status === 'active' ? '活动目标' : '归档目标'}</Hint>
      <Text style={styles.label}>计划</Text><Text style={styles.body}>{target.plan || '未填写'}</Text>
      <Text style={styles.label}>实际进度</Text><Text style={styles.body}>{target.progress || '未观察／未填写'}</Text>
      <Text style={styles.badge}>结果：{target.outcome === 'archive' ? '达成归档' : target.outcome === 'adjust' ? '调整' : '继续'}</Text>
      {target.next_step ? <Text style={styles.body}>下一步：{target.next_step}</Text> : null}
    </Card>)}
    {session.has_difficulty && <Card><Text style={styles.cardTitle}>困难</Text><Text style={styles.body}>{session.difficulty}</Text>
      <Hint>{data.difficulties.find(d => d.source_session_id === session.id)?.status === 'resolved' ? '已解决' : '已进入跟进'}</Hint></Card>}
    {session.has_experience && <Card><Text style={styles.cardTitle}>成功经验</Text><Text style={styles.body}>{session.experience}</Text>
      <Hint>{({ verified: '已验证', stopped: '已停用', pending: '待验证' } as Record<string, string>)[data.experiences.find(e => e.source_session_id === session.id)?.status ?? 'pending']}</Hint></Card>}
    {(session.has_difficulty || session.has_experience) && <Button title="查看困难与经验跟进" secondary onPress={goFollowups} />}
  </>;
}

function FollowUpCard({ item, kind, busy, source, update }: {
  item: FollowUp; kind: 'difficulties' | 'experiences'; busy: boolean; source: () => void;
  update: (input: { expected_version: number; status: string; conclusion: string; next_review_date: string | null }) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [status, setStatus] = useState(item.status);
  const [conclusion, setConclusion] = useState(item.conclusion);
  const [date, setDate] = useState(item.next_review_date ?? '');
  useEffect(() => { setStatus(item.status); setConclusion(item.conclusion); setDate(item.next_review_date ?? ''); }, [item.version]);
  const options = kind === 'difficulties' ? [['pending', '待确认'], ['active', '跟进中'], ['resolved', '已解决']] :
    [['pending', '待验证'], ['verified', '已验证'], ['stopped', '停用']];
  return <Card>
    <Text style={styles.badge}>{options.find(o => o[0] === item.status)?.[1]}{item.kind ? ` · ${item.kind === 'general' ? '长期普适' : '中短期任务'}` : ''}</Text>
    <Text style={styles.body}>{item.content}</Text>
    {item.next_review_date && <Hint>下次回看：{item.next_review_date}</Hint>}
    {item.conclusion ? <Hint>结论：{item.conclusion}</Hint> : null}
    <View style={styles.row}><Button title="查看来源训练" secondary onPress={source} /><Button title={expanded ? '收起' : '更新跟进'} secondary onPress={() => setExpanded(v => !v)} /></View>
    {expanded && <>
      <View style={styles.row}>{options.map(([v, title]) => <Chip key={v} title={title} selected={status === v} onPress={() => setStatus(v)} disabled={busy} />)}</View>
      <Field label={kind === 'difficulties' ? '跟进结论' : '验证依据'} value={conclusion} onChange={setConclusion} multiline disabled={busy} />
      <Field label="下次回看日期（可选）" value={date} onChange={setDate} placeholder="YYYY-MM-DD" disabled={busy} />
      <Button title="保存跟进" disabled={busy} onPress={() => update({ expected_version: item.version, status, conclusion, next_review_date: date || null })} />
    </>}
  </Card>;
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: colors.cream, alignItems: 'center' },
  shell: { flex: 1, width: '100%', backgroundColor: colors.cream },
  header: { paddingHorizontal: 22, paddingTop: 20, paddingBottom: 15, flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' },
  brand: { flexDirection: 'row', alignItems: 'center', gap: 8 }, star: { color: colors.green, fontSize: 30 },
  brandName: { color: colors.ink, fontSize: 26, fontWeight: '700', letterSpacing: -0.7 },
  prototype: { fontSize: 11, color: colors.muted, backgroundColor: '#ECEEE7', padding: 7, borderRadius: 20 },
  content: { paddingHorizontal: 20, paddingBottom: 28, gap: 13 },
  title: { color: colors.ink, fontSize: 25, fontWeight: '700', flexShrink: 1 },
  subtitle: { color: colors.ink, fontSize: 18, fontWeight: '600', marginTop: 12, marginBottom: 2 },
  sectionHeader: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', gap: 10 },
  hero: { padding: 25, borderRadius: 24, backgroundColor: '#DCEADD', gap: 15, marginTop: 3 },
  eyebrow: { fontSize: 12, color: colors.green, letterSpacing: 0.3 },
  heroTitle: { fontSize: 30, fontWeight: '700', lineHeight: 41, color: colors.ink },
  heroText: { color: '#536A5C', lineHeight: 23, fontSize: 14 },
  card: { backgroundColor: colors.white, borderRadius: 18, padding: 18, borderWidth: 1, borderColor: colors.border, gap: 9 },
  cardTitle: { fontSize: 16, fontWeight: '600', color: colors.ink, lineHeight: 24, flexShrink: 1 },
  body: { color: colors.ink, fontSize: 14, lineHeight: 23 },
  hint: { color: colors.muted, fontSize: 12, lineHeight: 20 },
  badge: { color: colors.green, fontSize: 12, lineHeight: 20 },
  button: { backgroundColor: colors.green, paddingHorizontal: 17, paddingVertical: 13, borderRadius: 12, alignItems: 'center', justifyContent: 'center', minHeight: 44 },
  secondaryButton: { backgroundColor: colors.light },
  buttonText: { color: colors.white, fontSize: 14, fontWeight: '600' },
  row: { flexDirection: 'row', flexWrap: 'wrap', gap: 8, marginVertical: 3 },
  chip: { paddingHorizontal: 13, paddingVertical: 11, backgroundColor: '#F5F6F2', borderWidth: 1, borderColor: colors.border, borderRadius: 11, minHeight: 42 },
  selectedChip: { borderColor: colors.green, backgroundColor: colors.light },
  chipText: { color: colors.muted, fontSize: 13 },
  label: { color: colors.ink, fontSize: 13, fontWeight: '500', marginTop: 10, marginBottom: 4 },
  field: { gap: 3 },
  input: { borderWidth: 1, borderColor: colors.border, paddingHorizontal: 12, paddingVertical: 12, borderRadius: 10, color: colors.ink, backgroundColor: '#FCFCF9', fontSize: 14, minHeight: 45 },
  textarea: { minHeight: 90, textAlignVertical: 'top', lineHeight: 23 },
  target: { borderTopWidth: 1, borderTopColor: colors.border, paddingTop: 14, gap: 8, marginTop: 8 },
  context: { color: colors.green, backgroundColor: colors.light, padding: 10, borderRadius: 8, fontSize: 12, lineHeight: 20 },
  candidates: { padding: 12, backgroundColor: '#F8F9F5', borderRadius: 12, gap: 9 },
  link: { color: colors.green, fontSize: 12, padding: 8 },
  stats: { flexDirection: 'row', gap: 10 },
  stat: { flex: 1, backgroundColor: colors.white, padding: 15, borderRadius: 16, alignItems: 'center', gap: 5 },
  statNumber: { fontSize: 27, color: colors.ink, fontWeight: '600' },
  navigation: { flexDirection: 'row', backgroundColor: colors.white, borderTopWidth: 1, borderColor: colors.border, paddingBottom: 12, paddingTop: 10 },
  navItem: { flex: 1, alignItems: 'center', gap: 3, paddingVertical: 4 },
  navIcon: { fontSize: 22, color: '#89968C' }, navLabel: { fontSize: 11, color: '#89968C' },
  notice: { backgroundColor: colors.light, padding: 14, borderRadius: 12 }, noticeText: { color: colors.green, fontSize: 13, lineHeight: 21 },
  error: { backgroundColor: '#FBECE2', padding: 14, borderRadius: 12 }, errorText: { color: '#925231', fontSize: 13, lineHeight: 21 },
  footer: { textAlign: 'center', color: '#97A098', fontSize: 11, marginTop: 20 },
});

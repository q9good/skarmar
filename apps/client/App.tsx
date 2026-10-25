import { Pressable, Text, TextInput, ReadabilityProvider } from './src/readability';
import { DetailOverlay } from './src/overlay';
import { FoundationWorkspace } from './src/foundation-view';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import { ActivityIndicator, ScrollView, StyleSheet, View, useWindowDimensions, StatusBar, AppState as NativeAppState, Platform } from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { AuthBoundary, AccountPanel, useAuth } from './src/auth';
import { ExperienceBindings, ExperienceCreator, PathwayLibrary, RelatedKnowledge } from './src/knowledge';
import { PlanningOverview } from './src/planning-view';
import { DateField, Select } from './src/controls';
import { DeleteRecordButton, type DeletedHandler } from './src/delete-record';
import { LessonChoice, LessonStatistics, LessonOptionManager } from './src/lessons';
import { buildPlanning } from './src/planning';
import { mergeState, orderedGoals, replaceTarget, shiftDate } from './src/workflow';
import {
  ApiError, request, isReviewable, isTrainable, emptyLessonLinks, type AppState, type FollowUp, type Goal, type Level,
  type GoalProgress, type GoalUpdate, type GoalRevision, type Pathway, type FollowUpUpdate, type SessionRevision,
  type LessonLinks, type Role, type SaveRequest, type SaveResult, type Session, type SessionInput, type Lesson, type LessonInput, type LessonOption, type NewLessonOption,
} from './src/api';
import { clearDraft, emptyInput, localDate, readDraft, saveDraft, uuid, type Draft } from './src/draft';

const colors = { ink: '#243D36', muted: '#53665A', green: '#34765B', light: '#E9F2E9',
  cream: '#F6F5EF', white: '#FFFFFF', border: '#E0E6DD', orange: '#AA713B' };
const levels: Record<Level, string> = { long: '长期', medium: '中期', short: '短期' };
const progressLabels: Record<GoalProgress, string> = { cultivating: '持续培养中', internalized: '已内化',
  not_started: '未开始', in_progress: '进行中', achieved: '已达成', completed: '已完成', overdue: '已逾期（历史状态）' };
const roles: Record<Role, string> = { primary: '主目标', secondary: '副目标', review: '复习' };
const activities = ['桌面教学', '感统活动', '游戏', '家务劳动', '生活自理', '户外活动', '阅读'];
const tabs = [ ['today', '☀', '今天'], ['goals', '◎', '目标'], ['pathways', '↗', '方法'], ['history', '▤', '记录'], ['followups', '◇', '跟进'] ] as const;
type Tab = typeof tabs[number][0];
const emptyState: AppState = { goals: [], sessions: [], difficulties: [], experiences: [], pathways: [], lessons: [] };
type LessonDraft = { id: string; expectedVersion: number | null; input: LessonInput; difficultyVersion?: number | null };
const emptyLesson = (): LessonInput => ({ business_date: localDate(), course: '', teacher: '', content: '', goals: '', problems: '', home_cooperation: '', links: emptyLessonLinks() });
const lessonInput = ({ id: _id, version: _version, target_snapshots: _snapshots, ...input }: Lesson): LessonInput => input;
function readLessonDraft(raw: string | null): LessonDraft | null {
  if (!raw) return null;
  try {
    const value: LessonDraft = JSON.parse(raw);
    const fields: (keyof LessonInput)[] = ['business_date', 'course', 'teacher', 'content', 'goals', 'problems', 'home_cooperation'];
    if (typeof value.id === 'string' && (value.expectedVersion === null || Number.isSafeInteger(value.expectedVersion)) &&
      fields.every(key => typeof value.input?.[key] === 'string')) {
      const links = value.input.links;
      const ids = (values: unknown): values is string[] => Array.isArray(values) && values.every(v => typeof v === 'string');
      if (links !== undefined && (!links || !Array.isArray(links.targets) || typeof links.track_problem !== 'boolean' ||
        !ids(links.pathway_ids) || !ids(links.problem_goal_ids) || !ids(links.problem_pathway_ids) ||
        !links.targets.every(t => t && typeof t.goal_id === 'string' && ['primary','secondary','review'].includes(t.role) && typeof t.plan === 'string' && typeof t.progress === 'string'))) return null;
      return value;
    }
  } catch { /* Discard an unreadable local draft. */ }
  return null;
}

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
      editable={!disabled} multiline={multiline} placeholder={placeholder} placeholderTextColor="#58695E"
      style={[styles.input, multiline && styles.textarea]} />
  </View>;
}

function Card({ children, testID }: { children: React.ReactNode; testID?: string }) { return <View testID={testID} style={styles.card}>{children}</View>; }
function Hint({ children }: { children: React.ReactNode }) { return <Text style={styles.hint}>{children}</Text>; }

function inputOf(session: Session): SessionInput {
  const { id, version, status, target_snapshots, ...input } = session;
  return input;
}

export default function App() {
  return <ReadabilityProvider><AuthBoundary><WorkspaceRouter /></AuthBoundary></ReadabilityProvider>;
}

function WorkspaceRouter() {
  const auth = useAuth();
  const [model, setModel] = useState<'legacy' | 'prd_v1' | null>(null);
  const [error, setError] = useState('');
  const load = useCallback(async () => {
    try { const value = await request<{ model: string }>('/runtime');
      if (value.model !== 'legacy' && value.model !== 'prd_v1') throw new Error('页面与服务版本不匹配，请保留填写后更新页面。');
      setModel(value.model); setError('');
    } catch (e) { setError(e instanceof Error ? e.message : '暂时无法连接'); }
  }, []);
  useEffect(() => { void load(); }, [load]);
  if (!model) return <View style={{ padding: 24, gap: 16 }}>{error ? <><Text accessibilityRole="alert">{error}</Text><Button title="重试连接" onPress={() => void load()} /></> : <ActivityIndicator accessibilityLabel="正在读取服务信息" />}</View>;
  return model === 'prd_v1' ? <FoundationWorkspace key={`${auth.identity.space}:${auth.identity.user.id}`} /> : <Workspace />;
}

function Workspace() {
  const auth = useAuth();
  const scope = `${auth.identity.space}:${auth.identity.user.id}`;
  const lessonStorageKey = `skarma:${scope}:lesson-draft`;
  const [showAccount, setShowAccount] = useState(false);
  const [data, setData] = useState<AppState>(emptyState);
  const [tab, setTab] = useState<Tab>('today');
  const [recordKind, setRecordKind] = useState<'lessons' | 'interventions'>('lessons');
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [draft, setDraft] = useState<Draft | null>(null);
  const [editing, setEditing] = useState(false);
  const [stage, setStage] = useState<'plan' | 'summary'>('plan');
  const [showPlanning, setShowPlanning] = useState(false);
  const [localSaved, setLocalSaved] = useState(false);
  const [storageReady, setStorageReady] = useState(false);
  const [detail, setDetail] = useState<Session | null>(null);
  const [lessonDraft, setLessonDraft] = useState<LessonDraft | null>(null);
  const [lessonDraftReady, setLessonDraftReady] = useState(false);
  const [lessonLocalSaved, setLessonLocalSaved] = useState(false);
  const [editingLesson, setEditingLesson] = useState(false);
  const [selectedGoal, setSelectedGoal] = useState<Goal | null>(null);
  const [showGoalForm, setShowGoalForm] = useState(false);
  const [showExperienceForm, setShowExperienceForm] = useState(false);
  const [pathwayToOpen, setPathwayToOpen] = useState<Pathway | undefined>();
  const [conflict, setConflict] = useState<Session | null>(null);
  const [today, setToday] = useState(localDate);
  const [showDirections, setShowDirections] = useState(false);
  const [showInterventionChoice, setShowInterventionChoice] = useState(false);
  const [syncError, setSyncError] = useState('');
  const [lastSynced, setLastSynced] = useState('');
  const scroll = useRef<ScrollView>(null);
  const refreshSequence = useRef(0);
  const mounted = useRef(true);
  const persistence = useRef<Promise<void>>(Promise.resolve());
  const lessonPersistence = useRef<Promise<void>>(Promise.resolve());
  const { width } = useWindowDimensions();

  const persist = useCallback((next: Draft) => {
    const task = persistence.current.catch(() => undefined).then(() => saveDraft(scope, next));
    persistence.current = task;
    return task;
  }, [scope]);

  const refresh = useCallback(async () => {
    const sequence = ++refreshSequence.current;
    const next = await request<AppState>('/state');
    const normalized = { ...next, lessons: next.lessons ?? [], lesson_options: next.lesson_options ?? [] };
    if (mounted.current && sequence === refreshSequence.current) {
      setData(current => mergeState(current, normalized));
      setSyncError(''); setLastSynced(new Date().toLocaleTimeString('zh-CN', { timeZone: 'Asia/Shanghai' }));
    }
    return normalized;
  }, []);

  const removed: DeletedHandler = (kind, id) => {
    setData(current => mergeState(current, { ...current, deleted: [...(current.deleted ?? []), { kind, id }] }));
    if (kind === 'goals') setSelectedGoal(null);
    if (kind === 'sessions') {
      setDetail(null); setEditing(false);
      if (draft?.id === id) { setDraft(null); void persistence.current.then(() => clearDraft(scope)).catch(() => setError('记录已删除，本机草稿清理失败，请勿重新保存。')); }
    }
    if (kind === 'lessons') {
      setEditingLesson(false);
      if (lessonDraft?.id === id) { setLessonDraft(null); void lessonPersistence.current.then(() => AsyncStorage.removeItem(lessonStorageKey)).catch(() => setError('记录已删除，本机上课草稿清理失败。')); }
    }
    setMessage('记录已删除，其他记录保留。');
    void refresh().catch(() => setSyncError('记录已删除，列表暂未刷新。'));
  };
  useEffect(() => {
    const gone = (kind: string, id?: string) => data.deleted?.some(item => item.kind === kind && item.id === id);
    if (gone('goals', selectedGoal?.id)) setSelectedGoal(null);
    if (gone('sessions', detail?.id)) setDetail(null);
  }, [data.deleted, selectedGoal?.id, detail?.id]);
  const deletedDraft = data.deleted?.some(item => item.kind === 'sessions' && item.id === draft?.id);
  const deletedLesson = data.deleted?.some(item => item.kind === 'lessons' && item.id === lessonDraft?.id);

  const scrollTop = () => scroll.current?.scrollTo({ y: 0, animated: false });
  useEffect(() => { scrollTop(); }, [tab, editing, editingLesson, selectedGoal?.id, detail?.id, showAccount]);
  useEffect(() => { if (error || message) scrollTop(); }, [error, message]);
  useEffect(() => {
    mounted.current = true;
    const sync = () => {
      setToday(localDate());
      if (Platform.OS !== 'web' || document.visibilityState === 'visible') {
        void refresh().catch(() => { if (mounted.current) setSyncError('自动刷新暂时失败，填写内容仍保留。请检查网络或手动刷新。'); });
      }
    };
    const interval = setInterval(sync, 20000);
    const subscription = NativeAppState.addEventListener('change', state => { if (state === 'active') sync(); });
    if (Platform.OS === 'web') { window.addEventListener('focus', sync); document.addEventListener('visibilitychange', sync); }
    return () => {
      mounted.current = false; ++refreshSequence.current; clearInterval(interval); subscription.remove();
      if (Platform.OS === 'web') { window.removeEventListener('focus', sync); document.removeEventListener('visibilitychange', sync); }
    };
  }, [refresh]);

  useEffect(() => {
    let active = true;
    Promise.allSettled([refresh(), readDraft(scope), AsyncStorage.getItem(lessonStorageKey)]).then(results => {
      if (!active) return;
      const [remote, local, localLesson] = results;
      if (remote.status === 'rejected') setError('暂时无法连接，请重试。已有本机草稿仍可继续编辑。');
      if (local.status === 'fulfilled') { setDraft(local.value); setStage(local.value?.stage ?? (local.value?.version || local.value?.pending?.action === 'complete' ? 'summary' : 'plan')); setStorageReady(true); }
      else setError('无法读取本机草稿，请保留浏览器数据后重试。');
      if (localLesson.status === 'fulfilled') { setLessonDraft(readLessonDraft(localLesson.value)); setLessonDraftReady(true); }
      else setError('无法读取本机上课草稿，请保留浏览器数据后重试。');
      setLoading(false);
    });
    return () => { active = false; };
  }, [refresh, scope, lessonStorageKey]);

  useEffect(() => {
    if (!draft) return;
    let active = true;
    setLocalSaved(false);
    persist(draft).then(() => { if (active) setLocalSaved(true); })
      .catch(() => { if (active) setError('本机草稿保存失败，请勿关闭页面。'); });
    return () => { active = false; };
  }, [draft, persist]);

  useEffect(() => {
    if (!lessonDraftReady) return;
    setLessonLocalSaved(false);
    const raw = lessonDraft ? JSON.stringify(lessonDraft) : null;
    const task = lessonPersistence.current.catch(() => undefined).then(() =>
      raw ? AsyncStorage.setItem(lessonStorageKey, raw) : AsyncStorage.removeItem(lessonStorageKey));
    lessonPersistence.current = task;
    task.then(() => setLessonLocalSaved(true))
      .catch(() => setError('本机上课草稿保存失败，请勿关闭页面。'));
  }, [lessonDraft, lessonDraftReady, lessonStorageKey]);

  const safely = async (action: () => Promise<void>) => {
    setBusy(true); setError(''); setMessage('');
    try { await action(); return true; }
    catch (e) { setError(e instanceof Error ? e.message : '操作未完成，请重试'); return false; }
    finally { setBusy(false); }
  };

  const dirty = () => draft && (draft.pending || draft.revision || (draft.version === null
    ? JSON.stringify(draft.input) !== JSON.stringify(emptyInput())
    : !data.sessions.some(s => s.id === draft.id && s.version === draft.version && JSON.stringify(inputOf(s)) === JSON.stringify(draft.input))));

  const startNewAt = (nextStage: 'plan' | 'summary') => {
    if (!storageReady) { setError('无法读取已有本机草稿，请先恢复存储访问后继续。'); return; }
    if (dirty()) { setEditing(true); setError('请先保存当前草稿，再创建另一条训练。'); return; }
    try {
      setDraft({ id: uuid(), version: null, input: emptyInput(), stage: nextStage });
      setStage(nextStage); setShowPlanning(false);
      setEditing(true); setEditingLesson(false); setDetail(null); setConflict(null); setError(''); setMessage('');
    } catch (e) { setError((e as Error).message); }
  };

  const startNew = () => startNewAt('plan');

  const planGoal = (goal: Goal) => {
    if (!storageReady) { setError('无法读取已有本机草稿，请先恢复存储访问后继续。'); return; }
    if (draft?.pending) { setEditing(true); setError('请先确认上次提交结果，再添加目标。'); return; }
    try {
      const next = draft ?? { id: uuid(), version: null, input: emptyInput() };
      if (!next.input.targets.some(target => target.goal_id === goal.id)) {
        const role: Role = isReviewable(goal) ? 'review' : next.input.targets.some(target => target.role === 'primary') ? 'secondary' : 'primary';
        setDraft({ ...next, stage: 'plan', input: { ...next.input, targets: [...next.input.targets, { goal_id: goal.id, role, plan: '', progress: '', outcome: null, next_step: '' }] } });
      }
      setEditing(true); setEditingLesson(false); setDetail(null); setSelectedGoal(null); setConflict(null); setError('');
      setStage('plan'); setShowPlanning(false);
      setMessage('目标已加入当前计划，请核对角色与活动安排。');
    } catch (error) { setError((error as Error).message); }
  };

  const openSession = (session: Session) => {
    if (session.status === 'completed') { setDetail(session); setEditing(false); setSelectedGoal(null); return; }
    if (!storageReady) { setError('无法读取已有本机草稿，请先恢复存储访问后继续。'); return; }
    if (draft?.id !== session.id && dirty()) { setError('当前本机草稿尚未提交，请先保存后切换。'); return; }
    setDraft(draft?.id === session.id ? { ...draft, stage: 'summary' } : { id: session.id, version: session.version, input: inputOf(session), stage: 'summary' });
    setEditing(true); setEditingLesson(false); setDetail(null); setConflict(null); setError(''); setSelectedGoal(null);
    setStage('summary'); setShowPlanning(false);
  };

  const reviseSession = (session: Session) => {
    if (!storageReady) { setError('无法读取本机草稿，请先恢复存储访问。'); return; }
    if (draft && dirty()) { setError('请先保存当前本机草稿，再修订训练。'); return; }
    setDraft({ id: session.id, version: session.version, input: inputOf(session), stage: 'summary', revision: {
      note: '', difficulty_version: data.difficulties.find(item => item.source_session_id === session.id)?.version ?? null,
      experience_version: data.experiences.find(item => item.source_session_id === session.id)?.version ?? null,
    } });
    setStage('summary'); setEditing(true); setDetail(null); setSelectedGoal(null); setConflict(null); setError(''); setMessage('');
  };

  const change = (value: Partial<SessionInput>) => {
    if (!draft || busy || draft.pending) return;
    setDraft({ ...draft, input: { ...draft.input, ...value } }); setMessage('');
  };

  useEffect(() => {
    if (!editing || !draft || draft.pending) return;
    const latest = data.sessions.find(session => session.id === draft.id);
    if (latest && latest.version !== draft.version) setConflict(latest);
  }, [editing, draft?.id, draft?.version, draft?.pending, data.sessions]);

  const lessonDirty = () => lessonDraft && JSON.stringify(lessonDraft.input) !== JSON.stringify(
    data.lessons.find(item => item.id === lessonDraft.id) ? lessonInput(data.lessons.find(item => item.id === lessonDraft.id)!) : emptyLesson());
  const startLesson = () => {
    if (!lessonDraftReady) { setError('无法读取已有上课草稿，请先恢复本机存储访问。'); return; }
    if (!lessonDraft || !lessonDirty()) setLessonDraft({ id: uuid(), expectedVersion: null, input: emptyLesson() });
    setEditingLesson(true); setEditing(false); setDetail(null); setSelectedGoal(null); setError('');
  };
  const openLesson = (lesson: Lesson) => {
    if (lessonDraft?.id === lesson.id && lessonDirty()) {
      setEditingLesson(true); setError(''); return;
    }
    if (lessonDraft && lessonDraft.id !== lesson.id && lessonDirty()) {
      setEditingLesson(true); setError('请先保存当前上课记录，再打开另一条。'); return;
    }
    setLessonDraft({ id: lesson.id, expectedVersion: lesson.version, input: lessonInput(lesson), difficultyVersion: data.difficulties.find(item => item.source_lesson_id === lesson.id && !item.source_deleted)?.version ?? null });
    setEditingLesson(true); setEditing(false); setDetail(null); setSelectedGoal(null); setError('');
  };
  const saveLesson = () => safely(async () => {
    if (!lessonDraft) return;
    let saved: Lesson;
    try {
      saved = await request<Lesson>(`/lessons/${lessonDraft.id}`, 'PUT', {
        expected_version: lessonDraft.expectedVersion, lesson: lessonDraft.input, difficulty_version: lessonDraft.difficultyVersion ?? null,
      });
    } catch (e) { if (e instanceof ApiError && e.status === 409) await refresh().catch(() => undefined); throw e; }
    setData(current => mergeState(current, { ...current, lessons: [saved, ...current.lessons.filter(item => item.id !== saved.id)] }));
    await lessonPersistence.current;
    await AsyncStorage.removeItem(lessonStorageKey);
    setLessonDraft(null); setEditingLesson(false); setMessage('上课记录已保存。');
    try { await refresh(); } catch { setError('记录已保存，列表暂未刷新。请稍后刷新查看。'); }
  });

  const submit = (action: SaveRequest['action']) => safely(async () => {
    if (!draft) return;
    // Retain the exact request before sending. An uncertain response reuses the same operation ID.
    const pending: SaveRequest = draft.pending ?? { operation_id: uuid(), expected_version: draft.version, action: draft.revision ? 'revise' : action, session: draft.input, ...(draft.revision ? { revision: draft.revision } : {}) };
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
    setData(current => mergeState(current, { ...current, sessions: [result.session, ...current.sessions.filter(s => s.id !== result.session.id)] }));
    if (result.session.status === 'completed') {
      setDraft(null); setEditing(false); setDetail(result.session);
      // Await prior writes before clearing, so a delayed autosave cannot resurrect the draft.
      await persistence.current;
      await clearDraft(scope);
      setMessage(pending.action === 'revise' ? '训练修改已保存，历史版本已保留。' : '这次训练已完成，困难与经验已按填写内容保存。');
    } else {
      const saved: Draft = { id: result.session.id, version: result.session.version, input: inputOf(result.session), stage: 'summary' };
      await persist(saved); setDraft(saved);
      setEditing(false); setShowPlanning(false); setStage('summary'); setTab('today');
      setMessage(pending.action === 'save_draft' ? '草稿已保存到服务器。干预后从今天的安排填写总结。' : '计划已保存。');
    }
    // A refresh failure must not turn a confirmed save into an uncertain mutation.
    try { await refresh(); } catch { setError('记录已保存，列表暂未刷新。请稍后刷新查看。'); }
  });

  const resolveConflict = (useRemote: boolean) => safely(async () => {
    if (!draft || !conflict) return;
    await AsyncStorage.setItem(`skarma:${scope}:conflict-backup:${draft.id}`, JSON.stringify(draft));
    if (conflict.status === 'completed') {
      setDraft(null); await persistence.current; await clearDraft(scope);
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

  const todaySessions = data.sessions.filter(s => s.business_date === today);
  const todayLessons = data.lessons.filter(item => item.business_date === today);
  const due = data.difficulties.filter(d => d.status !== 'resolved' && d.next_review_date && d.next_review_date <= today);
  const dueGoals = data.goals.filter(g => g.next_review_date && g.next_review_date <= today);
  const currentGoal = selectedGoal ? data.goals.find(goal => goal.id === selectedGoal.id) ?? selectedGoal : null;
  const remoteLesson = lessonDraft?.expectedVersion !== null ? data.lessons.find(item => item.id === lessonDraft?.id) : undefined;
  const currentLessonDifficulty = data.difficulties.find(item=>item.source_lesson_id===lessonDraft?.id&&!item.source_deleted);
  const lessonStale = Boolean(remoteLesson && (remoteLesson.version !== lessonDraft?.expectedVersion || (lessonDraft?.input.links?.track_problem && (currentLessonDifficulty?.version ?? null) !== (lessonDraft?.difficultyVersion ?? null))));
  const longDirections = data.goals.filter(goal => goal.level === 'long' && goal.status !== 'archived');
  const pendingToday = todaySessions.filter(session => session.status === 'draft');
  const pageTitle = { today: '今天', goals: '目标', pathways: '方法', history: '记录', followups: '跟进' }[tab];
  const coverage = buildPlanning(data, { today, days: 30, level: 'short', includeArchived: false });
  const acceptOption = (saved: LessonOption) => setData(current => ({ ...current,
    lesson_options: [...(current.lesson_options ?? []).filter(item => item.kind !== saved.kind || item.value !== saved.value), saved] }));
  const updateFollowUp = async (kind: 'difficulties' | 'experiences', id: string, body: FollowUpUpdate) => {
    try {
      const saved = await request<FollowUp>(`/${kind}/${id}`, 'PUT', body);
      setData(current => ({ ...current, [kind]: current[kind].map(item => item.id === id ? saved : item) }));
      setMessage(kind === 'experiences' ? '验证记录已保存。' : '跟进已保存。');
      try { await refresh(); } catch { setSyncError('跟进已保存，自动刷新暂时失败。'); }
      return saved;
    } catch (e) { if (e instanceof ApiError && e.status === 409) await refresh().catch(() => undefined); throw e; }
  };

  return <View style={styles.root}>
    <StatusBar barStyle="dark-content" />
    <View style={[styles.shell, width > 700 && { maxWidth: 960 }]}>
      <View style={styles.header}>
        <View style={styles.brand}><Text style={styles.star}>✦</Text><Text style={styles.brandName}>skarmar</Text></View>
        <Pressable accessibilityRole="button" accessibilityLabel="我的账号" style={{ maxWidth: '55%', flexShrink: 1 }} disabled={busy || auth.identity.demo} onPress={() => setShowAccount(true)}><Text numberOfLines={1} style={styles.prototype}>{auth.identity.demo ? '体验版 · 示例空间' : `${auth.identity.user.display_name} · 账号`}</Text></Pressable>
      </View>
      <ScrollView ref={scroll} style={{ flex: 1 }} contentContainerStyle={styles.content} keyboardShouldPersistTaps="handled">
        {syncError ? <Hint>{syncError}</Hint> : null}
        {error ? <View accessibilityRole="alert" style={styles.error}><Text style={styles.errorText}>{error}</Text></View> : null}
        {message ? <View accessibilityRole="alert" style={styles.notice}><Text style={styles.noticeText}>{message}</Text></View> : null}
        {showAccount ? <AccountPanel back={() => setShowAccount(false)} logout={async () => { await Promise.all([persistence.current, lessonPersistence.current]); await auth.logout(); }} onImported={refresh} /> : loading ? <ActivityIndicator color={colors.green} style={{ marginVertical: 40 }} /> :
          editingLesson && lessonDraft ? <>
            <View style={styles.sectionHeader}><Text style={styles.title}>{lessonDraft.expectedVersion ? '修改上课记录' : '记录今天上课'}</Text>
              <Button title="返回" secondary onPress={() => setEditingLesson(false)} disabled={busy} /></View>
            <Hint>无需先建训练计划。课程名称和授课老师为必填项。</Hint>
            <Hint>{lessonLocalSaved ? '上课草稿已保存在本机' : '正在保存本机上课草稿…'}</Hint>
            {lessonStale && remoteLesson && <Card>
              <Text style={styles.errorText}>上课记录或关联困难已有新版本，当前填写仍保留。请核对后重新填写修改。</Text>
              <Hint>最新课程：{remoteLesson.course} · 老师：{remoteLesson.teacher}</Hint>
              <Text style={styles.body}>最新内容：{remoteLesson.content || '未填写'}</Text>
              <Button title="备份我的填写并载入最新上课记录" secondary disabled={busy} onPress={() => void safely(async () => {
                await AsyncStorage.setItem(`skarma:${scope}:lesson-conflict-backup:${lessonDraft.id}`, JSON.stringify(lessonDraft));
                setLessonDraft({ id: remoteLesson.id, expectedVersion: remoteLesson.version, input: lessonInput(remoteLesson), difficultyVersion: data.difficulties.find(item => item.source_lesson_id === remoteLesson.id && !item.source_deleted)?.version ?? null });
              })} />
            </Card>}
            {deletedLesson && <><Text accessibilityRole="alert" style={styles.errorText}>记录已被管理员删除，当前本机填写仍保留，无法保存到原记录。</Text>
              <Button title="备份填写并关闭已删除草稿" secondary disabled={busy} onPress={() => void safely(async () => {
                await lessonPersistence.current; await AsyncStorage.setItem(`skarma:${scope}:deleted-lesson-backup:${lessonDraft.id}`, JSON.stringify(lessonDraft));
                await AsyncStorage.removeItem(lessonStorageKey); setLessonDraft(null); setEditingLesson(false); setMessage('已备份本机填写，可以新建其他记录。');
              })} />
            </>}
            <Button title={showPlanning?'收起课程规划提示':'查看课程均衡规划提示'} secondary disabled={busy} onPress={()=>setShowPlanning(!showPlanning)} />
            {showPlanning && <PlanningOverview data={data} today={today} disabled={busy} hasDraft context="lesson" continueDraft={()=>setShowPlanning(false)}
              plan={goal=>setLessonDraft(current=>{if(!current)return current;const links=current.input.links??emptyLessonLinks();if(links.targets.some(t=>t.goal_id===goal.id))return current;return {...current,input:{...current.input,links:{...links,targets:[...links.targets,{goal_id:goal.id,role:links.targets.length?'secondary':'primary',plan:'',progress:''}]}}};})}
              openGoal={goal=>{setEditingLesson(false);setSelectedGoal(goal);}} openPathway={pathway=>{setEditingLesson(false);setPathwayToOpen(pathway);setTab('pathways');}} />}
            <LessonEditor data={data} openGoal={goal=>{setEditingLesson(false);setSelectedGoal(goal);}} openPathway={pathway=>{setEditingLesson(false);setPathwayToOpen(pathway);setTab('pathways');}} addOption={async option => {
              const saved = await request<LessonOption>('/lesson-options', 'POST', option);
              acceptOption(saved);
              if (!saved.active) throw new Error('同名选项已停用，请在记录页的选项维护中恢复使用。');
            }} value={lessonDraft.input} change={patch => setLessonDraft(current => current && !busy ? { ...current, input: { ...current.input, ...patch } } : current)} disabled={busy} />
            {data.difficulties.some(item=>item.source_lesson_id===lessonDraft.id&&!item.source_deleted) && <Button title="查看课程困难跟进" secondary onPress={()=>{setEditingLesson(false);setTab('followups');}} />}
            <Button title={busy ? '正在保存…' : '保存上课记录'} onPress={() => void saveLesson()} disabled={busy || deletedLesson || lessonStale || !lessonDraft.input.course.trim() || !lessonDraft.input.teacher.trim()} />
            {lessonDraft.expectedVersion !== null && <DeleteRecordButton kind="lessons" id={lessonDraft.id} version={lessonDraft.expectedVersion} label="上课记录" hint="来源困难会保留，并标明来源上课已删除。" removed={removed} />}
          </> : editing && draft ? <>
            <View style={styles.sectionHeader}><Text style={styles.title}>{draft.revision ? '修改已完成训练' : stage === 'plan' ? '家庭干预计划' : '填写干预总结'}</Text>
              <Button title="返回" secondary onPress={() => setEditing(false)} disabled={busy} /></View>
            <Hint>{draft.pending ? busy ? '正在保存，请稍候…' : '上次提交结果尚未确认，请重试同一次提交。' :
              localSaved ? '草稿已保存在本机 · 正式保存后可在其他设备继续' : '正在保存本机草稿…'}</Hint>
            <Button title="刷新" secondary disabled={busy} onPress={() => void safely(async () => { await refresh(); })} />
            {stage === 'plan' && <Hint>规划提示：近 30 天有 {coverage.uncovered.length} 个短期目标尚未覆盖。{coverage.uncovered.slice(0, 3).map(row => row.goal.title).join('、')}</Hint>}
            <Button title={showPlanning ? '收起均衡规划提示' : '查看均衡规划提示'} secondary onPress={() => setShowPlanning(!showPlanning)} disabled={busy} />
            {showPlanning && <PlanningOverview data={data} today={today} disabled={busy || !storageReady} hasDraft={true}
              continueDraft={() => setShowPlanning(false)} plan={planGoal} openGoal={goal => { setSelectedGoal(goal); setEditing(false); }}
              openPathway={pathway => { setPathwayToOpen(pathway); setEditing(false); setTab('pathways'); }} />}
            <Button title={stage === 'plan' ? '转到干预总结' : '修改本次计划'} secondary onPress={() => {
              const next = stage === 'plan' ? 'summary' : 'plan'; setStage(next); setDraft({ ...draft, stage: next });
            }} disabled={busy || Boolean(draft.pending)} />
            {conflict ? <Card>
              <Text style={styles.cardTitle}>发现另一个版本</Text>
              <Text style={styles.body}>最新安排：{conflict.plan || '未填写'}</Text>
              <Text style={styles.body}>最新观察：{conflict.observation || '未填写'}</Text>
              <Hint>请核对双方修改。操作前会另存当前草稿为本机备份。</Hint>
              <Button title="采用最新记录" secondary onPress={() => void resolveConflict(true)} disabled={busy} />
              {conflict.status !== 'completed' && <Button title="保留我的内容，核对后继续" secondary onPress={() => void resolveConflict(false)} disabled={busy} />}
            </Card> : null}
            {deletedDraft && <><Text accessibilityRole="alert" style={styles.errorText}>训练已被管理员删除，当前本机填写仍保留，无法保存到原记录。</Text>
              <Button title="备份填写并关闭已删除草稿" secondary disabled={busy} onPress={() => void safely(async () => {
                await persistence.current; await AsyncStorage.setItem(`skarma:${scope}:deleted-training-backup:${draft.id}`, JSON.stringify(draft));
                await clearDraft(scope); setDraft(null); setEditing(false); setMessage('已备份本机填写，可以新建其他记录。');
              })} />
            </>}
            {draft.revision && <Card><Hint>修订保留历史版本，不会再次归档目标。仅在你修改困难或待验证经验的内容／绑定时同步跟进。已验证或停用经验在跟进页单独维护，不会被训练修改覆盖；取消标记不会删除跟进，管理员已删除的跟进不会自动恢复。</Hint>
              <Field label="本次修订说明" value={draft.revision.note} onChange={note => setDraft({ ...draft, revision: { ...draft.revision!, note } })} multiline disabled={busy} />
            </Card>}
            <TrainingEditor stage={stage} pathways={data.pathways} draft={draft} goals={data.goals} sessions={data.sessions} difficulties={data.difficulties}
              disabled={busy || deletedDraft || Boolean(draft.pending)} change={change} />
            {draft.revision && !draft.pending ? <Button title={busy ? '正在保存…' : '保存训练修改'} onPress={() => void submit('revise')} disabled={busy || deletedDraft || !draft.revision.note.trim()} /> : draft.pending ? <Button title={busy ? '正在保存…' : '重试同一次提交'} onPress={() => void submit(draft.pending!.action)} disabled={busy || deletedDraft} /> :
              <View style={styles.row}><Button title={stage === 'plan' ? '保存计划并退出' : '保存总结草稿并退出'} secondary onPress={() => void submit('save_draft')} disabled={busy || deletedDraft} />
                {stage === 'summary' && <Button title={busy ? '正在保存…' : '提交干预总结'} onPress={() => void submit('complete')} disabled={busy || deletedDraft} />}</View>}
            {draft.revision && !draft.pending && !deletedDraft && <Button title="备份填写并取消修改" secondary disabled={busy} onPress={() => void safely(async () => {
              await persistence.current; await AsyncStorage.setItem(`skarma:${scope}:cancelled-revision-backup:${draft.id}`, JSON.stringify(draft));
              await clearDraft(scope); setDraft(null); setEditing(false); setDetail(data.sessions.find(item => item.id === draft.id) ?? null);
              setMessage('本次修改未提交，填写已备份到本机。');
            })} />}
            {draft.version !== null && <DeleteRecordButton kind="sessions" id={draft.id} version={draft.version} label="训练" removed={removed} hint="已有困难与经验会保留，并标明原训练已删除。" />}
          </> : detail ? <SessionDetail edit={reviseSession} removed={removed} session={data.sessions.find(item => item.id === detail.id) ?? detail} data={data} back={() => setDetail(null)} goFollowups={() => { setDetail(null); setTab('followups'); }} />
          : selectedGoal ? <>
            <View style={styles.sectionHeader}><Text style={styles.title}>{currentGoal?.title}</Text><Button title="返回" secondary onPress={() => setSelectedGoal(null)} /></View>
            <RelatedKnowledge data={data} openLesson={openLesson} goalId={selectedGoal.id} openPathway={pathway => { setSelectedGoal(null); setPathwayToOpen(pathway); setTab('pathways'); }} />
            <GoalReview key={selectedGoal.id} removed={removed} feedback={scrollTop} scope={scope} goal={currentGoal!} goals={data.goals}
              source={id => { const s = data.sessions.find(s => s.id === id); if (s) openSession(s); }}
              save={async body => {
                try {
                  const saved = await request<Goal>(`/goals/${selectedGoal.id}`, 'PUT', body);
                  setData(current => ({ ...current, goals: current.goals.map(g => g.id === saved.id ? saved : g) }));
                  setSelectedGoal(saved); return saved;
                } catch (e) {
                  if (e instanceof ApiError && e.status === 409) {
                    const latest = await refresh();
                    setSelectedGoal(latest.goals.find(g => g.id === selectedGoal.id) ?? selectedGoal);
                  }
                  throw e;
                }
              }} />
            <Text style={styles.subtitle}>相关训练</Text>
            {data.sessions.filter(s => s.targets.some(t => t.goal_id === selectedGoal.id)).map(s => <SessionCard key={s.id} session={s} onPress={() => openSession(s)} />)}
            {!data.sessions.some(s => s.targets.some(t => t.goal_id === selectedGoal.id)) && <Hint>开始记录后，这里会呈现相关训练与逐目标结果。</Hint>}
          </> : <>
            <View style={styles.sectionHeader}><Text style={styles.title}>{pageTitle}{tab === 'today' ? ` · ${today.slice(5).replace('-', '/')}` : ''}</Text>
              <Button title="刷新" secondary disabled={busy} onPress={() => void safely(async () => { await refresh(); })} /></View>
            {tab === 'today' && <>
              <Pressable accessibilityRole="button" accessibilityLabel="查看长期关注" onPress={() => setShowDirections(true)}
                style={{ backgroundColor: colors.light, borderRadius: 10, padding: 10, flexDirection: 'row', alignItems: 'center', gap: 6 }}>
                <Text numberOfLines={1} style={{ color: colors.green, fontSize: 14, flex: 1 }}>{longDirections.length
                  ? `长期关注 · ${longDirections.slice(0, 3).map(goal => goal.title).join(' · ')}`
                  : '先写下想长期关注的方向'}</Text>
                <Text style={{ color: colors.green, fontSize: 14 }}>{longDirections.length > 3 ? `另有 ${longDirections.length - 3} 项 ▾` : '▾'}</Text>
              </Pressable>
              <View style={{ gap: 10 }} testID="today-primary-actions">
                <Button title="记录上课" onPress={startLesson} disabled={busy} />
                <Button title={pendingToday.length ? '填写今天的总结' : '记录一次家庭干预'} secondary disabled={busy}
                  onPress={() => pendingToday.length === 1 ? openSession(pendingToday[0]) : setShowInterventionChoice(true)} />
              </View>
              {pendingToday.length > 0 && <Button title="另建干预计划" secondary disabled={busy} onPress={startNew} />}
              {(dueGoals.length > 0 || due.length > 0) && <Card>
                <Text style={styles.cardTitle}>给我的待办</Text>
                <Hint>待复盘目标 {dueGoals.length} 项 · 到期困难 {due.length} 项</Hint>
                {dueGoals.map(goal => <Button key={goal.id} title={`复盘：${goal.title}`} secondary onPress={() => setSelectedGoal(goal)} />)}
                {due.length > 0 && <Button title="查看到期困难" secondary onPress={() => setTab('followups')} />}
              </Card>}
              {draft && (dirty() || !draft.version) ? <Card><Text style={styles.cardTitle}>继续你的草稿</Text><Hint>{draft.input.business_date} · {draft.version ? '已有计划，可继续回填' : '本机尚未提交'}</Hint>
                <Button title="继续填写" secondary onPress={() => setEditing(true)} /></Card> : null}
              {lessonDraft && lessonDirty() ? <Card><Text style={styles.cardTitle}>继续填写上课记录</Text>
                <Hint>草稿仅保存在这台设备，正式保存后其他设备才可查看。</Hint>
                <Button title="继续填写" secondary onPress={() => { setEditingLesson(true); setEditing(false); }} /></Card> : null}
              <Text style={styles.subtitle}>今天的安排</Text>
              {todayLessons.map(item => <LessonCard key={item.id} lesson={item} onPress={() => openLesson(item)} />)}
              {todaySessions.length ? todaySessions.map(s => <SessionCard key={s.id} session={s} onPress={() => openSession(s)} />) :
                !todayLessons.length ? <Card><Text style={styles.cardTitle}>今天还没有记录</Text><Hint>先记课程和老师，或安排一次家庭干预。</Hint></Card> : null}

            </>}
            {tab === 'goals' && <>
              <Hint>目标记录想达到什么，方法记录具体怎样做。长期方向持续关注，中短期目标进入家庭干预。</Hint>

              <Button title={showGoalForm ? '收起新目标' : '＋ 新建目标'} secondary onPress={() => setShowGoalForm(v => !v)} />
              {showGoalForm && <GoalForm goals={data.goals} create={async input => {
                const saved = await request<Goal>('/goals', 'POST', input);
                setData(current => mergeState(current, { ...current, goals: [saved, ...current.goals.filter(goal => goal.id !== saved.id)] }));
                setShowGoalForm(false); setMessage('新目标已保存。');
                try { await refresh(); } catch { setSyncError('目标已保存，自动刷新暂时失败。'); }
              }} />}
              {(['long', 'medium', 'short'] as Level[]).map(level => <View key={level}>
                <Text style={styles.subtitle}>{levels[level]}目标</Text>
                {data.goals.filter(g => g.level === level).map(g => <GoalCard key={g.id} goal={g} data={data} onPress={() => setSelectedGoal(g)} />)}
              </View>)}
            </>}
            {tab === 'pathways' && <PathwayLibrary removed={removed} data={data} openLesson={openLesson} initialPathway={pathwayToOpen} openGoal={setSelectedGoal} saved={(pathway: Pathway) => {
              setData(current => mergeState(current, { ...current, pathways: [pathway, ...current.pathways.filter(item => item.id !== pathway.id)] }));
              setMessage('方法已保存。');
            }} />}
            {tab === 'history' && <>
              <View style={styles.row}>
                <Chip title="机构课程" selected={recordKind === 'lessons'} onPress={() => setRecordKind('lessons')} />
                <Chip title="家庭干预" selected={recordKind === 'interventions'} onPress={() => setRecordKind('interventions')} />
              </View>
              {recordKind === 'lessons' ? <>
                <Hint>机构老师的课程记录，与家人干预分开查看。</Hint>
                <Button title="＋ 新建上课记录" secondary onPress={startLesson} disabled={busy} />
                {lessonDraft && lessonDirty() && <Button title="继续未提交的上课记录" secondary onPress={() => setEditingLesson(true)} />}
                <LessonStatistics lessons={data.lessons} today={today} />
                <LessonOptionManager options={data.lesson_options ?? []} saved={acceptOption} />
                <Text style={styles.subtitle}>上课记录 · {data.lessons.length}</Text>
                {data.lessons.length ? [...data.lessons].sort((a, b) => b.business_date.localeCompare(a.business_date))
                  .map(item => <LessonCard key={item.id} lesson={item} onPress={() => openLesson(item)} />) : <Hint>还没有上课记录。</Hint>}
              </> : <>
                <Hint>家人给予的干预：历史计划和已完成的总结。今天页只展示当天安排。</Hint>
                <Button title="＋ 新建干预计划" secondary onPress={startNew} disabled={busy} />
                {draft && dirty() && <Button title="继续未提交的干预草稿" secondary onPress={() => setEditing(true)} />}
                <Text style={styles.subtitle}>家庭干预记录 · {data.sessions.length}</Text>
                {data.sessions.length ? [...data.sessions].sort((a, b) => b.business_date.localeCompare(a.business_date))
                  .map(session => <SessionCard key={session.id} session={session} onPress={() => openSession(session)} />) : <Hint>还没有家庭干预记录。</Hint>}
              </>}
            </>}
            {tab === 'followups' && <>
              <Hint>家庭干预与机构课程的问题可进入困难跟进；成功片段先保存为待验证经验。</Hint>
              <Text style={styles.subtitle}>困难跟进 · {data.difficulties.length}</Text>
              {data.difficulties.length ? data.difficulties.map(item => <FollowUpCard key={item.id} item={item} kind="difficulties" removed={removed} busy={busy} goals={data.goals} pathways={data.pathways}
                source={() => { const lesson=data.lessons.find(l=>l.id===item.source_lesson_id);if(lesson){openLesson(lesson);return;} const s = data.sessions.find(s => s.id === item.source_session_id); if (s) openSession(s); }}
                update={body => updateFollowUp('difficulties', item.id, body)} />) : <Hint>完成训练时填写困难，这里会保留来源与跟进过程。</Hint>}
              <Button title={showExperienceForm ? '收起新经验' : '＋ 新建经验'} secondary onPress={() => setShowExperienceForm(value => !value)} />
              {showExperienceForm && <ExperienceCreator data={data} cancel={() => setShowExperienceForm(false)} saved={item => {
                setData(current => mergeState(current, { ...current, experiences: [item, ...current.experiences.filter(existing => existing.id !== item.id)] }));
                setShowExperienceForm(false); setMessage('独立经验已保存，待验证。');
              }} />}
              <Text style={styles.subtitle}>成功经验 · {data.experiences.length}</Text>
              {data.experiences.length ? data.experiences.map(item => <FollowUpCard key={item.id} item={item} kind="experiences" removed={removed} busy={busy} goals={data.goals} pathways={data.pathways}
                source={() => { const lesson=data.lessons.find(l=>l.id===item.source_lesson_id);if(lesson){openLesson(lesson);return;} const s = data.sessions.find(s => s.id === item.source_session_id); if (s) openSession(s); }}
                update={body => updateFollowUp('experiences', item.id, body)} />) : <Hint>已保存不等于已验证。复用前，请补充适用情况和验证依据。</Hint>}
            </>}
          </>}
        {lastSynced ? <Text style={styles.footer}>上次同步 {lastSynced} · 页面可见时自动刷新</Text> : null}
      </ScrollView>
      {showDirections && <DetailOverlay title="长期关注" close={() => setShowDirections(false)}>
        {longDirections.length ? longDirections.map(goal => <Card key={goal.id}>
          <Text accessibilityRole="header" style={styles.cardTitle}>{goal.title}</Text>
          <Text style={styles.body}>{goal.criteria || '还没有填写关注说明，可在目标页补充。'}</Text>
          <Text style={styles.label}>最近观察</Text>
          <Text style={styles.body}>{goal.review_notes || '还没有此方向的直接观察记录。'}</Text>
          <RelatedKnowledge data={data} goalId={goal.id}
            openLesson={lesson => { setShowDirections(false); openLesson(lesson); }}
            openPathway={pathway => { setShowDirections(false); setPathwayToOpen(pathway); setTab('pathways'); }} />
          <Button title={`查看目标：${goal.title}`} secondary onPress={() => { setShowDirections(false); setSelectedGoal(goal); }} />
        </Card>) : <>
          <Text>写下希望长期支持的方向，相关经验和困难会在这里一起查看。</Text>
          <Button title="新建长期目标" onPress={() => { setShowDirections(false); setTab('goals'); setShowGoalForm(true); }} />
        </>}
      </DetailOverlay>}
      {showInterventionChoice && <DetailOverlay title="家庭干预" close={() => setShowInterventionChoice(false)}>
        {pendingToday.length > 0 && <>
          <Text>选择今天待总结的计划</Text>
          {pendingToday.map(session => <Button key={session.id} title={`${session.time_slot} · ${session.plan || '未填写安排'}`} secondary
            onPress={() => { setShowInterventionChoice(false); openSession(session); }} />)}
        </>}
        <Button title="准备去做：新建计划" onPress={() => { setShowInterventionChoice(false); startNew(); }} />
        <Button title="已经做完了：填写总结" secondary onPress={() => { setShowInterventionChoice(false); startNewAt('summary'); }} />
      </DetailOverlay>}
      <View style={styles.navigation}>
        {tabs.map(([key, icon, label]) => <Pressable key={key} accessibilityRole="button" accessibilityLabel={label}
          accessibilityState={{ selected: tab === key && !editing }} style={styles.navItem} disabled={busy}
          onPress={() => { setShowAccount(false); setTab(key); setPathwayToOpen(undefined); setEditing(false); setEditingLesson(false); setDetail(null); setSelectedGoal(null); setError(''); setMessage(''); }}>
          <Text style={[styles.navIcon, tab === key && { color: colors.green }]}>{icon}</Text>
          <Text style={[styles.navLabel, tab === key && { color: colors.green, fontWeight: '600' }]}>{label}</Text>
        </Pressable>)}
      </View>
    </View>
  </View>;
}

function LessonEditor({ value, change, disabled, data, addOption, openGoal, openPathway }: {
  value: LessonInput; change: (patch: Partial<LessonInput>) => void; disabled: boolean; data: AppState;
  addOption: (option: NewLessonOption) => Promise<void>; openGoal: (goal: Goal) => void; openPathway: (pathway: Pathway) => void;
}) {
  const links = value.links ?? emptyLessonLinks();
  const patch = (next: Partial<LessonLinks>) => change({ links: { ...links, ...next } });
  const choices = (kind: LessonOption['kind']) => {
    const stopped = new Set((data.lesson_options ?? []).filter(option => option.kind === kind && !option.active).map(option => option.value));
    return [...(data.lesson_options ?? []).filter(option => option.kind === kind && option.active).map(option => option.value), ...data.lessons.map(lesson => kind === 'course' ? lesson.course : lesson.teacher)].filter(name => !stopped.has(name));
  };
  const changeTargets = (targets: LessonLinks['targets']) => patch({ targets: targets.length && !targets.some(t=>t.role==='primary') ? targets.map((t,i)=>i===0?{...t,role:'primary'}:t) : targets });
  const primary = links.targets.find(t=>t.role==='primary');
  const methods = [...data.pathways].sort((a,b)=>Number(b.goal_ids.some(id=>links.targets.some(t=>t.goal_id===id)))-Number(a.goal_ids.some(id=>links.targets.some(t=>t.goal_id===id))));
  return <>
    <Card><Text style={styles.cardTitle}>1 · 上课安排与内容</Text>
      <DateField label="上课日期" value={value.business_date} onChange={business_date => change({ business_date })} disabled={disabled} />
      <LessonChoice label="课程名称（必填）" kind="course" value={value.course} values={choices('course')} change={course => change({ course })} add={addOption} disabled={disabled} />
      <LessonChoice label="授课老师（必填）" kind="teacher" value={value.teacher} values={choices('teacher')} change={teacher => change({ teacher })} add={addOption} disabled={disabled} />
      <Field label="上课内容" value={value.content} onChange={content => change({ content })} multiline disabled={disabled} />
    </Card>
    <Card><Text style={styles.cardTitle}>2 · 本次课程目标</Text>
      <Hint>参照干预计划选择主目标和副目标，逐个记录内容与观察。选择目标后保留一个主目标；将其他目标设为主目标可交换角色。可以先只记录课程，稍后补充关联；上课不会自动改变目标的达成或归档状态。</Hint>
      {links.targets.map((target,index)=>{
        const goal=data.goals.find(g=>g.id===target.goal_id);
        return <View key={target.goal_id} style={styles.target}>
          <Text style={styles.cardTitle}>{roles[target.role]} · {goal?.title ?? '目标待核对'}</Text>
          <Hint>{goal?.criteria}</Hint>
          <Select label={`更换课程${roles[target.role]}`} value={target.goal_id} disabled={disabled}
            options={data.goals.filter(g=>g.level!=='long').map(g=>({value:g.id,label:g.title+(g.status==='archived'?' · 已归档':'')}))}
            onChange={id=>changeTargets(replaceTarget(links.targets.map(t=>({...t,outcome:null,next_step:''})),index,id))} />
          <Select label={`课程目标角色：${goal?.title ?? target.goal_id}`} value={target.role} disabled={disabled}
            options={Object.entries(roles).filter(([value])=>target.role!=='primary'||value==='primary').map(([value,label])=>({value,label}))} onChange={role=>{
              const next=role as Role;
              changeTargets(links.targets.map((t,i)=>i===index?{...t,role:next}:next==='primary'&&t.role==='primary'?{...t,role:target.role==='primary'?'secondary':target.role}:t));
            }} />
          <Field label={`课程${roles[target.role]}内容`} value={target.plan} multiline disabled={disabled} onChange={plan=>patch({targets:links.targets.map((t,i)=>i===index?{...t,plan}:t)})} />
          <Field label={`课程${roles[target.role]}观察`} value={target.progress} multiline disabled={disabled} onChange={progress=>patch({targets:links.targets.map((t,i)=>i===index?{...t,progress}:t)})} />
          <Button title={`移除课程目标：${goal?.title}`} secondary disabled={disabled} onPress={()=>changeTargets(links.targets.filter((_,i)=>i!==index))} />
          {goal && <Button title={`查看课程目标：${goal.title}`} secondary disabled={disabled} onPress={()=>openGoal(goal)} />}
        </View>;
      })}
      <Select label="添加课程目标" value="" disabled={disabled} options={[{value:'',label:links.targets.length?'选择副目标或复习目标':'选择主目标'},...data.goals.filter(g=>g.level!=='long'&&!links.targets.some(t=>t.goal_id===g.id)).map(g=>({value:g.id,label:g.title+(g.status==='archived'?' · 已归档':'')}))]}
        onChange={id=>{if(id)patch({targets:[...links.targets,{goal_id:id,role:links.targets.length?'secondary':'primary',plan:'',progress:''}]});}} />
      <Field label="目标补充说明（保留原文）" value={value.goals} onChange={goals => change({ goals })} multiline disabled={disabled} />
      {!links.targets.length && value.goals ? <Hint>目标原文已保留，尚未选择关联目标；不会根据文字自动猜测关联。</Hint> : null}
    </Card>
    <Card><Text style={styles.cardTitle}>3 · 本次实际使用的方法</Text>
      <Hint>仅选择这节课实际用过的方法。方法的适用目标不会自动成为课程目标；已有关联目标的方法排在前面。</Hint>
      {methods.map(method=><View key={method.id}>
        <Pressable accessibilityRole="checkbox" accessibilityLabel={`课程实际方法：${method.title}`} accessibilityState={{checked:links.pathway_ids.includes(method.id),disabled}} aria-checked={links.pathway_ids.includes(method.id)} disabled={disabled}
          onPress={()=>patch({pathway_ids:links.pathway_ids.includes(method.id)?links.pathway_ids.filter(id=>id!==method.id):[...links.pathway_ids,method.id]})} style={styles.chip}>
          <Text>{links.pathway_ids.includes(method.id)?'✓ ':''}{method.title}{method.status==='archived'?' · 已归档':''}</Text>
        </Pressable>
        {links.pathway_ids.includes(method.id) && <><Hint>{method.steps || method.description}</Hint><Button title={`查看课程方法：${method.title}`} secondary disabled={disabled} onPress={()=>openPathway(method)} /></>}
      </View>)}
      {!methods.length && <Hint>暂无方法，可先保存课程，再到方法页新增。</Hint>}
    </Card>
    <Card><Text style={styles.cardTitle}>4 · 问题与家庭配合</Text>
      <Field label="存在问题" value={value.problems} onChange={problems => change({ problems })} multiline disabled={disabled} />
      <Text style={styles.label}>加入困难跟进</Text><View style={styles.row}>{[false,true].map(v=><Chip key={String(v)} title={v?'需要跟进':'仅保留课程备注'} selected={links.track_problem===v} disabled={disabled} onPress={()=>patch({track_problem:v,...(v&&!links.problem_goal_ids.length&&!links.problem_pathway_ids.length?{problem_goal_ids:primary?[primary.goal_id]:[],problem_pathway_ids:links.pathway_ids}: {})})} />)}</View>
      {links.track_problem && <>
        <Hint>默认关联本次主目标及实际方法，可独立调整。保存后进入困难跟进，保留上课来源；已单独修改或解决的内容不会被静默覆盖，取消此选项也不删除既有困难。</Hint>
        <ExperienceBindings goals={[...data.goals].sort((a,b)=>Number(links.targets.some(t=>t.goal_id===b.id))-Number(links.targets.some(t=>t.goal_id===a.id)))} pathways={methods}
          goalIds={links.problem_goal_ids} pathwayIds={links.problem_pathway_ids} disabled={disabled}
          changeGoals={problem_goal_ids=>patch({problem_goal_ids})} changePathways={problem_pathway_ids=>patch({problem_pathway_ids})} />
      </>}
      <Field label="需要家里配合的" value={value.home_cooperation} onChange={home_cooperation => change({ home_cooperation })} multiline disabled={disabled} />
    </Card>
  </>;
}

function LessonCard({ lesson, onPress }: { lesson: Lesson; onPress: () => void }) {
  return <Pressable accessibilityRole="button" onPress={onPress} style={styles.card}>
    <View style={styles.sectionHeader}><Text style={styles.cardTitle}>{lesson.course}</Text><Text style={styles.badge}>{lesson.business_date}</Text></View>
    <Hint>授课老师：{lesson.teacher}</Hint>
    {lesson.content ? <Text style={styles.body} numberOfLines={2}>{lesson.content}</Text> : null}
  </Pressable>;
}

function GoalCard({ goal, onPress, data }: { goal: Goal; onPress: () => void; data?: AppState }) {
  return <Pressable accessibilityRole="button" onPress={onPress} style={styles.card}>
    <View style={styles.sectionHeader}><Text style={styles.cardTitle}>{goal.title}</Text><Text style={styles.badge}>{goal.status === 'archived' ? '归档' : levels[goal.level]}</Text></View>
    <Hint>{goal.criteria || '尚未填写达成标准'}</Hint>
    {data && <Hint>关联方法 {data.pathways.filter(item => item.goal_ids.includes(goal.id)).length} · 关联经验 {data.experiences.filter(item => item.goal_ids.includes(goal.id) || item.pathway_ids.some(id => data.pathways.some(pathway => pathway.id === id && pathway.goal_ids.includes(goal.id)))).length}</Hint>}
    <Hint>{goal.progress_status ? progressLabels[goal.progress_status] : '状态待核对'}{goal.next_review_date ? ` · 复盘 ${goal.next_review_date}` : ''}</Hint>
    {goal.due_date && goal.due_date < localDate() && isTrainable(goal) && <Text style={styles.badge}>已超过截止日期 · 请复盘</Text>}
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

function TrainingEditor({ draft, goals, sessions, difficulties, disabled, change, stage, pathways }: {
  draft: Draft; goals: Goal[]; sessions: Session[]; difficulties: FollowUp[];
  disabled: boolean; change: (input: Partial<SessionInput>) => void; stage: 'plan' | 'summary'; pathways: Pathway[];
}) {
  const input = draft.input;
  const [adding, setAdding] = useState<Role | null>(null);
  const updateTarget = (index: number, patch: Partial<SessionInput['targets'][number]>) =>
    change({ targets: input.targets.map((t, i) => i === index ? { ...t, ...patch } : t) });
  const candidates = goals.filter(g => (adding === 'review' ? isReviewable(g) : isTrainable(g)) &&
    !input.targets.some(t => t.goal_id === g.id));
  const saved = sessions.find(s => s.id === draft.id);
  const changedGoal = saved?.target_snapshots.some(t => goals.find(g => g.id === t.goal_id)?.version !== t.goal_snapshot.version);
  const primary = input.targets.find(target => target.role === 'primary')?.goal_id;
  const bindings = (kind: 'difficulty' | 'experience') => {
    const goalKey = `${kind}_goal_ids` as const;
    const pathwayKey = `${kind}_pathway_ids` as const;
    const ids = input[goalKey] ?? (primary ? [primary] : []);
    const methods = input[pathwayKey] ?? [];
    return <>
      <Select label={`${kind === 'difficulty' ? '困难' : '经验'}关联目标`} value={ids[0] ?? ''} disabled={disabled}
        options={[{ value: '', label: '仅关联方法（未选方法时默认主目标）' }, ...orderedGoals(goals, input).map(goal => ({ value: goal.id, label: `${input.targets.some(target => target.goal_id === goal.id) ? '本次计划 · ' : ''}${goal.title}` }))]}
        onChange={id => change({ [goalKey]: id ? [id] : [] })} />
      <Select label={`${kind === 'difficulty' ? '困难' : '经验'}关联方法`} value={methods[0] ?? ''} disabled={disabled}
        options={[{ value: '', label: '不关联方法' }, ...[...pathways].sort((a, b) => Number(b.goal_ids.some(id => input.targets.some(t => t.goal_id === id))) - Number(a.goal_ids.some(id => input.targets.some(t => t.goal_id === id)))).map(pathway => ({ value: pathway.id, label: pathway.title }))]}
        onChange={id => change({ [pathwayKey]: id ? [id] : [] })} />
    </>;
  };
  return <>
    {stage === 'plan' && <>
    <Card><Text style={styles.cardTitle}>1 · 课前安排</Text>
      <DateField label="训练日期" value={input.business_date} onChange={v => change({ business_date: v })} disabled={disabled} />
      <Text style={styles.label}>时段</Text><View style={styles.row}>{['上午', '下午', '晚上'].map(v => <Chip key={v} title={v} selected={v === input.time_slot} onPress={() => change({ time_slot: v })} disabled={disabled} />)}</View>
      <Text style={styles.label}>训练活动 · 可多选</Text><View style={styles.row}>{activities.map(v => <Chip key={v} title={v} selected={input.activities.includes(v)} disabled={disabled}
        onPress={() => change({ activities: input.activities.includes(v) ? input.activities.filter(a => a !== v) : [...input.activities, v] })} />)}</View>
      <Text style={styles.label}>组织方式</Text><View style={styles.row}>{['居家', '一对一', '小组课'].map(v => <Chip key={v} title={v} selected={v === input.organization} onPress={() => change({ organization: v })} disabled={disabled} />)}</View>
      <Field label="整次训练安排" value={input.plan} onChange={v => change({ plan: v })} multiline placeholder="准备做什么？在哪里、和谁一起？" disabled={disabled} />
    </Card>
    <Card><Text style={styles.cardTitle}>2 · 本次目标</Text><Hint>正式完成时需要一个主目标，副目标可选。复习目标单独添加。</Hint>
      {changedGoal && <Text style={styles.errorText}>计划关联的目标已更新。请核对最新标准与目标角色，再保存计划确认；直接完成会提示冲突。</Text>}
      {input.targets.map((target, index) => {
        const goal = goals.find(g => g.id === target.goal_id);
        const last = sessions.filter(s => s.id !== draft.id && s.status === 'completed')
          .sort((a, b) => b.business_date.localeCompare(a.business_date))
          .flatMap(s => s.targets.filter(t => t.goal_id === target.goal_id).map(t => ({ date: s.business_date, progress: t.progress })))[0];
        const unresolved = difficulties.filter(d => d.status !== 'resolved' && (d.goal_ids.includes(target.goal_id) || d.pathway_ids.some(id=>pathways.some(path=>path.id===id && path.goal_ids.includes(target.goal_id))) || sessions.some(s => s.id === d.source_session_id && s.targets.some(t => t.goal_id === target.goal_id))));
        return <View style={styles.target} key={target.goal_id}>
          <View style={styles.sectionHeader}><Text style={styles.cardTitle}>{roles[target.role]} · {goal?.title ?? '目标已变更'}</Text>
            <Pressable accessibilityRole="button" accessibilityLabel={`移除${goal?.title}`} disabled={disabled} onPress={() => change({ targets: input.targets.filter((_, i) => i !== index) })}><Text style={styles.link}>移除</Text></Pressable></View>
          <Hint>{goal?.criteria}</Hint>
          <Select label={`更换${roles[target.role]}`} value={target.goal_id} disabled={disabled}
            options={goals.filter(item => item.id === target.goal_id || (target.role === 'review' ? isReviewable(item) : isTrainable(item))).map(item => ({ value: item.id, label: item.title }))}
            onChange={id => change({ targets: replaceTarget(input.targets, index, id) })} />
          {last && <Text style={styles.context}>上次（{last.date}）：{last.progress || '未填写逐目标观察'}</Text>}
          {unresolved.length > 0 && <Hint>相关目标或方法有 {unresolved.length} 条未解决困难，可在跟进页查看。</Hint>}
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
    </>}
    {stage === 'summary' && <Card><Text style={styles.cardTitle}>干预完成总结</Text><Hint>逐个目标填写观察，困难和经验会按所选目标与方法进入跟进。</Hint>
      {input.targets.map((target, index) => <View key={target.goal_id} style={styles.target}>
        <Text style={styles.label}>{roles[target.role]} · {goals.find(g => g.id === target.goal_id)?.title}</Text>
        <Field label={`${roles[target.role]}实际进度`} value={target.progress} onChange={v => updateTarget(index, { progress: v })} multiline placeholder="实际发生了什么？接受了哪些帮助？未观察可以留空。" disabled={disabled} />
        <View style={styles.row}>{([['continue', '继续'], ['archive', '达成归档'], ['adjust', '调整']] as const).map(([value, label]) =>
          <Chip key={value} title={label} selected={target.outcome === value} onPress={() => updateTarget(index, { outcome: value })} disabled={disabled} />)}</View>
        <Field label={`${roles[target.role]}下一步`} value={target.next_step} onChange={v => updateTarget(index, { next_step: v })} placeholder={target.outcome === 'adjust' ? '调整时请填写具体说明' : '可选：下次准备怎样继续'} disabled={disabled} />
      </View>)}
      <Field label="整次训练观察" value={input.observation} onChange={v => change({ observation: v })} multiline placeholder="补充整次活动的情况、环境或值得留意的片段" disabled={disabled} />
      <Text style={styles.label}>本次是否遇到困难？</Text><View style={styles.row}>{[false, true].map(v => <Chip key={String(v)} title={v ? '是' : '否'} selected={input.has_difficulty === v} onPress={() => change({ has_difficulty: v, ...(v && input.difficulty_goal_ids === undefined ? { difficulty_goal_ids: primary ? [primary] : [] } : {}) })} disabled={disabled} />)}</View>
      {input.has_difficulty && <><Field label="困难详情" value={input.difficulty} onChange={v => change({ difficulty: v })} multiline placeholder="描述观察到的困难，完成后进入待确认跟进" disabled={disabled} />{bindings('difficulty')}</>}
      <Text style={styles.label}>本次是否有成功经验？</Text><View style={styles.row}>{[false, true].map(v => <Chip key={String(v)} title={v ? '是' : '否'} selected={input.has_experience === v} onPress={() => change({ has_experience: v, ...(v && input.experience_goal_ids === undefined ? { experience_goal_ids: primary ? [primary] : [] } : {}) })} disabled={disabled} />)}</View>
      {input.has_experience && <>
        <Field label="成功经验详情" value={input.experience} onChange={v => change({ experience: v })} multiline placeholder="做了什么、在什么条件下有帮助？完成后保存为待验证经验" disabled={disabled} />
        {bindings('experience')}
        <View style={styles.row}>{([['task', '中短期任务'], ['general', '长期普适']] as const).map(([v, title]) => <Chip key={v} title={title} selected={input.experience_kind === v} onPress={() => change({ experience_kind: v })} disabled={disabled} />)}</View>
      </>}
    </Card>}
  </>;
}

function GoalForm({ goals, create }: {
  goals: Goal[];
  create: (input: { id: string; title: string; level: Level; parent_id: string | null; area: string; criteria: string }) => Promise<void>;
}) {
  const [id] = useState(uuid);
  const [title, setTitle] = useState('');
  const [level, setLevel] = useState<Level>('short');
  const [parent, setParent] = useState<string | null>(null);
  const [area, setArea] = useState('');
  const [criteria, setCriteria] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const submit = async () => {
    if (busy) return;
    if (!title.trim()) { setError('请填写目标名称。'); return; }
    setBusy(true); setError('');
    try { await create({ id, title, level, parent_id: parent, area, criteria }); }
    catch (e) { setError(e instanceof Error ? e.message : '保存失败，填写内容已保留，请重试。'); }
    finally { setBusy(false); }
  };
  const areas = [...new Set([...goals.map(goal => goal.area).filter(Boolean), '沟通', '社交', '认知', '运动', '生活自理', '情绪与行为'])];
  return <Card><Text style={styles.cardTitle}>新目标</Text>
    <Field label="目标名称" value={title} onChange={setTitle} disabled={busy} />
    <View style={styles.row}>{(['long', 'medium', 'short'] as Level[]).map(v => <Chip key={v} title={levels[v]} selected={v === level} onPress={() => { setLevel(v); setParent(null); }} disabled={busy} />)}</View>
    {level !== 'long' && <Select label="上级目标（可选）" value={parent ?? ''} disabled={busy} onChange={id => setParent(id || null)}
      options={[{ value: '', label: '暂不关联上级' }, ...goals.filter(goal => goal.status === 'active' && goal.level === (level === 'medium' ? 'long' : 'medium')).map(goal => ({ value: goal.id, label: goal.title }))]} />}
    <Hint>可先独立创建目标，之后再补充上级；中期可关联长期，短期可关联中期。</Hint>
    <Select label="选择已有领域分类" value={areas.includes(area) ? area : ''} onChange={setArea} disabled={busy} options={[{ value: '', label: '暂不分类／自行填写' }, ...areas.map(value => ({ value, label: value }))]} />
    <Field label="领域分类（可选）" value={area} onChange={setArea} disabled={busy} />
    <Hint>训练领域是分类标签，例如沟通、社交、认知或运动，便于筛选；它不是上级目标。</Hint>
    <Field label="达成标准" value={criteria} onChange={setCriteria} multiline placeholder="什么观察可以作为达成依据？" disabled={busy} />
    {error && <Text accessibilityRole="alert" style={styles.errorText}>{error}</Text>}
    <Button title={busy ? '正在保存…' : '保存目标'} onPress={() => void submit()} disabled={busy} />
  </Card>;
}

function goalUpdate(goal: Goal): GoalUpdate {
  const { id, level, version, ...fields } = goal;
  return { ...fields, expected_version: version, change_note: '' };
}

function GoalReview({ scope, goal, goals, save, source, feedback, removed }: {
  removed: DeletedHandler;
  feedback: () => void;
  scope: string;
  goal: Goal; goals: Goal[]; save: (body: GoalUpdate) => Promise<Goal>; source: (id: string) => void;
}) {
  const [form, setForm] = useState<GoalUpdate>(() => goalUpdate(goal));
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [history, setHistory] = useState<GoalRevision[]>([]);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  useEffect(() => {
    let active = true;
    request<GoalRevision[]>(`/goals/${goal.id}/history`).then(items => { if (active) setHistory(items); })
      .catch(() => { if (active) setError('修改历史暂未加载，请稍后重试。'); });
    return () => { active = false; };
  }, [goal.id, goal.version]);
  const patch = (values: Partial<GoalUpdate>) => { setForm({ ...form, ...values }); setNotice(''); };
  const stale = form.expected_version !== goal.version;
  const progressOptions: GoalProgress[] = goal.level === 'long' ? ['cultivating', 'internalized'] :
    ['not_started', 'in_progress', 'achieved', 'completed'];
  const submit = async () => {
    setSaving(true); setError(''); setNotice('');
    try {
      const updated = await save({ ...form, change_note: form.change_note.trim() || '目标复盘' });
      setForm(goalUpdate(updated)); setEditing(false); setNotice('目标复盘已保存，修改历史已保留。'); feedback();
    } catch (e) { setError(e instanceof Error ? e.message : '保存失败，请保留填写内容后重试。'); }
    finally { setSaving(false); }
  };
  const useLatest = async () => {
    setSaving(true); setError('');
    try {
      await AsyncStorage.setItem(`skarma:${scope}:goal-conflict-backup:${goal.id}`, JSON.stringify(form));
      setForm(goalUpdate(goal)); setNotice('已采用最新目标，本机原填写内容另存为备份。');
    } catch { setError('备份未保存，当前填写内容已保留。'); }
    finally { setSaving(false); }
  };
  const dateField = (key: 'start_date' | 'due_date' | 'last_review_date' | 'next_review_date' | 'completion_date', label: string) =>
    <DateField key={key} label={label} value={form[key] ?? ''} onChange={v => patch({ [key]: v.trim() || null })}
      disabled={saving} />;
  return <>
    <Card>
      <Text style={styles.badge}>{levels[goal.level]} · {goal.progress_status ? progressLabels[goal.progress_status] : '状态待核对'} · {goal.status === 'archived' ? '已归档' : '活动'}</Text>
      <Text style={styles.body}>{goal.criteria || '尚未填写达成标准'}</Text>
      <Hint>上级：{goals.find(g => g.id === goal.parent_id)?.title ?? '未关联'}</Hint>
      {goal.next_review_date && <Hint>下次复盘：{goal.next_review_date}</Hint>}
      {goal.review_notes && <Text style={styles.body}>复盘：{goal.review_notes}</Text>}
      <Button title={editing ? '复盘内容尚未提交' : '修改／复盘目标'} secondary disabled={editing || saving}
        onPress={() => { setEditing(true); setForm({ ...goalUpdate(goal), last_review_date: localDate(), next_review_date: shiftDate(localDate(), 14) }); setNotice(''); }} />
    </Card>
    {!editing && error ? <Text accessibilityRole="alert" style={styles.errorText}>{error}</Text> : null}
    {notice ? <Text accessibilityRole="alert" style={styles.noticeText}>{notice}</Text> : null}
    {editing && <Card>
      <Text style={styles.cardTitle}>目标复盘</Text>
      <Hint>填写内容需手动保存。培养状态与归档分别记录；修改不会改写过去训练的目标快照。</Hint>
      {stale && <View style={styles.error}>
        <Text style={styles.errorText}>目标已有新版本，当前填写内容仍保留。请核对上方最新目标，再采用最新内容后重新填写修改。</Text>
        <Text style={styles.body}>最新标准：{goal.criteria || '未填写'}</Text>
        <Button title="备份我的填写并采用最新目标" secondary onPress={() => void useLatest()} disabled={saving} />
      </View>}
      <Field label="目标名称" value={form.title} onChange={v => patch({ title: v })} disabled={saving} />
      {goal.level !== 'long' && <><Text style={styles.label}>上级目标</Text><View style={styles.row}>
        <Chip title="暂不关联上级" selected={form.parent_id === null} onPress={() => patch({ parent_id: null })} disabled={saving} />
        {goals.filter(g => g.level === (goal.level === 'medium' ? 'long' : 'medium') && (g.status === 'active' || g.id === form.parent_id))
          .map(g => <Chip key={g.id} title={g.title} selected={form.parent_id === g.id} onPress={() => patch({ parent_id: g.id })} disabled={saving} />)}
      </View></>}
      <Field label="领域分类（可选）" value={form.area} onChange={v => patch({ area: v })} disabled={saving} />
      <Field label="达成标准" value={form.criteria} onChange={v => patch({ criteria: v })} multiline disabled={saving} />
      <Text style={styles.label}>培养／训练状态</Text><View style={styles.row}>
        <Chip title="待核对" selected={form.progress_status === null} onPress={() => patch({ progress_status: null })} disabled={saving} />
        {progressOptions.map(v => <Chip key={v} title={progressLabels[v]} selected={form.progress_status === v} onPress={() => patch({ progress_status: v })} disabled={saving} />)}
      </View>
      {goal.level !== 'long' ? <><Text style={styles.label}>归档状态</Text><View style={styles.row}>
        <Chip title="活动" selected={form.status === 'active'} onPress={() => patch({ status: 'active' })} disabled={saving} />
        <Chip title="已归档" selected={form.status === 'archived'} onPress={() => patch({ status: 'archived' })} disabled={saving} />
      </View></> : <Hint>长期目标内化后继续保留；不设置截止日期或归档。</Hint>}
      {dateField('start_date', '开始日期（可选）')}
      {goal.level !== 'long' && dateField('due_date', '截止日期（可选）')}
      {dateField('last_review_date', '本次复盘日期（可选）')}
      {dateField('next_review_date', '下次复盘日期（可选）')}
      {dateField('completion_date', goal.level === 'long' ? '内化确认日期（可选）' : '达成确认日期（可选）')}
      <Field label="进度与复盘记录" value={form.review_notes} onChange={v => patch({ review_notes: v })} multiline disabled={saving} />
      <Field label="本次修改说明（可选）" value={form.change_note} onChange={v => patch({ change_note: v })} multiline placeholder="为什么修改？保留本次观察与依据" disabled={saving} />
      {error && <Text accessibilityRole="alert" style={styles.errorText}>{error}</Text>}
      <Button title={saving ? '正在保存…' : '保存目标复盘'} onPress={() => void submit()} disabled={saving || stale} />
    </Card>}
    <DeleteRecordButton kind="goals" id={goal.id} version={goal.version} label="目标" removed={removed} hint="有下级目标或训练引用时需先处理；方法与经验会保留并解除关联。" />
    <Text style={styles.subtitle}>目标修改历史 · {history.length}</Text>
    {history.map(revision => <Card key={revision.goal.version}>
      <Text style={styles.badge}>版本 {revision.goal.version} · {new Date(revision.recorded_at).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })}</Text>
      <Text style={styles.cardTitle}>{revision.change_note}</Text>
      <Hint>{revision.goal.progress_status ? progressLabels[revision.goal.progress_status] : '状态待核对'} · {revision.goal.status === 'archived' ? '已归档' : '活动'}</Hint>
      <Text style={styles.body}>{revision.goal.criteria || '未填写标准'}</Text>
      {revision.goal.review_notes && <Text style={styles.body}>{revision.goal.review_notes}</Text>}
      {revision.source_session_id && <Button title="查看引起变更的训练" secondary onPress={() => source(revision.source_session_id!)} />}
    </Card>)}
  </>;
}

function SessionDetail({ session, data, back, goFollowups, edit, removed }: { session: Session; data: AppState; back: () => void; goFollowups: () => void; edit: (session: Session) => void; removed: DeletedHandler }) {
  const [history, setHistory] = useState<SessionRevision[]>([]);
  const [historyError, setHistoryError] = useState('');
  useEffect(() => { let active = true;
    request<SessionRevision[]>(`/sessions/${session.id}/history`).then(value => { if (active) { setHistory(value); setHistoryError(''); } }).catch(() => { if (active) setHistoryError('修订历史暂未加载，请重试。'); });
    return () => { active = false; };
  }, [session.id, session.version]);
  return <>
    <View style={styles.sectionHeader}><Text style={styles.title}>训练回看</Text><Button title="返回" secondary onPress={back} /></View>
    <Button title="修改训练" secondary onPress={() => edit(session)} />
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
      <Hint>{data.difficulties.some(d => d.source_session_id === session.id) ? data.difficulties.find(d => d.source_session_id === session.id)?.status === 'resolved' ? '已解决' : '已进入跟进' : '暂无对应跟进，保留原训练总结'}</Hint></Card>}
    {session.has_experience && <Card><Text style={styles.cardTitle}>成功经验</Text><Text style={styles.body}>{session.experience}</Text>
      <Hint>{data.experiences.some(e => e.source_session_id === session.id) ? ({ verified: '已验证', stopped: '已停用', pending: '待验证' } as Record<string, string>)[data.experiences.find(e => e.source_session_id === session.id)!.status] : '暂无对应跟进，保留原训练总结'}</Hint></Card>}
    {(session.has_difficulty || session.has_experience) && <Button title="查看困难与经验跟进" secondary onPress={goFollowups} />}
    <DeleteRecordButton kind="sessions" id={session.id} version={session.version} label="训练" removed={removed} hint="已有困难与经验会保留，并标明原训练已删除。" />
    <Text style={styles.subtitle}>训练版本与修订历史 · {history.length}</Text>
    {historyError && <Text accessibilityRole="alert" style={styles.errorText}>{historyError}</Text>}
    {history.map(item => <Card key={item.session.version}><Text style={styles.badge}>版本 {item.session.version}{item.recorded_at ? ` · ${new Date(item.recorded_at).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })}` : ''}</Text>
      <Text style={styles.cardTitle}>{item.note}</Text><Hint>{item.session.business_date}</Hint><Text style={styles.body}>安排：{item.session.plan || '未填写'}</Text><Text style={styles.body}>观察：{item.session.observation || '未填写'}</Text>
      {item.session.has_difficulty && <Text style={styles.body}>困难：{item.session.difficulty}</Text>}{item.session.has_experience && <Text style={styles.body}>经验：{item.session.experience}</Text>}
    </Card>)}
  </>;
}

function FollowUpCard({ item, kind, busy, source, update, goals, pathways, removed }: {
  removed: DeletedHandler;
  item: FollowUp; kind: 'difficulties' | 'experiences'; busy: boolean; source: () => void; goals: Goal[]; pathways: Pathway[];
  update: (input: FollowUpUpdate) => Promise<FollowUp>;
}) {
  const [expanded, setExpanded] = useState(false);
  const [status, setStatus] = useState(item.status);
  const [conclusion, setConclusion] = useState(item.conclusion);
  const [date, setDate] = useState(item.next_review_date ?? '');
  const [content, setContent] = useState(item.content);
  const [goalIds, setGoalIds] = useState(item.goal_ids);
  const [pathwayIds, setPathwayIds] = useState(item.pathway_ids);
  const [version, setVersion] = useState(item.version);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  busy = busy || saving;
  const reset = () => { setError(''); setStatus(item.status); setConclusion(item.conclusion); setDate(item.next_review_date ?? ''); setContent(item.content); setGoalIds(item.goal_ids); setPathwayIds(item.pathway_ids); setVersion(item.version); };
  useEffect(() => { if (!expanded) reset(); }, [item.version, expanded]);
  const options = kind === 'difficulties' ? [['pending', '待确认'], ['active', '跟进中'], ['resolved', '已解决']] :
    [['pending', '待验证'], ['verified', '已验证'], ['stopped', '停用']];
  return <Card testID={`followup-${item.id}`}>
    <Text style={styles.badge}>{options.find(o => o[0] === item.status)?.[1]}{item.kind ? ` · ${item.kind === 'general' ? '长期普适' : '中短期任务'}` : ''}</Text>
    <Text style={styles.body}>{item.content}</Text>
    {<>
      <Hint>{item.goal_ids.length ? `关联目标：${item.goal_ids.map(id => goals.find(goal => goal.id === id)?.title ?? '待核对').join('、')}` : '尚未绑定目标'}</Hint>
      <Hint>{item.pathway_ids.length ? `关联方法：${item.pathway_ids.map(id => pathways.find(pathway => pathway.id === id)?.title ?? '待核对').join('、')}` : '尚未绑定方法'}</Hint>
    </>}
    {item.next_review_date && <Hint>下次回看：{item.next_review_date}</Hint>}
    {item.conclusion ? <Hint>{kind === 'experiences' ? '验证依据' : '跟进结论'}：{item.conclusion}</Hint> : null}
    <View style={styles.row}>{!item.source_deleted && (item.source_session_id || item.source_lesson_id) ? <Button title={item.source_lesson_id ? '查看来源上课' : '查看来源训练'} secondary onPress={source} /> : <Hint>{item.source_deleted ? item.source_lesson_id ? '原来源上课记录已删除 · 跟进保留' : '原来源训练已删除 · 跟进保留' : '独立记录 · 无来源训练'}</Hint>}<Button title={expanded ? '收起' : '更新跟进'} secondary disabled={busy} onPress={() => setExpanded(v => !v)} /></View>
    {expanded && <>
      {version !== item.version && <><Hint>记录已被更新，当前填写仍保留。请核对最新记录后再保存。</Hint><Button title="载入最新跟进内容" secondary onPress={reset} disabled={busy} /></>}
      {<>
        <Field label={kind === 'experiences' ? '经验内容' : '困难内容'} value={content} onChange={setContent} multiline disabled={busy} />
        <ExperienceBindings goals={goals} pathways={pathways} goalIds={goalIds} pathwayIds={pathwayIds} changeGoals={setGoalIds} changePathways={setPathwayIds} disabled={busy} />
      </>}
      <View style={styles.row}>{options.map(([v, title]) => <Chip key={v} title={title} selected={status === v} onPress={() => setStatus(v)} disabled={busy} />)}</View>
      <Field label={kind === 'difficulties' ? '跟进结论' : '验证依据'} value={conclusion} onChange={setConclusion} multiline disabled={busy} />
      <DateField label="下次回看日期（可选）" value={date} onChange={setDate} disabled={busy} />
      {error && <Text accessibilityRole="alert" style={styles.errorText}>{error}</Text>}
      <Button title={saving ? '正在保存…' : '保存跟进'} disabled={busy || version !== item.version} onPress={() => void (async () => {
        if (saving) return;
        setSaving(true); setError('');
        try {
          const saved = await update({ expected_version: version, status, conclusion, next_review_date: date || null, content, goal_ids: goalIds, pathway_ids: pathwayIds });
          setVersion(saved.version);
        } catch (e) { setError(e instanceof Error ? e.message : '保存失败，填写内容已保留。'); }
        finally { setSaving(false); }
      })()} />
    </>}
    <DeleteRecordButton kind={kind} id={item.id} version={item.version} label={kind === 'experiences' ? '经验' : '困难'} removed={removed} hint="原训练总结仍保留当时填写内容。" />
  </Card>;
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: colors.cream, alignItems: 'center' },
  shell: { flex: 1, width: '100%', backgroundColor: colors.cream },
  header: { paddingHorizontal: 22, paddingTop: 10, paddingBottom: 8, flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' },
  brand: { flexDirection: 'row', alignItems: 'center', gap: 8 }, star: { color: colors.green, fontSize: 30 },
  brandName: { color: colors.ink, fontSize: 23, fontWeight: '700', letterSpacing: -0.7 },
  prototype: { fontSize: 11, color: colors.muted, backgroundColor: '#ECEEE7', padding: 7, borderRadius: 20 },
  content: { paddingHorizontal: 20, paddingBottom: 28, gap: 13 },
  title: { color: colors.ink, fontSize: 25, fontWeight: '700', flexShrink: 1 },
  subtitle: { color: colors.ink, fontSize: 18, fontWeight: '600', marginTop: 12, marginBottom: 2 },
  sectionHeader: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', gap: 10 },
  card: { backgroundColor: colors.white, borderRadius: 18, padding: 18, borderWidth: 1, borderColor: colors.border, gap: 9 },
  cardTitle: { fontSize: 16, fontWeight: '600', color: colors.ink, lineHeight: 24, flexShrink: 1 },
  body: { color: colors.ink, fontSize: 14, lineHeight: 23 },
  hint: { color: colors.muted, fontSize: 12, lineHeight: 20 },
  badge: { color: colors.green, fontSize: 12, lineHeight: 20 },
  button: { backgroundColor: colors.green, paddingHorizontal: 17, paddingVertical: 13, borderRadius: 12, alignItems: 'center', justifyContent: 'center', minHeight: 44 },
  secondaryButton: { backgroundColor: colors.light },
  buttonText: { color: colors.white, fontSize: 17, fontWeight: '600' },
  row: { flexDirection: 'row', flexWrap: 'wrap', gap: 8, marginVertical: 3 },
  chip: { paddingHorizontal: 13, paddingVertical: 11, backgroundColor: '#F5F6F2', borderWidth: 1, borderColor: colors.border, borderRadius: 11, minHeight: 42 },
  selectedChip: { borderColor: colors.green, backgroundColor: colors.light },
  chipText: { color: colors.muted, fontSize: 13 },
  label: { color: colors.ink, fontSize: 17, fontWeight: '500', marginTop: 10, marginBottom: 4 },
  field: { gap: 3 },
  input: { borderWidth: 1, borderColor: colors.border, paddingHorizontal: 12, paddingVertical: 12, borderRadius: 10, color: colors.ink, backgroundColor: '#FCFCF9', fontSize: 14, minHeight: 45 },
  textarea: { minHeight: 90, textAlignVertical: 'top', lineHeight: 23 },
  target: { borderTopWidth: 1, borderTopColor: colors.border, paddingTop: 14, gap: 8, marginTop: 8 },
  context: { color: colors.green, backgroundColor: colors.light, padding: 10, borderRadius: 8, fontSize: 12, lineHeight: 20 },
  candidates: { padding: 12, backgroundColor: '#F8F9F5', borderRadius: 12, gap: 9 },
  link: { color: colors.green, fontSize: 12, padding: 8 },
  navigation: { flexDirection: 'row', backgroundColor: colors.white, borderTopWidth: 1, borderColor: colors.border, paddingBottom: 12, paddingTop: 10 },
  navItem: { flex: 1, alignItems: 'center', gap: 3, paddingVertical: 4 },
  navIcon: { fontSize: 22, color: '#53665A' }, navLabel: { fontSize: 11, color: '#53665A' },
  notice: { backgroundColor: colors.light, padding: 14, borderRadius: 12 }, noticeText: { color: colors.green, fontSize: 13, lineHeight: 21 },
  error: { backgroundColor: '#FBECE2', padding: 14, borderRadius: 12 }, errorText: { color: '#925231', fontSize: 13, lineHeight: 21 },
  footer: { textAlign: 'center', color: '#53665A', fontSize: 11, marginTop: 20 },
});

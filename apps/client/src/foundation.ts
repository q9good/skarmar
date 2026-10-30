import type {Session} from './prd-sessions';
import type { Lesson, Knowledge, NextAction } from './prd-lessons';
import type { Level } from './api';
export type CatalogSnapshot = { id: string; version: number; kind: 'course' | 'teacher' | 'domain'; name: string };
export type GoalInput = { long_priority?:number|null;resume_on?:string|null;title: string; level: Level; meaning: string; parent_id: string | null; domain: CatalogSnapshot | null; criteria: string; support: string;
  template: 'count' | 'duration' | 'steps' | 'text' | null; state: 'not_started' | 'active' | 'maintaining' | 'paused' | 'achieved' | null;
  archived: boolean; frequency: 'weekly' | 'fortnightly' | 'monthly' | 'none' | null; attention_start: string | null; due: string | null };
export type MethodInput = { basis?:{id:string;version:number}[]; resume_on?:string|null;title: string; description: string; steps: string; observe: string; scope: string; goal_ids: string[]; guidance: string;
  state: 'active' | 'paused' | 'retired' | null; pinned: boolean; needs_attention: boolean; attention_reason: string };
export type CatalogInput = { kind: 'course' | 'teacher' | 'domain'; name: string; active: boolean };
export type RecordOf<T> = { id: string; version: number; input: T };
export type Goal = RecordOf<GoalInput>;
export type Method = RecordOf<MethodInput>&{basis_snapshots?:{id:string;version:number;kind:string;content:string;conditions:string}[]};
export type Catalog = RecordOf<CatalogInput>;
export type Snapshot = { schema: 'skarmar.prd.v1'; goals: Goal[]; methods: Method[]; catalog: Catalog[]; lessons: Lesson[]; sessions:Session[]; knowledge: Knowledge[]; actions: NextAction[] };
export type Mutation = { kind: 'goal'; value: GoalInput } | { kind: 'method'; value: MethodInput } | { kind: 'catalog'; value: CatalogInput };
export type Saved = { kind: 'goal'; value: Goal } | { kind: 'method'; value: Method } | { kind: 'catalog'; value: Catalog };
export type Write = { method_change?:'adjustment'|'correction';method_create_mode?:'full'|'quick';operation_id: string; id: string; expected_version: number | null; change_note: string; mutation: Mutation };
export type Draft = Write & { pending?: Write; conflict_backup?: Mutation };
export type Revision = { entity: Saved; recorded_at: string; actor_id: string; change_note: string; event: string };
export const emptySnapshot: Snapshot = { schema: 'skarmar.prd.v1', goals: [], methods: [], catalog: [], lessons: [], sessions:[], knowledge: [], actions: [] };
export const newGoal = (): GoalInput => ({ title: '', level: 'short', meaning: '', parent_id: null, domain: null, criteria: '', support: '', template: null,
  state: 'active', archived: false, frequency: null, attention_start: null, due: null });
export const newMethod = (): MethodInput => ({ title: '', description: '', steps: '', observe: '', scope: '', goal_ids: [], guidance: '', state: 'active', pinned: false, needs_attention: false, attention_reason: '' });
function merge<T extends { id: string; version: number }>(old: T[], next: T[]): T[] {
  const map = new Map(old.map(item => [item.id, item]));
  for (const item of next) if (!map.has(item.id) || map.get(item.id)!.version <= item.version) map.set(item.id, item);
  return [...map.values()];
}
export const mergeSnapshot = (old: Snapshot, next: Snapshot): Snapshot => ({ schema: next.schema,
  goals: merge(old.goals, next.goals), methods: merge(old.methods, next.methods), catalog: merge(old.catalog, next.catalog), lessons: merge(old.lessons ?? [], next.lessons ?? []), sessions:merge(old.sessions??[],next.sessions??[]),
  knowledge: merge((old.knowledge ?? []).map(k => ({...k, id:k.input.id})), (next.knowledge ?? []).map(k => ({...k, id:k.input.id}))),
  actions: merge((old.actions ?? []).map(k => ({...k, id:k.input.id})), (next.actions ?? []).map(k => ({...k, id:k.input.id}))) });
export const acceptSaved = (state: Snapshot, saved: Saved) => mergeSnapshot(state, { ...emptySnapshot,
  goals: saved.kind === 'goal' ? [saved.value] : [], methods: saved.kind === 'method' ? [saved.value] : [], catalog: saved.kind === 'catalog' ? [saved.value] : [] });
export function findSaved(state: Snapshot, id: string): Saved | undefined {
  const goal = state.goals.find(g => g.id === id); if (goal) return { kind: 'goal', value: goal };
  const method = state.methods.find(g => g.id === id); if (method) return { kind: 'method', value: method };
  const catalog = state.catalog.find(g => g.id === id); if (catalog) return { kind: 'catalog', value: catalog };
}
export function mutationOf(saved: Saved): Mutation {
  return { kind: saved.kind, value: saved.value.input } as Mutation;
}
export const nameOf = (mutation: Mutation) => mutation.kind === 'catalog' ? mutation.value.name : mutation.value.title;
export const levelNames = { long: '长期方向', medium: '中期目标', short: '近期本领' };
export const goalStateName = (g: GoalInput) => ({ not_started: '未开始', active: g.level === 'long' ? '关注中' : '重点推进', maintaining: '维持', paused: '暂停', achieved: '已达成' }[g.state ?? 'active']);
export const longDirectionsOf = (data:Snapshot) => data.goals.filter(g=>g.input.level==='long'&&!g.input.archived)
  .sort((a,b)=>(b.input.long_priority??1)-(a.input.long_priority??1));
export const examples = { count: '例如：5次轮流机会中，3次只需一次提醒', duration: '例如：一起看绘本4分钟，提示两次', steps: '例如：能自己开水龙头，涂皂时需示范', text: '例如：需要帮助时会主动表达' };

/** Reject malformed local queues without deleting the original browser value. */
export function decodeDrafts(raw: string | null): Record<string, Draft> {
  const parsed: unknown = raw ? JSON.parse(raw) : {};
  const record = (v: unknown): v is Record<string, unknown> => Boolean(v && typeof v === 'object' && !Array.isArray(v));
  const validMutation = (m: unknown) => {
    if (!record(m) || !record(m.value)) return false;
    const v = m.value;
    if (m.kind === 'goal') {
      const legacyKeys=Object.keys(newGoal()).filter(k=>!['domain','due','support'].includes(k));
      const domain=v.domain;
      return legacyKeys.every(k => k in v) && (v.long_priority===undefined||v.long_priority===null||Number.isInteger(v.long_priority)&&Number(v.long_priority)>=0&&Number(v.long_priority)<=2) && typeof v.title === 'string' && typeof v.meaning === 'string' && typeof v.criteria === 'string' && ['long','medium','short'].includes(String(v.level)) && typeof v.archived === 'boolean' &&
        (v.support===undefined||typeof v.support==='string') && (v.due===undefined||v.due===null||typeof v.due==='string') &&
        (domain===undefined||domain===null||record(domain)&&typeof domain.id==='string'&&Number.isSafeInteger(domain.version)&&Number(domain.version)>0&&domain.kind==='domain'&&typeof domain.name==='string');
    }
    if (m.kind === 'method') {
      const legacyKeys=Object.keys(newMethod()).filter(k=>!['observe','scope'].includes(k));
      return legacyKeys.every(k => k in v) && typeof v.title === 'string' && typeof v.description === 'string' && typeof v.steps === 'string' && (v.observe===undefined||typeof v.observe==='string') && (v.scope===undefined||typeof v.scope==='string') && typeof v.guidance === 'string' && Array.isArray(v.goal_ids) && v.goal_ids.every(id => typeof id === 'string') && typeof v.pinned === 'boolean' && typeof v.needs_attention === 'boolean' && typeof v.attention_reason === 'string';
    }
    return m.kind === 'catalog' && ['course','teacher','domain'].includes(String(v.kind)) && typeof v.name === 'string' && typeof v.active === 'boolean';
  };
  const validWrite = (v: unknown): v is Write => record(v) && (v.method_change===undefined||['adjustment','correction'].includes(String(v.method_change))) && (v.method_create_mode===undefined||['full','quick'].includes(String(v.method_create_mode))) && typeof v.id === 'string' && typeof v.operation_id === 'string' &&
    (v.expected_version === null || Number.isSafeInteger(v.expected_version) && Number(v.expected_version) > 0) && typeof v.change_note === 'string' && validMutation(v.mutation);
  if (!record(parsed) || !Object.entries(parsed).every(([id, value]) => validWrite(value) && id === value.id &&
    (!(value as Draft).pending || validWrite((value as Draft).pending) && (value as Draft).pending!.id === id) &&
    (!(value as Draft).conflict_backup || validMutation((value as Draft).conflict_backup)))) {
    throw new Error('本机草稿格式无法读取，请保留浏览器数据后重试。');
  }
  return parsed as Record<string, Draft>;
}

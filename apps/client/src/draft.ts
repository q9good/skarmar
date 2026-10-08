import AsyncStorage from '@react-native-async-storage/async-storage';
import type { SaveRequest, SessionInput } from './api';

export const DRAFT_KEY = 'skarma:training-draft:v1';
export type Draft = {
  id: string; version: number | null; input: SessionInput;
  pending?: SaveRequest;
};

// Secure-origin browser UUIDs; the native adapter will be supplied in phase two.
export function uuid(): string {
  if (!globalThis.crypto?.randomUUID) throw new Error('请使用支持安全存储的现代浏览器打开');
  return globalThis.crypto.randomUUID();
}
export function localDate(): string {
  const parts = new Intl.DateTimeFormat('en-CA', {
    timeZone: 'Asia/Shanghai', year: 'numeric', month: '2-digit', day: '2-digit',
  }).formatToParts(new Date());
  const value = (name: string) => parts.find(p => p.type === name)?.value;
  return `${value('year')}-${value('month')}-${value('day')}`;
}
export function emptyInput(): SessionInput {
  return {
    business_date: localDate(), time_slot: '上午', activities: [], organization: '居家',
    plan: '', observation: '', targets: [], has_difficulty: false, difficulty: '',
    has_experience: false, experience: '', experience_kind: 'task',
  };
}
export const saveDraft = (draft: Draft) => AsyncStorage.setItem(DRAFT_KEY, JSON.stringify(draft));
export const clearDraft = () => AsyncStorage.removeItem(DRAFT_KEY);
export async function readDraft(): Promise<Draft | null> {
  const value = await AsyncStorage.getItem(DRAFT_KEY);
  if (!value) return null;
  const draft = JSON.parse(value) as Draft;
  if (typeof draft.id !== 'string' || !draft.input || !Array.isArray(draft.input.targets)) {
    throw new Error('本机草稿格式异常，请先导出备份后处理');
  }
  return draft;
}

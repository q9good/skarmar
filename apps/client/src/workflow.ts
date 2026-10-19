import type { AppState, Goal, Lesson, SessionInput, Target } from './api';

export function shiftDate(date: string, days: number): string {
  const value = new Date(`${date}T12:00:00Z`);
  value.setUTCDate(value.getUTCDate() + days);
  return value.toISOString().slice(0, 10);
}
export function previousMonth(today: string): { start: string; end: string } {
  const first = `${today.slice(0, 7)}-01`;
  const end = shiftDate(first, -1);
  return { start: `${end.slice(0, 7)}-01`, end };
}
export function validDate(value: string): boolean {
  const date = new Date(`${value}T12:00:00Z`);
  return /^\d{4}-\d{2}-\d{2}$/.test(value) && !Number.isNaN(date.getTime()) && date.toISOString().slice(0, 10) === value;
}
export function lessonCounts(lessons: Lesson[], start: string, end: string) {
  const counts = new Map<string, number>();
  const ids = new Set<string>();
  for (const lesson of lessons) {
    if (ids.has(lesson.id) || lesson.business_date < start || lesson.business_date > end) continue;
    ids.add(lesson.id);
    counts.set(lesson.teacher, (counts.get(lesson.teacher) ?? 0) + 1);
  }
  return [...counts].sort(([a], [b]) => a.localeCompare(b, 'zh-CN')).map(([teacher, count]) => ({ teacher, count }));
}

// Switching to a goal already in the plan swaps roles while keeping its own notes.
export function replaceTarget(targets: Target[], index: number, goalId: string): Target[] {
  const other = targets.findIndex(target => target.goal_id === goalId);
  if (other === index) return targets;
  if (other !== -1) return targets.map((target, i) => i === index ? { ...targets[other], role: target.role } :
    i === other ? { ...targets[index], role: target.role } : target);
  return targets.map((target, i) => i === index ? { goal_id: goalId, role: target.role, plan: '', progress: '', outcome: null, next_step: '' } : target);
}
export function orderedGoals(goals: Goal[], input: SessionInput): Goal[] {
  const priority = new Map(input.targets.map((target, i) => [target.goal_id, i]));
  return [...goals].sort((a, b) => (priority.get(a.id) ?? Infinity) - (priority.get(b.id) ?? Infinity));
}

// A late GET must not replace a newer saved entity, even across overlapping refreshes.
export function mergeState(current: AppState, incoming: AppState): AppState {
  const deleted = [...new Map([...(current.deleted ?? []), ...(incoming.deleted ?? [])].map(item => [`${item.kind}:${item.id}`, item])).values()];
  const removed = new Set(deleted.map(item => `${item.kind}:${item.id}`));
  const merge = <T extends { id: string; version: number }>(old: T[], next: T[], kind: string) => {
    const previous = new Map(old.map(item => [item.id, item]));
    const values = next.map(item => (previous.get(item.id)?.version ?? -1) > item.version ? previous.get(item.id)! : item);
    const ids = new Set(next.map(item => item.id));
    return [...values, ...old.filter(item => !ids.has(item.id))].filter(item => !removed.has(`${kind}:${item.id}`));
  };
  return { deleted, goals: merge(current.goals, incoming.goals, 'goals'), sessions: merge(current.sessions, incoming.sessions, 'sessions'),
    lessons: merge(current.lessons, incoming.lessons ?? [], 'lessons'), pathways: merge(current.pathways, incoming.pathways, 'pathways'),
    difficulties: merge(current.difficulties, incoming.difficulties, 'difficulties'), experiences: merge(current.experiences, incoming.experiences, 'experiences'),
    lesson_options: merge((current.lesson_options ?? []).map(item => ({ ...item, id: `${item.kind}:${item.value}` })),
      (incoming.lesson_options ?? []).map(item => ({ ...item, id: `${item.kind}:${item.value}` })), 'lesson_options').map(({ id: _id, ...item }) => item) };
}

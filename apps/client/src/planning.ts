import type { AppState, Goal } from './api';

export type PlanningPeriod = 7 | 30 | 90 | null;
export type PlanningLevel = 'medium' | 'short';
export type PlanningSource = 'family' | 'lessons' | 'all';
export type ProgressGroup = 'not_started' | 'in_progress' | 'done' | 'archived' | 'overdue' | 'unknown';
export const progressGroups: { id: ProgressGroup; title: string; color: string }[] = [
  { id: 'not_started', title: '未开始', color: '#C49661' },
  { id: 'in_progress', title: '进行中', color: '#538A76' },
  { id: 'done', title: '已达成／完成', color: '#537DAD' },
  { id: 'archived', title: '已归档 · 达成待核对', color: '#9A91AF' },
  { id: 'overdue', title: '已逾期（历史状态）', color: '#BE7C6B' },
  { id: 'unknown', title: '状态待核对', color: '#A7AFA6' },
];
export const hasReachedGoal = (goal: Goal) => goal.progress_status === 'achieved' || goal.progress_status === 'completed';
export const needsTraining = (goal: Goal) => goal.status === 'active' && !hasReachedGoal(goal);
export function progressGroup(goal: Goal): ProgressGroup {
  if (hasReachedGoal(goal)) return 'done';
  if (goal.status === 'archived') return 'archived';
  if (goal.progress_status === 'not_started' || goal.progress_status === 'in_progress' || goal.progress_status === 'overdue') return goal.progress_status;
  return 'unknown';
}

// Calendar arithmetic on the stored business date, independent of the device timezone.
export function periodStart(today: string, days: PlanningPeriod): string | null {
  if (days === null) return null;
  const date = new Date(`${today}T00:00:00Z`);
  date.setUTCDate(date.getUTCDate() - days + 1);
  return date.toISOString().slice(0, 10);
}
type Hit = { direct: boolean; observation: string };
function sessionHits(session: { targets: { goal_id: string; progress: string }[]; target_snapshots: { goal_id: string; goal_snapshot: Goal }[] }, goals: Map<string, Goal>) {
  const hits = new Map<string, Hit>();
  const snapshots = new Map(session.target_snapshots.map(target => [target.goal_id, target.goal_snapshot]));
  for (const target of session.targets) {
    const goal = snapshots.get(target.goal_id) ?? goals.get(target.goal_id);
    const existing = hits.get(target.goal_id);
    hits.set(target.goal_id, { direct: true, observation: target.progress || existing?.observation || '' });
    // Use the parent recorded for that training, so later reparenting cannot move history.
    if (goal?.level === 'short' && goal.parent_id && goals.get(goal.parent_id)?.level === 'medium') {
      const parent = hits.get(goal.parent_id);
      hits.set(goal.parent_id, { direct: parent?.direct ?? false,
        observation: parent?.observation || (target.progress ? `${goal.title}：${target.progress}` : '') });
    }
  }
  return hits;
}
export type PlanningRow = {
  goal: Goal; count: number; familyCount: number; lessonCount: number; directCount: number; childOnlyCount: number;
  lastDate: string | null; observation: string; observationDate: string | null;
  totalChildren: number; completedChildren: number; pathwayCount: number;
};

export function buildPlanning(data: AppState, options: {
  today: string; days: PlanningPeriod; level: PlanningLevel; includeArchived: boolean; source?: PlanningSource;
}) {
  const start = periodStart(options.today, options.days);
  const source = options.source ?? 'family';
  const goals = new Map(data.goals.map(goal => [goal.id, goal]));
  const rows: PlanningRow[] = data.goals
    .filter(goal => goal.level === options.level && (options.includeArchived || goal.status === 'active'))
    .map(goal => {
      const children = data.goals.filter(child => child.level === 'short' && child.parent_id === goal.id);
      return { goal, count: 0, familyCount: 0, lessonCount: 0, directCount: 0, childOnlyCount: 0, lastDate: null, observation: '', observationDate: null,
        totalChildren: children.length, completedChildren: children.filter(hasReachedGoal).length,
        pathwayCount: data.pathways.filter(path => path.status === 'active' && path.goal_ids.includes(goal.id)).length };
    });
  const byId = new Map(rows.map(row => [row.goal.id, row]));
  let familyCompletedCount = 0;
  let lessonCompletedCount = 0;
  const apply = (record: { business_date: string; targets: { goal_id: string; progress: string }[]; target_snapshots: { goal_id: string; goal_snapshot: Goal }[] }, kind: 'family' | 'lessons') => {
    if (record.business_date > options.today) return;
    const inPeriod = start === null || record.business_date >= start;
    if (inPeriod) { if(kind === 'family') familyCompletedCount++; else lessonCompletedCount++; }
    const included = source === 'all' || source === kind;
    for (const [id, hit] of sessionHits(record, goals)) {
      const row = byId.get(id); if (!row) continue;
      if (inPeriod) { if(kind === 'family') row.familyCount++; else row.lessonCount++; }
      if (!included) continue;
      if (row.lastDate === null || record.business_date > row.lastDate) row.lastDate = record.business_date;
      if (hit.observation && (row.observationDate === null || record.business_date > row.observationDate)) {
        row.observationDate = record.business_date; row.observation = hit.observation;
      }
      if (inPeriod) { row.count++; if(hit.direct) row.directCount++; else row.childOnlyCount++; }
    }
  };
  for(const session of new Map(data.sessions.map(value=>[value.id,value])).values()) {
    if(session.status === 'completed') apply(session,'family');
  }
  const lessons = [...new Map((data.lessons ?? []).map(value=>[value.id,value])).values()];
  for(const lesson of lessons) apply({ ...lesson, targets: lesson.links?.targets ?? [], target_snapshots: lesson.target_snapshots ?? [] },'lessons');
  const completedCount = source === 'family' ? familyCompletedCount : source === 'lessons' ? lessonCompletedCount : familyCompletedCount + lessonCompletedCount;
  rows.sort((a, b) => a.count - b.count || (a.lastDate ?? '').localeCompare(b.lastDate ?? '') || a.goal.title.localeCompare(b.goal.title, 'zh-CN'));
  const uncovered = rows.filter(row => row.count === 0 && needsTraining(row.goal));
  const coverage = rows.reduce((sum, row) => sum + row.count, 0);
  return { start, source, rows, coverage, completedCount, familyCompletedCount, lessonCompletedCount, uncovered,
    coveredGoals: rows.filter(row => row.count > 0).length,
    progress: progressGroups.map(group => ({ ...group, value: rows.filter(row => progressGroup(row.goal) === group.id).length })),
    pathways: data.pathways.filter(path => options.includeArchived || path.status === 'active').map(pathway => ({
      pathway,
      goals: pathway.goal_ids.flatMap(id => byId.has(id) ? [byId.get(id)!.goal] : []),
      experienceCount: data.experiences.filter(experience => experience.pathway_ids.includes(pathway.id)).length,
      lessonUseCount: lessons.filter(lesson => lesson.business_date <= options.today && (start === null || lesson.business_date >= start) && lesson.links?.pathway_ids.includes(pathway.id)).length,
    })),
    goalsWithPathways: rows.filter(row => row.pathwayCount > 0).length,
  };
}

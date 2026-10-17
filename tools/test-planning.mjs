import assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildPlanning, periodStart, progressGroup } from '../apps/client/src/planning.ts';

const goal = (id, level = 'short', patch = {}) => ({ id, title: id, level, parent_id: null, status: 'active', progress_status: 'in_progress', ...patch });
const medium = goal('medium', 'medium');
const short = goal('short', 'short', { parent_id: medium.id });
const other = goal('other', 'short', { parent_id: medium.id });
const session = (id, date, goals, patch = {}) => ({ id, business_date: date, status: 'completed',
  targets: goals.map(goal => ({ goal_id: goal.id, role: 'secondary', progress: `观察 ${goal.title}` })),
  target_snapshots: goals.map(goal => ({ goal_id: goal.id, goal_snapshot: { ...goal } })), ...patch });
const data = (sessions = [], goals = [medium, short, other], pathways = [], experiences = []) => ({ goals, sessions, pathways, experiences, difficulties: [] });
const options = { today: '2026-10-18', days: 30, level: 'short', includeArchived: false };
const row = (result, id) => result.rows.find(row => row.goal.id === id);

test('business-day windows include today and handle leap days and year boundaries', () => {
  assert.equal(periodStart('2026-10-18', 30), '2026-09-19');
  assert.equal(periodStart('2024-03-01', 7), '2024-02-24');
  assert.equal(periodStart('2026-01-02', 7), '2025-12-27');
  assert.equal(periodStart('2026-10-18', null), null);
});
test('only completed, nonfuture sessions count, including the lower bound; IDs are not counted twice', () => {
  const recent = session('recent', '2026-10-18', [short]);
  const result = buildPlanning(data([recent, recent,
    session('boundary', '2026-09-19', [short]), session('old', '2026-09-18', [short]),
    session('future', '2026-10-19', [short]), session('draft', '2026-10-18', [other], { status: 'draft' }),
  ]), options);
  assert.equal(result.completedCount, 2); assert.equal(row(result, short.id).count, 2);
  assert.equal(row(result, other.id).count, 0); assert.equal(result.coverage, 2);
});
test('multiple goals each get one coverage, while the session total stays one', () => {
  const result = buildPlanning(data([session('one', '2026-10-18', [short, other])]), options);
  assert.equal(result.completedCount, 1); assert.equal(result.coverage, 2); assert.equal(result.coveredGoals, 2);
});
test('medium rollup deduplicates direct and child training and records child-only sessions', () => {
  const result = buildPlanning(data([
    session('both', '2026-10-18', [short, medium, other]), session('children', '2026-10-17', [other, short]),
  ]), { ...options, level: 'medium' });
  assert.equal(row(result, medium.id).count, 2);
  assert.equal(row(result, medium.id).directCount, 1);
  assert.equal(row(result, medium.id).childOnlyCount, 1);
});
test('later reparenting does not move historical training to the new medium goal', () => {
  const newParent = goal('new-parent', 'medium');
  const history = session('past', '2026-10-18', [short]);
  const result = buildPlanning(data([history], [medium, newParent, { ...short, parent_id: newParent.id }]), { ...options, level: 'medium' });
  assert.equal(row(result, medium.id).count, 1); assert.equal(row(result, newParent.id).count, 0);
  assert.equal(row(result, medium.id).totalChildren, 0); assert.equal(row(result, newParent.id).totalChildren, 1);
});
test('legacy sessions without snapshots fall back to the current known hierarchy', () => {
  const result = buildPlanning(data([session('legacy', '2026-10-18', [short], { target_snapshots: [] })]), { ...options, level: 'medium' });
  assert.equal(row(result, medium.id).count, 1);
});
test('zero-period coverage preserves the last older training and its observation', () => {
  const result = buildPlanning(data([session('older', '2026-09-18', [short])]), options);
  assert.equal(row(result, short.id).count, 0); assert.equal(row(result, short.id).lastDate, '2026-09-18');
  assert.equal(row(result, short.id).observationDate, '2026-09-18'); assert.equal(row(result, short.id).observation, '观察 short');
  assert.equal(result.rows[0].goal.id, other.id, 'no history comes before older history at equal counts');
});
test('all-time includes old training but excludes future completions and drafts', () => {
  const result = buildPlanning(data([session('old', '2020-01-01', [short]), session('future', '2026-10-19', [short])]), { ...options, days: null });
  assert.equal(row(result, short.id).count, 1); assert.equal(row(result, short.id).lastDate, '2020-01-01');
});
test('current progress stays factual: archive is not completion, unknown is not zero capability', () => {
  const archived = goal('archived', 'short', { status: 'archived', progress_status: null });
  const done = goal('done', 'short', { progress_status: 'achieved' });
  const unknown = goal('unknown', 'short', { progress_status: null });
  assert.equal(progressGroup(archived), 'archived'); assert.equal(progressGroup(unknown), 'unknown');
  const result = buildPlanning(data([], [medium, archived, done, unknown]), { ...options, includeArchived: true });
  assert.deepEqual(result.uncovered.map(row => row.goal.id), [unknown.id]);
  assert.equal(result.progress.find(group => group.id === 'done').value, 1);
  assert.equal(result.progress.reduce((sum, group) => sum + group.value, 0), 3);
  const parents = buildPlanning(data([], [medium, { ...done, parent_id: medium.id }, { ...archived, parent_id: medium.id }]), { ...options, level: 'medium' });
  assert.equal(row(parents, medium.id).completedChildren, 1); assert.equal(row(parents, medium.id).totalChildren, 2);
});
test('pathway coverage uses explicit active mappings once, without inferring training or descendant links', () => {
  const paths = [
    { id: 'a', status: 'active', goal_ids: [medium.id, short.id] },
    { id: 'b', status: 'active', goal_ids: [short.id] },
    { id: 'c', status: 'archived', goal_ids: [other.id] },
  ];
  const result = buildPlanning(data([], [medium, short, other], paths, [{ pathway_ids: ['a', 'b'] }]), options);
  assert.equal(result.goalsWithPathways, 1); assert.equal(row(result, short.id).pathwayCount, 2);
  assert.equal(row(result, other.id).pathwayCount, 0); assert.equal(result.coverage, 0);
  assert.equal(result.pathways.length, 2); assert.equal(result.pathways[0].experienceCount, 1);
  assert.deepEqual(result.pathways[0].goals.map(goal => goal.id), [short.id]);
  assert.equal(buildPlanning(data([], [medium, short, other], paths), { ...options, includeArchived: true }).pathways.length, 3);
});
test('empty space yields zero coverage and finite values', () => {
  const result = buildPlanning(data([], []), options);
  assert.deepEqual(result.rows, []); assert.equal(result.coverage, 0); assert.equal(result.goalsWithPathways, 0);
});

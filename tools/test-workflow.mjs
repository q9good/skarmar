import assert from 'node:assert/strict';
import { test } from 'node:test';
import { lessonCounts, previousMonth, shiftDate, validDate, replaceTarget, mergeState, orderedGoals } from '../apps/client/src/workflow.ts';

test('previous calendar month includes year and leap boundaries; review dates advance 14 days', () => {
  assert.deepEqual(previousMonth('2026-01-09'), { start: '2025-12-01', end: '2025-12-31' });
  assert.deepEqual(previousMonth('2024-03-01'), { start: '2024-02-01', end: '2024-02-29' });
  assert.equal(shiftDate('2026-12-25', 14), '2027-01-08');
  assert.equal(validDate('2026-02-30'), false);
  assert.equal(validDate('2024-02-29'), true);
});
test('teacher counts are inclusive and use actual lesson dates, excluding duplicates and other periods', () => {
  const lessons = [
    { id: '1', business_date: '2026-09-01', teacher: '甲' },
    { id: '2', business_date: '2026-09-30', teacher: '甲' },
    { id: '3', business_date: '2026-09-12', teacher: '乙' },
    { id: '3', business_date: '2026-09-12', teacher: '乙' },
    { id: '4', business_date: '2026-08-31', teacher: '甲' },
    { id: '5', business_date: '2026-10-01', teacher: '乙' },
  ];
  const result = new Map(lessonCounts(lessons, '2026-09-01', '2026-09-30').map(row => [row.teacher, row.count]));
  assert.equal(result.get('甲'), 2); assert.equal(result.get('乙'), 1);
});
test('switching primary and secondary keeps notes with their goal and preserves uniqueness', () => {
  const original = [{ goal_id: 'a', role: 'primary', progress: 'a observation' }, { goal_id: 'b', role: 'secondary', progress: 'b observation' }];
  const swapped = replaceTarget(original, 0, 'b');
  assert.deepEqual(swapped, [{ goal_id: 'b', role: 'primary', progress: 'b observation' }, { goal_id: 'a', role: 'secondary', progress: 'a observation' }]);
  assert.equal(original[0].goal_id, 'a');
  assert.equal(replaceTarget(original, 0, 'c')[0].progress, '', 'changing to an unused goal must not reuse another goal’s observation');
});
test('a stale GET cannot undo a just-saved record or reactivate a stopped dropdown option', () => {
  const base = { goals: [], sessions: [], lessons: [], difficulties: [], experiences: [], pathways: [], lesson_options: [] };
  const current = { ...base, goals: [{ id: 'a', version: 3 }], lesson_options: [{ kind: 'teacher', value: '甲', version: 2, active: false }] };
  const stale = { ...base, goals: [{ id: 'a', version: 2 }, { id: 'b', version: 1 }], lesson_options: [{ kind: 'teacher', value: '甲', version: 1, active: true }] };
  const result = mergeState(current, stale);
  assert.equal(result.goals[0].version, 3); assert.equal(result.goals[1].id, 'b');
  assert.equal(result.lesson_options[0].active, false);
  assert.equal(mergeState(current, base).goals[0].version, 3, 'a stale list must not erase a new goal');
});
test('plan goals appear before unrelated goals in follow-up choices', () => {
  const goals = [{ id: 'outside' }, { id: 'second' }, { id: 'main' }];
  const ordered = orderedGoals(goals, { targets: [{ goal_id: 'main' }, { goal_id: 'second' }] });
  assert.deepEqual(ordered.map(goal => goal.id), ['main', 'second', 'outside']);
});

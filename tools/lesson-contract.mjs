// Shared daily-class-record contract on native SQLite and Durable Object SQLite.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';

export async function lessonContract(label, api) {
  const before = await api('GET', '/api/state');
  const id = randomUUID();
  const path = `/api/lessons/${id}`;
  const lesson = {
    business_date: '2026-10-18', course: '音乐课', teacher: '合成老师',
    content: '', goals: '', problems: '', home_cooperation: '',
  };
  for (const field of ['course', 'teacher']) {
    await api('PUT', path, { expected_version: null, lesson: { ...lesson, [field]: ' ' } }, 422);
  }
  const created = await api('PUT', path, { expected_version: null, lesson });
  assert.equal(created.id, id);
  assert.equal(created.version, 1);
  assert.equal(created.teacher, lesson.teacher);
  const changed = { ...lesson, content: '跟唱\n保留中文与换行', home_cooperation: '家里共同练习' };
  const edited = await api('PUT', path, { expected_version: 1, lesson: changed });
  assert.equal(edited.version, 2);
  await api('PUT', path, { expected_version: 1, lesson }, 409);
  const state = await api('GET', '/api/state');
  assert.equal(state.lessons.length, before.lessons.length + 1);
  assert.deepEqual(state.lessons.find(row => row.id === id), edited);
  assert.equal(edited.content, changed.content);
  assert.equal(edited.home_cooperation, changed.home_cooperation);
  assert.deepEqual(state.sessions, before.sessions, 'daily class does not need or create a training plan');
  console.log(`PASS ${label}: independent class records, course/teacher required, optional fields, edit and stale-version conflict`);
}

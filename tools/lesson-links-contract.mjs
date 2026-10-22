// Synthetic, shared by native SQLite and the actual Rust Worker/DO adapter.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';

export async function lessonLinksContract(label, api, exec, admin) {
  const goal = await api('POST', '/api/goals', { id: randomUUID(), title: '课程关联合成目标', level: 'short', parent_id: null, area: '', criteria: '保存当时标准' });
  const method = await api('POST', '/api/pathways', { id: randomUUID(), title: '课程实际方法', description: '', steps: '先展示再等待', goal_ids: [goal.id] });
  const id = randomUUID(), path = `/api/lessons/${id}`;
  const input = { business_date: '2026-10-21', course: '关联课程', teacher: '关联老师', content: '课程原文', goals: '历史目标原文', problems: '需要分步提示', home_cooperation: '家庭继续观察',
    links: { targets: [{ goal_id: goal.id, role: 'primary', plan: '轮流活动', progress: '主动表达两次' }], pathway_ids: [method.id], track_problem: true, problem_goal_ids: [goal.id], problem_pathway_ids: [method.id] } };
  const request = { expected_version: null, difficulty_version: null, lesson: input };
  const state = () => api('GET', '/api/state');
  const difficulty = async () => (await state()).difficulties.find(item => item.source_lesson_id === id);
  for (const patch of [
    { targets: [input.links.targets[0], input.links.targets[0]] },
    { targets: [{ ...input.links.targets[0], role: 'secondary' }] },
    { pathway_ids: [randomUUID()] }, { problem_goal_ids: [randomUUID()] },
  ]) await api('PUT', path, { ...request, lesson: { ...input, links: { ...input.links, ...patch } } }, 422);
  assert.ok(!(await state()).lessons.some(row => row.id === id));
  if (exec) {
    const before = await state();
    await exec("CREATE TRIGGER fail_lesson_problem BEFORE INSERT ON difficulties WHEN json_extract(NEW.data,'$.source_lesson_id') IS NOT NULL BEGIN SELECT RAISE(ABORT,'synthetic lesson rollback'); END;");
    await api('PUT', path, request, 500);
    assert.deepEqual(await state(), before, 'late failure rolls back lesson, targets, options and difficulty');
    await exec('DROP TRIGGER fail_lesson_problem;');
  }
  let saved = await api('PUT', path, request);
  assert.equal(saved.target_snapshots[0].goal_snapshot.criteria, '保存当时标准');
  assert.deepEqual(await api('PUT', path, request), saved);
  let item = await difficulty();
  assert.equal(item.content, input.problems);
  assert.equal(item.source_session_id, null);
  assert.deepEqual(item.goal_ids, [goal.id]); assert.deepEqual(item.pathway_ids, [method.id]);
  assert.equal((await state()).difficulties.filter(row => row.source_lesson_id === id).length, 1);

  // Old clients editing only free text preserve all structured links and snapshots.
  const legacy = { ...input, content: '旧客户端修改正文' }; delete legacy.links;
  saved = await api('PUT', path, { expected_version: saved.version, lesson: legacy });
  assert.deepEqual(saved.links, input.links);
  await api('PUT', `/api/goals/${goal.id}`, { ...goal, title: '现在目标名', criteria: '现在标准', expected_version: goal.version, change_note: '合成复盘' });
  saved = await api('PUT', path, { expected_version: saved.version, lesson: { ...saved, content: '再编辑正文' } });
  assert.equal(saved.target_snapshots[0].goal_snapshot.criteria, '保存当时标准');
  await api('PUT', path, { expected_version: 1, lesson: input }, 409);

  // Independent followup edits cannot be overwritten by a stale lesson editor.
  const curated = await api('PUT', `/api/difficulties/${item.id}`, { expected_version: item.version, status: 'active', content: '家人独立维护的困难', conclusion: '独立观察', next_review_date: null, goal_ids: [], pathway_ids: [] });
  const changed = { ...saved, problems: '课程的新问题', links: { ...saved.links, problem_pathway_ids: [] } };
  const beforeConflict = await state();
  await api('PUT', path, { expected_version: saved.version, difficulty_version: item.version, lesson: changed }, 409);
  assert.deepEqual(await state(), beforeConflict, 'followup conflict rolls back the entire lesson edit');
  saved = await api('PUT', path, { expected_version: saved.version, difficulty_version: curated.version, lesson: changed });
  item = await difficulty();
  assert.equal(item.content, curated.content); assert.deepEqual(item.goal_ids, []); assert.deepEqual(item.pathway_ids, []);
  assert.equal(item.conclusion, '独立观察');
  const resolved = await api('PUT', `/api/difficulties/${item.id}`, { expected_version: item.version, status: 'resolved', conclusion: '已独立解决', next_review_date: null });
  saved = await api('PUT', path, { expected_version: saved.version, difficulty_version: resolved.version, lesson: { ...saved, problems: '原文再次修正' } });
  assert.deepEqual(await difficulty(), resolved);

  // Renaming creates a selectable new name without touching historical lessons.
  const old = (await state()).lesson_options.find(row => row.kind === 'teacher' && row.value === input.teacher);
  const rename = { ...old, expected_version: old.version, new_value: `老师新名${id.slice(0,8)}` };
  const renamed = await api('PUT', '/api/lesson-options', rename);
  assert.equal(renamed.value, rename.new_value);
  assert.deepEqual(await api('PUT', '/api/lesson-options', rename), renamed);
  assert.equal((await state()).lessons.find(row => row.id === id).teacher, input.teacher);
  const alias = (await state()).lesson_options.find(row => row.kind === old.kind && row.value === old.value);
  assert.equal(alias.active, false); assert.equal(alias.renamed_to, renamed.value);
  await api('PUT', '/api/lesson-options', { ...alias, expected_version: alias.version, active: true }, 409);
  await api('PUT', '/api/lesson-options', { ...renamed, expected_version: renamed.version, new_value: input.teacher }, 409);
  const stopped = await api('PUT', '/api/lesson-options', { ...renamed, expected_version: renamed.version, active: false });
  assert.equal((await api('PUT', '/api/lesson-options', { ...stopped, expected_version: stopped.version, active: true })).active, true);

  if (admin) {
    await api('DELETE', path, { expected_version: saved.version }, 403);
    const nowGoal = (await state()).goals.find(row => row.id === goal.id);
    await admin('DELETE', `/api/goals/${goal.id}`, { expected_version: nowGoal.version }, 409);
    await admin('DELETE', `/api/pathways/${method.id}`, { expected_version: method.version });
    saved = (await state()).lessons.find(row => row.id === id);
    assert.deepEqual(saved.links.pathway_ids, []);
    await admin('DELETE', `/api/difficulties/${resolved.id}`, { expected_version: resolved.version });
    saved = await api('PUT', path, { expected_version: saved.version, lesson: { ...saved, problems: '不能重建已删除跟进' } });
    assert.equal(await difficulty(), undefined);
    // A separate lesson proves deleting a source keeps the followup, marked as deleted.
    const secondId = randomUUID();
    const second = await api('PUT', `/api/lessons/${secondId}`, { expected_version: null, lesson: { ...input, links: { ...input.links, pathway_ids: [], problem_pathway_ids: [] } } });
    await admin('DELETE', `/api/lessons/${secondId}`, { expected_version: second.version });
    const retained = (await state()).difficulties.find(row => row.source_lesson_id === secondId);
    assert.ok(retained.source_deleted); assert.equal(retained.content, input.problems);
  }
  console.log(`PASS ${label}: lesson goal/method snapshots, followup links, replay, rollback, curated fields, catalog rename${admin ? ', member permissions and deletion' : ''}`);
}

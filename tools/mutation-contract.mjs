// Permission and atomicity contract: the same authenticated HTTP calls on SQLite and DO.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';

export async function mutationContract(label, admin, member, exec) {
  const state = () => admin('GET', '/api/state');
  const find = (snapshot, kind, id) => snapshot[kind].find(item => item.id === id);
  const goal = await member('POST', '/api/goals', { id: randomUUID(), title: 'mutation goal', level: 'short', parent_id: null, area: '', criteria: 'old criteria' });
  const method = await member('POST', '/api/pathways', { id: randomUUID(), title: 'mutation method', description: '', steps: '', goal_ids: [goal.id] });
  const independent = await member('POST', '/api/experiences', { id: randomUUID(), content: 'independent retained', goal_ids: [goal.id], pathway_ids: [method.id] });
  const input = { business_date: '2026-10-19', time_slot: '上午', activities: ['游戏'], organization: '居家', plan: 'before revision', observation: 'original observation',
    targets: [{ goal_id: goal.id, role: 'primary', plan: '', progress: '', outcome: 'archive', next_step: '' }], has_difficulty: true, difficulty: 'source difficulty',
    has_experience: true, experience: 'source experience', experience_kind: 'task', difficulty_pathway_ids: [method.id], experience_pathway_ids: [method.id] };
  const id = randomUUID();
  const originalRequest = { operation_id: randomUUID(), expected_version: null, action: 'complete', session: input };
  const initial = await member('PUT', `/api/sessions/${id}`, originalRequest);
  const revisionRequest = snapshot => {
    const current = find(snapshot, 'sessions', id);
    const { id: _id, version, status: _status, target_snapshots: _snapshots, ...session } = current;
    return { operation_id: randomUUID(), action: 'revise', expected_version: version, session, revision: { note: 'member correction',
      difficulty_version: snapshot.difficulties.find(item => item.source_session_id === id)?.version ?? null,
      experience_version: snapshot.experiences.find(item => item.source_session_id === id)?.version ?? null } };
  };
  const before = await state();
  const missingNote = revisionRequest(before); missingNote.revision.note = '';
  await member('PUT', `/api/sessions/${id}`, missingNote, 422);
  assert.deepEqual(await state(), before);
  const changed = revisionRequest(before); changed.session.observation = 'corrected observation'; changed.session.experience = 'corrected experience';
  const revised = await member('PUT', `/api/sessions/${id}`, changed);
  assert.equal(revised.session.status, 'completed'); assert.equal(revised.session.version, 2);
  assert.equal(revised.experience_id, initial.experience_id); assert.equal(revised.difficulty_id, initial.difficulty_id);
  assert.deepEqual(revised.session.target_snapshots[0].goal_snapshot, initial.session.target_snapshots[0].goal_snapshot);
  assert.deepEqual(await member('PUT', `/api/sessions/${id}`, changed), revised);
  const after = await state();
  assert.equal(find(after, 'experiences', initial.experience_id).content, 'corrected experience');
  assert.equal(find(after, 'experiences', initial.experience_id).version, 2);
  assert.deepEqual(find(after, 'goals', goal.id), find(before, 'goals', goal.id), 'revision does not repeat or undo goal archival');
  const history = await member('GET', `/api/sessions/${id}/history`);
  assert.equal(history.length, 2); assert.equal(history[1].session.observation, 'original observation');
  const stale = structuredClone(changed); stale.operation_id = randomUUID();
  await member('PUT', `/api/sessions/${id}`, stale, 409);
  assert.deepEqual(await state(), after);

  // A separate follow-up update must not be silently lost by a training revision.
  const conflict = revisionRequest(after); conflict.session.experience = 'must not overwrite concurrently';
  await member('PUT', `/api/experiences/${initial.experience_id}`, { expected_version: 2, content: 'independently maintained', status: 'verified', conclusion: 'retained evidence', next_review_date: null, goal_ids: [goal.id], pathway_ids: [] });
  const maintained = await state();
  await member('PUT', `/api/sessions/${id}`, conflict, 409);
  assert.deepEqual(await state(), maintained);
  assert.equal((await member('GET', `/api/sessions/${id}/history`)).length, 2);

  // Late failure after session/target writes rolls back both data and history.
  const retry = revisionRequest(maintained); retry.session.observation = 'atomic corrected observation';
  await exec("CREATE TRIGGER fail_session_revision BEFORE INSERT ON session_revisions BEGIN SELECT RAISE(ABORT,'synthetic revision failure'); END;");
  await member('PUT', `/api/sessions/${id}`, retry, 500);
  assert.deepEqual(await state(), maintained);
  await exec('DROP TRIGGER fail_session_revision;');
  await member('PUT', `/api/sessions/${id}`, retry);
  assert.equal(find(await state(), 'experiences', initial.experience_id).content, 'independently maintained', 'unrelated training edits keep follow-up edits');
  const bindingSafe = revisionRequest(await state()); bindingSafe.session.experience = 'training content corrected again';
  await member('PUT', `/api/sessions/${id}`, bindingSafe);
  const bindingResult = find(await state(), 'experiences', initial.experience_id);
  assert.equal(bindingResult.content, 'independently maintained', 'validated experience is never overwritten by source revision');
  assert.deepEqual(bindingResult.goal_ids, [goal.id]); assert.deepEqual(bindingResult.pathway_ids, []);
  assert.equal(bindingResult.status, 'verified'); assert.equal(bindingResult.conclusion, 'retained evidence');

  const lesson = await member('PUT', `/api/lessons/${randomUUID()}`, { expected_version: null, lesson: { business_date: '2026-10-19', course: 'synthetic course', teacher: 'synthetic teacher', content: '', goals: '', problems: '', home_cooperation: '' } });
  for (const [kind, record] of [['goals', goal], ['pathways', method], ['experiences', independent], ['difficulties', find(await state(), 'difficulties', initial.difficulty_id)], ['sessions', find(await state(), 'sessions', id)], ['lessons', lesson]]) {
    await member('DELETE', `/api/${kind}/${record.id}`, { expected_version: record.version }, 403);
  }
  const beforeDelete = await state();
  await admin('DELETE', `/api/sessions/${id}`, { expected_version: 1 }, 409);
  assert.deepEqual(await state(), beforeDelete);
  const currentExp = find(beforeDelete, 'experiences', initial.experience_id);
  // A late relation-delete failure must roll back tombstone, row and both link tables.
  await exec("CREATE TRIGGER fail_experience_delete BEFORE DELETE ON experiences BEGIN SELECT RAISE(ABORT,'synthetic delete failure'); END;");
  await admin('DELETE', `/api/experiences/${currentExp.id}`, { expected_version: currentExp.version }, 500);
  assert.deepEqual(await state(), beforeDelete);
  await exec('DROP TRIGGER fail_experience_delete;');
  await admin('DELETE', `/api/experiences/${currentExp.id}`, { expected_version: currentExp.version });
  await admin('DELETE', `/api/experiences/${currentExp.id}`, { expected_version: currentExp.version });
  await member('POST', '/api/experiences', { ...independent, id: currentExp.id }, 409);
  const noResurrection = revisionRequest(await state()); noResurrection.session.experience = 'summary change after admin deletion';
  await member('PUT', `/api/sessions/${id}`, noResurrection);
  assert.equal(find(await state(), 'experiences', currentExp.id), undefined);
  await admin('DELETE', `/api/goals/${goal.id}`, { expected_version: find(await state(), 'goals', goal.id).version }, 409);

  await admin('DELETE', `/api/pathways/${method.id}`, { expected_version: method.version });
  let snapshot = await state();
  assert.equal(find(snapshot, 'pathways', method.id), undefined);
  assert.deepEqual(find(snapshot, 'experiences', independent.id).pathway_ids, []);
  assert.deepEqual(find(snapshot, 'difficulties', initial.difficulty_id).pathway_ids, []);
  assert.deepEqual(find(snapshot, 'sessions', id).difficulty_pathway_ids ?? [], []);
  await admin('DELETE', `/api/sessions/${id}`, { expected_version: find(snapshot, 'sessions', id).version });
  snapshot = await state();
  const retained = find(snapshot, 'difficulties', initial.difficulty_id);
  assert.equal(retained.source_session_id, null); assert.equal(retained.source_deleted, true);
  assert.equal(retained.content, 'source difficulty');
  await member('PUT', `/api/sessions/${id}`, originalRequest, 409);
  await member('PUT', `/api/sessions/${id}`, { ...originalRequest, operation_id: randomUUID() }, 409);
  await admin('DELETE', `/api/goals/${goal.id}`, { expected_version: find(snapshot, 'goals', goal.id).version });
  snapshot = await state();
  assert.deepEqual(find(snapshot, 'experiences', independent.id).goal_ids, []);
  for (const [kind, record] of [['experiences', find(snapshot, 'experiences', independent.id)], ['difficulties', find(snapshot, 'difficulties', retained.id)], ['lessons', lesson]]) {
    await admin('DELETE', `/api/${kind}/${record.id}`, { expected_version: record.version });
    assert.equal(find(await state(), kind, record.id), undefined);
  }
  // Parent removal is explicit, never cascading child goals.
  const parent = await member('POST', '/api/goals', { ...goal, id: randomUUID(), level: 'medium', title: 'deletion parent' });
  const child = await member('POST', '/api/goals', { ...goal, id: randomUUID(), parent_id: parent.id, title: 'deletion child' });
  await admin('DELETE', `/api/goals/${parent.id}`, { expected_version: 1 }, 409);
  await admin('DELETE', `/api/goals/${child.id}`, { expected_version: 1 });
  await admin('DELETE', `/api/goals/${parent.id}`, { expected_version: 1 });
  assert.ok((await state()).deleted.some(item => item.kind === 'sessions' && item.id === id));
  console.log(`PASS ${label}: member revisions, admin-only deletion (6 types), version conflicts, link cleanup, no resurrection, history and late SQL rollback`);
}

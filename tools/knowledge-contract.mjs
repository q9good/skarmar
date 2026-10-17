import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';

// Shared HTTP contract, including late-SQL failure injection when the runtime exposes a test adapter.
export async function knowledgeContract(label, api, exec) {
  const goals = [];
  for (const title of ['knowledge purpose A', 'knowledge purpose B']) {
    goals.push(await api('POST', '/api/goals', { id: randomUUID(), title, level: 'long', parent_id: null, area: '', criteria: '' }));
  }
  const goalIds = goals.map(goal => goal.id);
  const paths = [];
  for (const title of ['knowledge pathway A', 'knowledge pathway B']) {
    const input = { id: randomUUID(), title, description: '用途', steps: '步骤一\n步骤二 😀', goal_ids: goalIds };
    const pathway = await api('POST', '/api/pathways', input);
    assert.deepEqual(await api('POST', '/api/pathways', input), pathway, 'uncertain-create retries reuse the same ID');
    paths.push(pathway);
  }
  const first = paths[0];
  const edit = { ...first, expected_version: first.version, description: '修改说明' };
  await api('PUT', `/api/pathways/${first.id}`, { ...edit, goal_ids: [randomUUID()] }, 422);
  await api('PUT', `/api/pathways/${first.id}`, { ...edit, goal_ids: [goalIds[0], goalIds[0]] }, 422);
  if (exec) {
    await exec("CREATE TRIGGER fail_pathway_relation BEFORE INSERT ON goal_pathways BEGIN SELECT RAISE(ABORT,'synthetic mapping failure'); END;");
    await api('PUT', `/api/pathways/${first.id}`, edit, 500);
    await exec('DROP TRIGGER fail_pathway_relation;');
    const current = await api('GET', '/api/state');
    assert.deepEqual(current.pathways.find(pathway => pathway.id === first.id), first);
  }
  const changed = await api('PUT', `/api/pathways/${first.id}`, edit);
  assert.equal(changed.version, 2);
  await api('PUT', `/api/pathways/${first.id}`, edit, 409);
  const items = [];
  for (const [goal_ids, pathway_ids] of [[goalIds, []], [[], paths.map(pathway => pathway.id)], [goalIds, paths.map(pathway => pathway.id)]]) {
    const input = { id: randomUUID(), content: 'knowledge content 中文\n=Literal text', goal_ids, pathway_ids };
    const item = await api('POST', '/api/experiences', input);
    assert.equal(item.source_session_id, null);
    assert.equal(item.status, 'pending');
    assert.deepEqual(item.goal_ids, goal_ids);
    assert.deepEqual(item.pathway_ids, pathway_ids);
    assert.deepEqual(await api('POST', '/api/experiences', input), item);
    items.push(item);
  }
  await api('POST', '/api/experiences', { id: randomUUID(), content: 'unbound', goal_ids: [], pathway_ids: [] }, 422);
  const both = items[2];
  const followUp = { expected_version: both.version, status: 'pending', next_review_date: null, conclusion: '保留验证线索' };
  if (exec) {
    await exec("CREATE TRIGGER fail_experience_relation BEFORE INSERT ON experience_pathways BEGIN SELECT RAISE(ABORT,'synthetic binding failure'); END;");
    await api('PUT', `/api/experiences/${both.id}`, { ...followUp, content: 'must roll back', goal_ids: [goalIds[1]] }, 500);
    await exec('DROP TRIGGER fail_experience_relation;');
    assert.deepEqual((await api('GET', '/api/state')).experiences.find(item => item.id === both.id), both);
  }
  const kept = await api('PUT', `/api/experiences/${both.id}`, followUp);
  assert.deepEqual(kept.goal_ids, both.goal_ids);
  assert.deepEqual(kept.pathway_ids, both.pathway_ids);
  await api('PUT', `/api/experiences/${both.id}`, followUp, 409);
  await api('PUT', `/api/experiences/${both.id}`, { ...followUp, expected_version: kept.version, pathway_ids: [randomUUID()] }, 422);
  await api('PUT', `/api/pathways/${changed.id}`, { ...changed, expected_version: changed.version, status: 'archived', goal_ids: [] });
  const state = await api('GET', '/api/state');
  assert.deepEqual(state.experiences.find(item => item.id === both.id), kept, 'reassigning a pathway never changes explicit experience bindings');
  assert.deepEqual(state.experiences.find(item => item.id === items[1].id).goal_ids, [], 'no transitive goal inference');
  console.log(`PASS ${label}: purpose/pathway many-to-many, independent principles/details, atomic bindings, archive retention and optimistic conflicts`);
}

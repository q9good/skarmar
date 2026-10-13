// Same exported OOXML content on native SQLite and the real Rust Durable Object.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';

export async function excelContract(label, send, cookie) {
  const headers = { cookie, 'x-skarma-request': '1', origin: 'https://contract.local' };
  async function api(method, path, body) {
    const response = await send(`/api${path}`, { method, headers: { ...headers, ...(body === undefined ? {} : { 'content-type': 'application/json' }) }, body: body === undefined ? undefined : JSON.stringify(body) });
    const text = await response.text();
    assert.equal(response.status, 200, `${label}: ${path}: ${text}`);
    return JSON.parse(text);
  }
  async function verify(fixture) {
    const state = await api('GET', '/state');
    const history = (await Promise.all(state.goals.map(goal => api('GET', `/goals/${goal.id}/history`)))).flat();
    const response = await send('/api/export.xlsx', { headers });
    assert.equal(response.status, 200, `${label}: export download`);
    assert.equal(response.headers.get('content-type'), 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet');
    assert.equal(response.headers.get('content-disposition'), 'attachment; filename="skarma-export.xlsx"');
    assert.equal(response.headers.get('cache-control'), 'no-store');
    const file = Buffer.from(await response.arrayBuffer());
    assert.equal(file.subarray(0, 4).toString('hex'), '504b0304');
    execFileSync('python3', ['tools/check_excel.py'], { input: JSON.stringify({ file: file.toString('base64'), state, history, fixture }), stdio: ['pipe', 'inherit', 'inherit'] });
    assert.deepEqual(await api('GET', '/state'), state, 'export must not mutate records');
  }
  await verify(); // Empty/new spaces keep all sheet headers, too.
  const long = await api('POST', '/goals', { id: randomUUID(), title: '导出长期', level: 'long', parent_id: null, area: '', criteria: '' });
  const medium = await api('POST', '/goals', { id: randomUUID(), title: '导出中期', level: 'medium', parent_id: long.id, area: '', criteria: '' });
  const original_title = '=HYPERLINK("https://example.invalid","中文😀")';
  const goal = await api('POST', '/goals', { id: randomUUID(), title: original_title, level: 'short', parent_id: medium.id, area: '沟通', criteria: '原始标准\n保留中文与换行 😀' });
  const secondary = await api('POST', '/goals', { id: randomUUID(), title: '导出副目标', level: 'short', parent_id: medium.id, area: '', criteria: '' });
  const input = {
    business_date: '2026-10-13', time_slot: '上午', activities: ['游戏', '生活场景'], organization: '居家',
    plan: '安排', observation: '实际观察\n保留换行 😀',
    targets: [goal, secondary].map((target, index) => ({ goal_id: target.id, role: index ? 'secondary' : 'primary', plan: '+原样保存', progress: '-观察记录', outcome: 'continue', next_step: '@后续安排' })),
    has_difficulty: true, difficulty: '困难明细', has_experience: true, experience: '经验明细', experience_kind: 'task',
  };
  const completed = randomUUID();
  const result = await api('PUT', `/sessions/${completed}`, { operation_id: randomUUID(), action: 'complete', expected_version: null, session: input });
  await api('PUT', `/difficulties/${result.difficulty_id}`, { expected_version: 1, status: 'resolved', next_review_date: '2026-10-20', conclusion: '采用分步提示后改善' });
  await api('PUT', `/experiences/${result.experience_id}`, { expected_version: 1, status: 'verified', next_review_date: null, conclusion: '在不同活动中观察到有效' });
  await api('PUT', `/goals/${goal.id}`, { ...goal, expected_version: goal.version, title: '复盘后的当前名称', criteria: '新标准', change_note: '依据本次观察调整' });
  const draft = randomUUID();
  await api('PUT', `/sessions/${draft}`, { operation_id: randomUUID(), action: 'save_draft', expected_version: null, session: { ...input, targets: [{ ...input.targets[0], progress: '', outcome: null }] } });
  await verify({ goal: goal.id, completed, draft, original_title });
  console.log(`PASS ${label}: authenticated Excel export preserves current data and historical goal definitions`);
}

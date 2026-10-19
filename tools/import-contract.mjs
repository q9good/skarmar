// Synthetic HTTP import contract shared by native SQLite and the Rust DO.
// `exec` is the local validation runner's private SQL hook, never a public API.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';

export async function importContract(label, send, owner, member, exec) {
  const origin = 'https://contract.local';
  async function json(method, path, body, cookie = owner, expected = 200) {
    const response = await send(path, {
      method,
      headers: { origin, 'x-skarma-request': '1', ...(cookie ? { cookie } : {}), ...(body === undefined ? {} : { 'content-type': 'application/json' }) },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await response.text();
    assert.equal(response.status, expected, `${label}: ${method} ${path}: ${text}`);
    assert.equal(response.headers.get('cache-control'), 'no-store');
    return JSON.parse(text);
  }
  async function upload(filename, content, cookie = owner, expected = 200) {
    const boundary = `synthetic-${randomUUID()}`;
    const body = Buffer.concat([
      Buffer.from(`--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="${filename}"\r\nContent-Type: application/octet-stream\r\n\r\n`),
      Buffer.isBuffer(content) ? content : Buffer.from(content),
      Buffer.from(`\r\n--${boundary}--\r\n`),
    ]);
    const response = await send('/api/import/preview', {
      method: 'POST', headers: { origin, 'x-skarma-request': '1', ...(cookie ? { cookie } : {}), 'content-type': `multipart/form-data; boundary=${boundary}` }, body,
    });
    const text = await response.text();
    assert.equal(response.status, expected, `${label}: import ${filename}: ${text}`);
    assert.equal(response.headers.get('cache-control'), 'no-store');
    return JSON.parse(text);
  }
  const commit = (preview, expected = 200, cookie = owner) => json('POST', '/api/import/commit', { preview_id: preview.preview_id }, cookie, expected);
  const state = () => json('GET', '/api/state');
  const baseline = await state();

  // Unauthorized users are rejected even when the workbook is not parseable.
  await upload('bad.xlsx', 'not an XLSX workbook', null, 401);
  await upload('bad.xlsx', 'not an XLSX workbook', member, 403);
  await commit({ preview_id: randomUUID() }, 401, null);
  await commit({ preview_id: randomUUID() }, 403, member);
  await commit({ preview_id: 'invalid-preview-id' }, 422);

  const unsupported = await upload('未知来源.csv', '未知字段\n合成原文\n');
  assert.equal(unsupported.can_commit, false, 'unknown schemas cannot silently become a successful import');
  await commit(unsupported, 422);
  assert.deepEqual(await state(), baseline);

  // Export contains only this runner's synthetic records. Its schema is not
  // the Tencent import schema, but a successful preview must still unzip and
  // read the real workbook before rejecting its unsupported sheets safely.
  const exported = await send('/api/export.xlsx', { headers: { origin, cookie: owner, 'x-skarma-request': '1' } });
  assert.equal(exported.status, 200, `${label}: synthetic XLSX export for import validation`);
  const workbook = Buffer.from(await exported.arrayBuffer());
  assert.equal(workbook.subarray(0, 4).toString('hex'), '504b0304');
  const xlsxPreview = await upload('合成导出.xlsx', workbook);
  assert.equal(xlsxPreview.can_commit, false, 'application export is not a supported Tencent import schema');
  assert.ok(xlsxPreview.sheets.length > 1, 'real XLSX decoding must inspect the exported worksheets');
  assert.deepEqual(await state(), baseline, 'XLSX preview cannot mutate business records');

  const sourceId = randomUUID();
  const csv = observation => `记录ID,日期,时间段,阶段性训练重点,训练记录,存在问题,下一步计划,记录人,训练形式\n${sourceId},2026-10-17,上午,合成训练计划,${observation},合成困难,合成后续,合成家人,合成形式\n`;
  const filename = '每日训练记录.csv';
  const incomplete = await upload(filename, csv(''));
  assert.equal(incomplete.totals.invalid, 1, 'recorder metadata cannot turn an empty observation into completed training');
  assert.equal(incomplete.can_commit, false);
  await commit(incomplete, 422);
  const preview = await upload(filename, csv('合成观察'));
  assert.equal(preview.can_commit, true);
  assert.equal(preview.totals.new, 1);
  assert.equal(preview.sheets[0].columns.find(column => column.source === '训练记录').target, '训练观察');
  assert.deepEqual(await state(), baseline, 'preview cannot mutate business records');
  await commit(preview, 403, member);
  assert.deepEqual(await state(), baseline);

  if (exec) {
    // Expire only this preview; session validity is tested by authContract.
    assert.match(preview.preview_id, /^[a-f0-9-]{36}$/);
    await exec(`UPDATE import_previews SET expires_at=0 WHERE id='${preview.preview_id}';`);
    await commit(preview, 410);
    assert.deepEqual(await state(), baseline, 'expired previews cannot import');
  }
  const ready = exec ? await upload(filename, csv('合成观察')) : preview;
  if (exec) {
    // Failure happens after the session and batch are inserted. Retrying the
    // same preview below also proves the failed batch ID did not survive.
    await exec("CREATE TRIGGER fail_import_source BEFORE INSERT ON import_source_rows BEGIN SELECT RAISE(ABORT,'synthetic import failure'); END;");
    try {
      await commit(ready, 500);
      assert.deepEqual(await state(), baseline, 'late SQL failure must roll back all imported business rows');
    } finally {
      await exec('DROP TRIGGER fail_import_source;');
    }
  }
  const committed = await commit(ready);
  const replays = await Promise.all([commit(ready), commit(ready)]);
  for (const replay of replays) assert.deepEqual(replay, committed, 'concurrent retry returns the original batch');
  const imported = await state();
  assert.equal(imported.sessions.length, baseline.sessions.length + 1);
  assert.deepEqual(imported.goals, baseline.goals);
  assert.deepEqual(imported.difficulties, baseline.difficulties, 'historical problem text must not create an unverified follow-up');
  const session = imported.sessions.find(row => !baseline.sessions.some(old => old.id === row.id));
  assert.equal(session.status, 'completed');
  assert.equal(session.observation, '合成观察\n\n原表记录人：合成家人\n\n原表训练形式：合成形式');
  assert.equal(session.difficulty, '合成困难');
  assert.deepEqual(session.target_snapshots, []);
  assert.ok(session.plan.includes('合成后续'));

  const duplicate = await upload(filename, csv('合成观察'));
  assert.equal(duplicate.totals.skipped, 1);
  assert.equal(duplicate.can_commit, false);
  await commit(duplicate, 422);
  const changed = await upload(filename, csv('变更后的合成观察'));
  assert.equal(changed.totals.invalid, 1);
  assert.equal(changed.can_commit, false);
  await commit(changed, 422);
  assert.deepEqual(await state(), imported, 'duplicate and changed source rows cannot overwrite imported records');

  const checks = exec ? ', preview expiry and late SQL rollback/retry' : '';
  console.log(`PASS ${label}: admin-only multipart import, mapping, preview isolation, unknown-schema rejection, concurrent idempotence and changed-source protection${checks}`);
}

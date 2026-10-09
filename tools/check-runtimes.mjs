// Run the same API contract on a fresh native SQLite DB and the real Rust Worker.
// Fault injection is confined to this local Miniflare wrapper, never the deployment bundle.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { mkdir, mkdtemp, rm } from 'node:fs/promises';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { resolve } from 'node:path';
import { Miniflare } from 'miniflare';

const root = resolve(import.meta.dirname, '..');
await mkdir(resolve(root, '.local'), { recursive: true });
const scratch = await mkdtemp(resolve(root, '.local/runtime-contract-'));
let child;
let mf;

async function startNative(database) {
  let nativeLog = '';
  child = spawn(resolve(root, 'target/debug/skarma-api'), [], {
    cwd: root,
    env: { ...process.env, SKARMA_STORAGE: 'sqlite', SKARMA_DATABASE: database, SKARMA_PORT: '0', SKARMA_DEMO: '1' },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  child.stderr.on('data', data => { nativeLog += data.toString(); });
  child.on('error', error => { nativeLog += error.message; });
  for (let attempt = 0; attempt < 100; attempt++) {
    const match = nativeLog.match(/listening on port (\d+)/);
    if (match) return `http://127.0.0.1:${match[1]}`;
    if (child.exitCode !== null) throw new Error(nativeLog);
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error(`Native startup failed: ${nativeLog}`);
}

async function contract(label, send, exec) {
  async function api(method, path, body, expected = 200) {
    const response = await send(path, {
      method, headers: { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await response.text();
    assert.equal(response.status, expected, `${label} ${method} ${path}: ${text}`);
    assert.match(response.headers.get('content-type'), /application\/json/);
    return JSON.parse(text);
  }
  assert.deepEqual(await api('GET', '/api/health'), { status: 'ok' });
  await api('GET', '/api/unknown', undefined, 404);
  const initial = await api('GET', '/api/state');
  assert.equal(initial.goals.length, 5);
  assert.equal(initial.sessions.length, 0);
  await assert.rejects(exec("INSERT INTO session_targets(session_id,goal_id,role) VALUES ('missing-session','missing-goal','primary');"));

  // Create a complete hierarchy using shared API routes, including SQL parent bindings.
  const long = await api('POST', '/api/goals', {
    id: randomUUID(), title: 'contract long', level: 'long', parent_id: null, area: '', criteria: '',
  });
  const medium = await api('POST', '/api/goals', {
    id: randomUUID(), title: 'contract medium', level: 'medium', parent_id: long.id, area: '', criteria: '',
  });
  const primary = await api('POST', '/api/goals', {
    id: randomUUID(), title: 'contract primary', level: 'short', parent_id: medium.id, area: '', criteria: '',
  });
  const secondary = await api('POST', '/api/goals', {
    id: randomUUID(), title: 'contract secondary', level: 'short', parent_id: medium.id, area: '', criteria: '',
  });
  const history = await api('GET', `/api/goals/${primary.id}/history`);
  assert.equal(history[0].goal.version, 1);
  assert.match(history[0].recorded_at, /^\d{4}-\d{2}-\d{2}T/);

  const session = {
    business_date: '2026-10-09', time_slot: '上午', activities: ['游戏'], organization: '居家',
    plan: 'contract plan', observation: 'contract observation',
    targets: [primary, secondary].map((goal, index) => ({
      goal_id: goal.id, role: index ? 'secondary' : 'primary', plan: '', progress: '',
      outcome: 'continue', next_step: '',
    })),
    has_difficulty: true, difficulty: 'contract difficulty',
    has_experience: true, experience: 'contract experience', experience_kind: 'task',
  };
  const id = randomUUID();
  const command = (action, expected_version, input = session) => ({
    operation_id: randomUUID(), action, expected_version, session: structuredClone(input),
  });
  const draft = await api('PUT', `/api/sessions/${id}`, command('save_draft', null));
  assert.equal(draft.session.version, 1);
  const completion = command('complete', 1);
  completion.session.targets[0].outcome = 'archive';

  // Failure occurs after session writes, target links, goal archive and revision inserts.
  await exec("CREATE TRIGGER fail_difficulty BEFORE INSERT ON difficulties BEGIN SELECT RAISE(ABORT,'contract failure'); END;");
  await api('PUT', `/api/sessions/${id}`, completion, 500);
  const rolledBack = await api('GET', '/api/state');
  assert.equal(rolledBack.sessions[0].version, 1);
  assert.equal(rolledBack.sessions[0].status, 'draft');
  assert.equal(rolledBack.goals.find(goal => goal.id === primary.id).status, 'active');
  assert.equal(rolledBack.difficulties.length, 0);
  assert.equal(rolledBack.experiences.length, 0);
  assert.equal((await api('GET', `/api/goals/${primary.id}/history`)).length, 1);
  await exec('DROP TRIGGER fail_difficulty;');

  const saved = await api('PUT', `/api/sessions/${id}`, completion);
  assert.equal(saved.session.id, id);
  assert.equal(saved.session.version, 2);
  assert.equal(saved.session.target_snapshots[0].goal_snapshot.status, 'active');
  assert.ok(saved.difficulty_id && saved.experience_id);
  const retries = await Promise.all([
    api('PUT', `/api/sessions/${id}`, completion),
    api('PUT', `/api/sessions/${id}`, completion),
  ]);
  for (const replay of retries) assert.deepEqual(replay, saved);
  const after = await api('GET', '/api/state');
  assert.equal(after.sessions.length, 1);
  assert.equal(after.difficulties.length, 1);
  assert.equal(after.experiences.length, 1);
  assert.equal(after.difficulties[0].source_session_id, id);
  assert.equal(after.goals.find(goal => goal.id === primary.id).status, 'archived');
  const revised = await api('GET', `/api/goals/${primary.id}/history`);
  assert.equal(revised.length, 2);
  assert.equal(revised[0].source_session_id, id);

  // Reusing an operation ID with changed content must conflict.
  const changed = structuredClone(completion);
  changed.session.plan = 'another body';
  await api('PUT', `/api/sessions/${id}`, changed, 409);
  await api('PUT', `/api/sessions/${id}`, command('save_draft', 2), 409);

  const followUp = { expected_version: 1, status: 'verified', next_review_date: null, conclusion: '' };
  await api('PUT', `/api/experiences/${saved.experience_id}`, followUp, 422);
  followUp.conclusion = 'contract evidence';
  assert.equal((await api('PUT', `/api/experiences/${saved.experience_id}`, followUp)).version, 2);
  await api('PUT', `/api/experiences/${saved.experience_id}`, followUp, 409);

  // Two writers use the same base version. Exactly one must win.
  const other = randomUUID();
  const concurrentDraft = command('save_draft', null);
  await api('PUT', `/api/sessions/${other}`, concurrentDraft);
  const writes = ['first', 'second'].map(plan => {
    const input = structuredClone(session); input.plan = plan;
    return send(`/api/sessions/${other}`, {
      method: 'PUT', headers: { 'content-type': 'application/json' },
      body: JSON.stringify(command('save_draft', 1, input)),
    });
  });
  const responses = await Promise.all(writes);
  assert.deepEqual(responses.map(response => response.status).sort(), [200, 409]);
  for (const response of responses) await response.text();
  const final = await api('GET', '/api/state');
  assert.equal(final.sessions.find(record => record.id === other).version, 2);
  console.log(`PASS ${label}: CRUD/history, late SQL rollback, atomic completion, concurrent replay, version conflict and follow-up evidence`);
}

try {
  const database = resolve(scratch, 'native.db');
  let origin = await startNative(database);
  await contract('native SQLite', (path, init) => fetch(origin + path, init), async sql => {
    execFileSync('python3', ['-c', 'import sqlite3,sys\nwith sqlite3.connect(sys.argv[1]) as db:\n db.execute("PRAGMA foreign_keys=ON")\n db.executescript(sys.argv[2])', database, sql]);
  });
  const nativeSnapshot = await (await fetch(origin + '/api/state')).json();
  child.kill('SIGTERM'); await once(child, 'exit');
  origin = await startNative(database);
  assert.deepEqual(await (await fetch(origin + '/api/state')).json(), nativeSnapshot);
  console.log('PASS native SQLite: restart preserves records and does not reseed');

  const options = {
    modules: true, modulesRoot: root, scriptPath: resolve(root, 'contract-wrapper.mjs'),
    modulesRules: [
      { type: 'ESModule', include: ['**/*.js', '**/*.mjs'] },
      { type: 'CompiledWasm', include: ['**/*.wasm'] },
    ],
    compatibilityDate: '2026-07-30',
    durableObjectsPersist: resolve(scratch, 'cf'),
    bindings: { SKARMA_STORAGE: 'durable_object', SKARMA_DEMO: '1', SKARMA_SPACE: 'contract' },
    durableObjects: { SKARMA_DB: { className: 'TestSpace', useSQLite: true } },
    script: `
      import backend, { SkarmaSpace } from './crates/worker/build/index.js';
      export default backend;
      export class TestSpace extends SkarmaSpace {
        constructor(ctx, env) { super(ctx, env); this.storage = ctx.storage; }
        async fetch(req) {
          if (new URL(req.url).pathname === '/test-sql') {
            const { sql } = await req.json();
            try {
              this.storage.sql.exec(sql);
              return new Response('ok');
            } catch {
              return new Response('SQL rejected', { status: 409 });
            }
          }
          return super.fetch(req);
        }
      }
    `,
  };
  mf = new Miniflare(options);
  const namespace = await mf.getDurableObjectNamespace('SKARMA_DB');
  const stub = namespace.get(namespace.idFromName('contract'));
  await contract('Rust Worker / DO SQLite', (path, init) => mf.dispatchFetch('https://contract.local' + path, init), async sql => {
    const response = await stub.fetch('https://contract.internal/test-sql', { method: 'POST', body: JSON.stringify({ sql }) });
    assert.equal(response.status, 200);
    await response.text();
  });
  const cfSnapshot = await (await mf.dispatchFetch('https://contract.local/api/state')).json();
  await mf.dispose();
  mf = new Miniflare(options);
  assert.deepEqual(await (await mf.dispatchFetch('https://contract.local/api/state')).json(), cfSnapshot);
  console.log('PASS Rust Worker / DO SQLite: restart preserves records and does not reseed');
} finally {
  if (mf) await mf.dispose();
  if (child?.pid && child.exitCode === null) { child.kill('SIGTERM'); await once(child, 'exit'); }
  await rm(scratch, { recursive: true, force: true });
}
